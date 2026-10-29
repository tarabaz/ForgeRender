//! `forgerender-cli`: rendering headless e benchmark riproducibili.
//!
//! ```text
//! forgerender-cli info
//! forgerender-cli render [--project F | --mesh F | --test-scene] [--out F.png]
//!                        [--spp N] [--width W] [--height H] [--threads T] [--bounces B]
//! forgerender-cli bench  [--spp N] [--width W] [--height H] [--threads 1,2,4,...] [--json F]
//! ```

use anyhow::{bail, Context};
use forge_import::ImportOptions;
use forge_render_core::{RenderBackend, RenderScene, RenderSession, RenderSettings, SessionState};
use forge_render_cpu::CpuBackend;
use forge_scene::{OrbitController, Scene};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

struct Args(Vec<String>);

impl Args {
    fn flag(&self, name: &str) -> bool {
        self.0.iter().any(|a| a == name)
    }
    fn value(&self, name: &str) -> Option<&str> {
        self.0
            .iter()
            .position(|a| a == name)
            .and_then(|i| self.0.get(i + 1))
            .map(|s| s.as_str())
    }
    fn parse<T: std::str::FromStr>(&self, name: &str) -> anyhow::Result<Option<T>> {
        match self.value(name) {
            None => Ok(None),
            Some(v) => v
                .parse()
                .map(Some)
                .map_err(|_| anyhow::anyhow!("valore non valido per {name}: {v}")),
        }
    }
}

fn main() -> anyhow::Result<()> {
    forge_telemetry::init_logging();
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = argv.first().cloned() else {
        print_help();
        return Ok(());
    };
    let args = Args(argv[1..].to_vec());
    match cmd.as_str() {
        "info" => {
            println!("{}", forge_telemetry::SystemInfo::detect().summary());
            Ok(())
        }
        "render" => render(&args),
        "bench" => bench(&args),
        "libreria" => library(&args),
        "-h" | "--help" | "help" => {
            print_help();
            Ok(())
        }
        other => bail!("comando sconosciuto: {other} (usa --help)"),
    }
}

fn print_help() {
    println!(
        "ForgeRender CLI {}\n\n\
         Comandi:\n  \
         info                       informazioni hardware rilevate\n  \
         render [opzioni]           rendering headless verso PNG\n    \
             --project F.frproject | --mesh F.obj/.stl/.step/.glb/.blend | --test-scene (default)\n    \
             --out F.png  --spp N  --width W  --height H  --threads T  --bounces B  --backend cpu|gpu\n    \
             --unit mm|cm|m|in  (per --mesh, default mm)  --quiet  --seed N\n    \
             --noise-threshold T (campionamento adattivo, es. 0.01; --spp resta il massimo)  --adaptive-min N\n    \
             --adaptive-map F.png (mappa dei campioni per pixel, PNG a 16 bit con i conteggi esatti)\n    \
             --hdri F.hdr|F.exr  --hdri-strength S  --hdri-rotation GRADI\n    \
             --denoise off|finale  --denoise-quality veloce|bilanciata|alta  --cuda  --denoise-strength 0..1\n  \
         bench [opzioni]            benchmark della scena di test con diversi numeri di thread\n    \
             --spp N (default 16)  --width W  --height H  --threads 1,2,4  --json F\n  \
         libreria elenco            materiali e HDRI della libreria\n  \
         libreria importa P [P…]    importa cartelle di mappe o zip come materiali  [--dimensione METRI]\n  \
         libreria scarica-tutto     scarica i cataloghi completi di ambientCG e Poly Haven nella libreria\n    \
             --fonte ambientcg|polyhaven|tutte (default tutte)  --tipo materiali|hdri|tutto (default materiali)\n    \
             --risoluzione 1K|2K|4K|8K (default 1K)  --filtro TESTO  --max N  --aggiorna-catalogo\n    \
             --solo-elenco (conta e stima lo spazio senza scaricare). Già scaricati: saltati; si può interrompere e riprendere",
        env!("CARGO_PKG_VERSION")
    );
}

fn load_scene(args: &Args) -> anyhow::Result<(Scene, RenderSettings)> {
    if let Some(p) = args.value("--project") {
        let l = forge_project::load(&PathBuf::from(p))?;
        for w in &l.report.warnings {
            tracing::warn!("{w}");
        }
        return Ok((l.scene, l.settings));
    }
    if let Some(p) = args.value("--mesh") {
        let mut scene = Scene::default();
        let mut opts = ImportOptions::default();
        if let Some(u) = args.value("--unit") {
            opts.unit = match u {
                "mm" => forge_scene::LengthUnit::Millimeter,
                "cm" => forge_scene::LengthUnit::Centimeter,
                "m" => forge_scene::LengthUnit::Meter,
                "in" => forge_scene::LengthUnit::Inch,
                _ => bail!("unità non valida: {u}"),
            };
        }
        let r = forge_import::import_file(&mut scene, &PathBuf::from(p), &opts)?;
        let b = scene.bounds_of(r.root);
        match r.camera {
            Some(cam) => scene.camera = cam,
            None => {
                let mut orbit = OrbitController::from_camera(&scene.camera);
                orbit.yaw = 0.6;
                orbit.pitch = 0.35;
                orbit.frame(&b);
                scene.camera = orbit.camera();
            }
        }
        scene.lights = if r.lights.is_empty() {
            forge_scene::light::preset("studio", b.center(), b.radius()).unwrap_or_default()
        } else {
            r.lights
        };
        return Ok((scene, RenderSettings::default()));
    }
    Ok((Scene::test_scene(), RenderSettings::default()))
}

fn render(args: &Args) -> anyhow::Result<()> {
    let (mut scene, mut settings) = load_scene(args)?;
    if let Some(p) = args.value("--hdri") {
        scene.environment = forge_scene::Environment::Hdri {
            path: PathBuf::from(p),
            strength: args.parse("--hdri-strength")?.unwrap_or(1.0),
            rotation_deg: args.parse("--hdri-rotation")?.unwrap_or(0.0),
        };
    }
    if let Some(v) = args.parse("--spp")? {
        settings.target_samples = v;
    }
    if let Some(v) = args.parse("--width")? {
        settings.width = v;
    }
    if let Some(v) = args.parse("--height")? {
        settings.height = v;
    }
    if let Some(v) = args.parse("--threads")? {
        settings.threads = v;
    }
    if let Some(v) = args.parse("--bounces")? {
        settings.max_bounces = v;
    }
    if let Some(v) = args.parse("--seed")? {
        settings.seed = v;
    }
    if let Some(v) = args.parse("--noise-threshold")? {
        settings.noise_threshold = v;
    }
    if let Some(v) = args.parse("--adaptive-min")? {
        settings.adaptive_min_samples = v;
    }
    if settings.target_samples == 0 {
        bail!("--spp deve essere > 0 per il rendering da riga di comando");
    }
    if let Some(m) = args.value("--denoise") {
        settings.denoise.mode = match m {
            "off" | "no" => forge_render_core::DenoiseMode::Off,
            "finale" | "final" => forge_render_core::DenoiseMode::Final,
            _ => bail!("--denoise: valori ammessi off|finale"),
        };
    }
    if let Some(q) = args.value("--denoise-quality") {
        settings.denoise.quality = parse_quality(q)?;
    }
    // Sulla CLI CUDA si attiva solo se richiesto esplicitamente (la creazione del dispositivo richiede tempo).
    settings.denoise.use_cuda = args.flag("--cuda");
    if let Some(v) = args.parse("--denoise-strength")? {
        settings.denoise.strength = v;
    }
    let out = PathBuf::from(args.value("--out").unwrap_or("render.png"));
    let t_compile = Instant::now();
    let rs = Arc::new(RenderScene::compile(&scene));
    let compile = t_compile.elapsed();
    if let Some(b) = args.value("--backend") {
        settings.backend = match b {
            "gpu" => forge_render_core::BackendKind::Gpu,
            "cpu" => forge_render_core::BackendKind::Cpu,
            _ => bail!("backend non valido: {b} (cpu|gpu)"),
        };
    }
    let backend: Box<dyn RenderBackend> = match settings.backend {
        forge_render_core::BackendKind::Gpu => Box::new(forge_render_gpu::GpuBackend::new()?),
        forge_render_core::BackendKind::Cpu => Box::new(CpuBackend::from_settings(&settings)?),
    };
    let session = RenderSession::new();
    if settings.denoise.mode != forge_render_core::DenoiseMode::Off {
        match forge_denoise::OidnDenoiser::load() {
            Ok(d) => session.set_denoiser(Some(Box::new(d))),
            Err(e) => eprintln!("riduzione del rumore non disponibile: {e:#}"),
        }
    }
    let job = session.start(rs.clone(), settings.clone(), backend);
    let mut last_print = Instant::now();
    let p = loop {
        let p = session.progress();
        if p.job_id == job && !matches!(p.state, SessionState::Rendering | SessionState::Idle) {
            break p;
        }
        if !args.flag("--quiet") && last_print.elapsed() > Duration::from_secs(1) {
            eprintln!("  {}/{} campioni, {:.1} s", p.samples, p.target, p.elapsed.as_secs_f64());
            last_print = Instant::now();
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    if let SessionState::Failed(e) = &p.state {
        bail!("rendering fallito: {e}");
    }
    let frame = session.latest_frame().context("nessun frame prodotto")?;
    forge_image_io::save_frame_png(&out, &frame, settings.tone_mapping, settings.exposure_ev, settings.denoise.strength)?;
    if !p.denoiser.is_empty() {
        println!("riduzione del rumore: {}", p.denoiser);
    }
    println!(
        "{}x{} | {} campioni | {} triangoli | compilazione scena {:.0} ms | rendering {:.2} s | {:.1} ms/passata | {:.2} Mcampioni/s | {} | -> {}",
        p.width,
        p.height,
        p.samples,
        rs.triangle_count(),
        compile.as_secs_f64() * 1e3,
        p.elapsed.as_secs_f64(),
        p.avg_pass().as_secs_f64() * 1e3,
        p.msamples_per_sec(),
        p.device,
        out.display()
    );
    if settings.noise_threshold > 0.0 && p.width > 0 && p.height > 0 {
        let px = p.width as f64 * p.height as f64;
        println!(
            "campionamento adattivo: soglia {} | campioni per pixel: media {:.1}, massimo {} | {} campioni in totale",
            settings.noise_threshold,
            p.pixel_samples as f64 / px,
            p.samples,
            p.pixel_samples
        );
        if let Some(d) = &frame.adaptive {
            if let Some(f) = d.b_fraction {
                println!(
                    "metà B (stima dell'errore): n_B/n media {:.3}, min {:.3}, max {:.3} su {} pixel",
                    f.mean, f.min, f.max, f.pixels
                );
            }
            if let Some(map) = args.value("--adaptive-map") {
                let px: Vec<u16> = d.counts.iter().map(|&c| c.min(u16::MAX as u32) as u16).collect();
                let img = image::ImageBuffer::<image::Luma<u16>, _>::from_raw(frame.width, frame.height, px)
                    .context("mappa dei campioni di dimensione errata")?;
                img.save(map).with_context(|| format!("salvataggio di {map}"))?;
                println!("mappa dei campioni per pixel -> {map}");
            }
        }
    } else if args.value("--adaptive-map").is_some() {
        eprintln!("--adaptive-map ignorato: serve --noise-threshold > 0");
    }
    Ok(())
}

fn bench(args: &Args) -> anyhow::Result<()> {
    let spp: u32 = args.parse("--spp")?.unwrap_or(16);
    let width: u32 = args.parse("--width")?.unwrap_or(640);
    let height: u32 = args.parse("--height")?.unwrap_or(360);
    let max_threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
    let threads: Vec<usize> = match args.value("--threads") {
        Some(list) => list
            .split(',')
            .map(|t| t.trim().parse())
            .collect::<Result<_, _>>()
            .context("--threads")?,
        None => {
            let mut v = vec![1];
            while *v.last().unwrap() * 2 < max_threads {
                v.push(v.last().unwrap() * 2);
            }
            v.push(max_threads);
            v.dedup();
            v
        }
    };
    let sys = forge_telemetry::SystemInfo::detect();
    println!("{}", sys.summary());
    let scene = Scene::test_scene();
    let t = Instant::now();
    let rs = RenderScene::compile(&scene);
    let compile_ms = t.elapsed().as_secs_f64() * 1e3;
    println!(
        "Scena di test: {} triangoli, compilazione+BVH {:.1} ms",
        rs.triangle_count(),
        compile_ms
    );
    println!("Risoluzione {width}x{height}, {spp} campioni/pixel\n");
    println!(
        "{:>8} {:>12} {:>14} {:>14} {:>10}",
        "thread", "tempo (s)", "ms/passata", "Mcampioni/s", "speedup"
    );
    let settings = RenderSettings {
        width,
        height,
        target_samples: spp,
        ..Default::default()
    };
    let mut results = Vec::new();
    let mut base = None;
    let cancel = std::sync::atomic::AtomicBool::new(false);
    for &t in &threads {
        let mut be = CpuBackend::new(t)?;
        let mut pass = vec![[0.0f32; 3]; (width * height) as usize];
        // Passata di riscaldamento (cache, pagine di memoria, avvio thread).
        be.render_pass(&rs, &settings, 0, 1, &mut pass, &cancel)?;
        let t0 = Instant::now();
        for s in 0..spp {
            be.render_pass(&rs, &settings, s, 1, &mut pass, &cancel)?;
        }
        let secs = t0.elapsed().as_secs_f64();
        let ms = secs * 1e3 / spp as f64;
        let msps = (width as f64 * height as f64 * spp as f64) / secs / 1e6;
        let speedup = base.map(|b: f64| b / secs).unwrap_or(1.0);
        if base.is_none() {
            base = Some(secs);
        }
        println!("{t:>8} {secs:>12.3} {ms:>14.2} {msps:>14.3} {speedup:>9.2}x");
        results.push(serde_json::json!({"threads": t, "seconds": secs, "ms_per_pass": ms, "msamples_per_sec": msps, "speedup": speedup}));
    }
    // GPU: passate adattive con più campioni per pixel.
    match forge_render_gpu::GpuBackend::new() {
        Ok(mut g) => {
            let name = g.info().name.clone();
            let mut pass = vec![[0.0f32; 3]; (width * height) as usize];
            g.render_pass(&rs, &settings, 0, 1, &mut pass, &cancel)?; // riscaldamento (caricamento scena)
            let t0 = Instant::now();
            let mut done = 0;
            while done < spp {
                done += g.render_pass(&rs, &settings, done, spp - done, &mut pass, &cancel)?;
            }
            let secs = t0.elapsed().as_secs_f64();
            let msps = (width as f64 * height as f64 * spp as f64) / secs / 1e6;
            let speedup = base.map(|b: f64| b / secs).unwrap_or(1.0);
            println!(
                "{:>8} {secs:>12.3} {:>14.2} {msps:>14.3} {speedup:>9.2}x   ({name})",
                "GPU",
                secs * 1e3 / spp as f64
            );
            results.push(serde_json::json!({"device": "gpu", "name": name, "seconds": secs, "msamples_per_sec": msps, "speedup": speedup}));
        }
        Err(e) => println!("GPU: non disponibile ({e})"),
    }
    // Riduzione del rumore: tempo del filtro alla risoluzione del benchmark.
    match forge_denoise::OidnDenoiser::load() {
        Ok(mut d) => {
            let img = vec![[0.5f32; 3]; (width * height) as usize];
            let guides = forge_render_cpu::render_guides(&rs, &settings, forge_render_cpu::GUIDE_SAMPLES, &cancel);
            for cuda in [false, true] {
                if cuda && !d.cuda_available() {
                    println!("Denoiser CUDA: non disponibile ({})", d.cuda_error().unwrap_or("?"));
                    continue;
                }
                let q = forge_render_core::DenoiseQuality::High;
                d.run(&img, guides.as_ref(), width, height, q, cuda)?; // riscaldamento
                let t0 = Instant::now();
                d.run(&img, guides.as_ref(), width, height, q, cuda)?;
                let ms = t0.elapsed().as_secs_f64() * 1e3;
                println!(
                    "Denoiser {}: {ms:.1} ms per immagine ({width}x{height}, qualità alta)",
                    forge_denoise::Denoiser::name(&d)
                );
                results.push(serde_json::json!({"denoiser": forge_denoise::Denoiser::name(&d), "ms": ms}));
            }
        }
        Err(e) => println!("Denoiser: non disponibile ({e})"),
    }
    if let Some(p) = args.value("--json") {
        let doc = serde_json::json!({
            "system": sys.summary(),
            "scene": "test_scene",
            "triangles": rs.triangle_count(),
            "bvh_build_ms": compile_ms,
            "width": width, "height": height, "spp": spp,
            "results": results,
        });
        std::fs::write(p, serde_json::to_string_pretty(&doc)?)?;
        println!("\nRisultati salvati in {p}");
    }
    Ok(())
}

fn parse_quality(q: &str) -> anyhow::Result<forge_render_core::DenoiseQuality> {
    Ok(match q {
        "veloce" | "fast" => forge_render_core::DenoiseQuality::Fast,
        "bilanciata" | "balanced" => forge_render_core::DenoiseQuality::Balanced,
        "alta" | "high" => forge_render_core::DenoiseQuality::High,
        _ => bail!("qualità del denoiser non valida: {q} (veloce|bilanciata|alta)"),
    })
}

/// Scarica tutto il catalogo dei siti nella libreria, saltando ciò che c'è già.
fn download_all(args: &Args) -> anyhow::Result<()> {
    use forge_library::online::{self, AssetKind, Provider};
    let providers = match args.value("--fonte").unwrap_or("tutte") {
        "ambientcg" => vec![Provider::AmbientCg],
        "polyhaven" => vec![Provider::PolyHaven],
        "tutte" => vec![Provider::AmbientCg, Provider::PolyHaven],
        f => bail!("fonte non valida: {f} (ambientcg|polyhaven|tutte)"),
    };
    let kinds = match args.value("--tipo").unwrap_or("materiali") {
        "materiali" => vec![AssetKind::Material],
        "hdri" => vec![AssetKind::Hdri],
        "tutto" => vec![AssetKind::Material, AssetKind::Hdri],
        t => bail!("tipo non valido: {t} (materiali|hdri|tutto)"),
    };
    let res = args.value("--risoluzione").unwrap_or("1K").to_uppercase();
    if !online::RESOLUTIONS.contains(&res.as_str()) {
        bail!("risoluzione non valida: {res} (1K|2K|4K|8K)");
    }
    let filter = args.value("--filtro").unwrap_or("");
    let max: Option<usize> = args.parse("--max")?;
    let dry = args.flag("--solo-elenco");
    // Stima indicativa dello spazio per elemento (MB), per risoluzione.
    let ri = online::RESOLUTIONS.iter().position(|r| *r == res).unwrap_or(0);
    let mb = |k: AssetKind| match k {
        AssetKind::Material => [8.0, 25.0, 90.0, 320.0][ri],
        AssetKind::Hdri => [1.5, 6.0, 24.0, 100.0][ri],
    };

    let mut local = online::local_source_keys();
    let hdris = forge_library::paths::list_hdri();
    let (mut done, mut skipped, mut failed) = (0usize, 0usize, 0usize);
    for &k in &kinds {
        for &p in &providers {
            let label = format!("{} · {}", p.label(), if k == AssetKind::Material { "materiali" } else { "HDRI" });
            println!("\n{label}: scarico il catalogo…");
            let all = online::filter(&online::catalog(p, k, args.flag("--aggiorna-catalogo"))?, filter);
            let todo: Vec<_> = all
                .iter()
                .filter(|a| match k {
                    AssetKind::Material => !online::already_downloaded(a, &local),
                    AssetKind::Hdri => !online::hdri_downloaded(a, &hdris),
                })
                .take(max.unwrap_or(usize::MAX))
                .cloned()
                .collect();
            skipped += all.len().min(max.unwrap_or(usize::MAX)) - todo.len();
            println!(
                "{label}: {} nel catalogo, {} già in libreria, {} da scaricare a {res} (circa {:.1} GB, stima indicativa)",
                all.len(),
                all.len() - todo.len(),
                todo.len(),
                todo.len() as f64 * mb(k) / 1024.0
            );
            if dry {
                continue;
            }
            for (i, a) in todo.iter().enumerate() {
                print!("[{}/{}] {} ({})… ", i + 1, todo.len(), a.name, a.id);
                use std::io::Write;
                let _ = std::io::stdout().flush();
                let r = match k {
                    AssetKind::Material => online::download_material(a, &res, &|_| {}).map(|m| {
                        local.insert(online::source_key(p, &a.id));
                        m.dir
                    }),
                    AssetKind::Hdri => online::download_hdri(a, &res, &|_| {}),
                };
                match r {
                    Ok(_) => {
                        done += 1;
                        println!("ok");
                    }
                    Err(e) => {
                        failed += 1;
                        println!("ERRORE: {e:#}");
                    }
                }
            }
        }
    }
    if !dry {
        println!("\nFatto: {done} scaricati, {skipped} già presenti, {failed} non riusciti.");
        if failed > 0 {
            println!("Rilancia lo stesso comando per riprovare quelli non riusciti.");
        }
    }
    Ok(())
}

fn library(args: &Args) -> anyhow::Result<()> {
    let root = forge_library::paths::ensure()?;
    match args.0.first().map(String::as_str) {
        Some("elenco") | None => {
            println!("Libreria: {}", root.display());
            let mats = forge_library::store::list();
            println!("\nMateriali ({}):", mats.len());
            for m in &mats {
                let maps = m.material.texture_slots().iter().filter(|s| s.is_some()).count();
                println!("  {:<40} {maps} mappe  [{}]", m.material.name, m.source);
            }
            let hdri = forge_library::paths::list_hdri();
            println!("\nHDRI ({}):", hdri.len());
            for h in &hdri {
                println!("  {}", h.display());
            }
            Ok(())
        }
        Some("importa") => {
            let size: Option<f32> = args.parse("--dimensione")?;
            let paths: Vec<&String> = args.0[1..].iter().take_while(|a| !a.starts_with("--")).collect();
            if paths.is_empty() {
                bail!("indica almeno una cartella o uno zip da importare");
            }
            for p in paths {
                let p = PathBuf::from(p);
                let r = if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip")) {
                    forge_library::store::import_zip(&p, None, size, "importato da riga di comando")
                } else {
                    forge_library::store::import_folder(&p, None, size, "importato da riga di comando")
                };
                match r {
                    Ok(m) => println!("importato «{}» in {}", m.material.name, m.dir.display()),
                    Err(e) => eprintln!("{}: {e:#}", p.display()),
                }
            }
            Ok(())
        }
        Some("scarica-tutto") => download_all(args),
        Some(other) => bail!("comando libreria sconosciuto: {other} (elenco | importa | scarica-tutto)"),
    }
}
