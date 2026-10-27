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
             --out F.png  --spp N  --width W  --height H  --threads T  --bounces B\n    \
             --unit mm|cm|m|in  (per --mesh, default mm)  --quiet\n    \
             --hdri F.hdr|F.exr  --hdri-strength S  --hdri-rotation GRADI\n  \
         bench [opzioni]            benchmark della scena di test con diversi numeri di thread\n    \
             --spp N (default 16)  --width W  --height H  --threads 1,2,4  --json F",
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
    if settings.target_samples == 0 {
        bail!("--spp deve essere > 0 per il rendering da riga di comando");
    }
    let out = PathBuf::from(args.value("--out").unwrap_or("render.png"));
    let t_compile = Instant::now();
    let rs = Arc::new(RenderScene::compile(&scene));
    let compile = t_compile.elapsed();
    let backend = CpuBackend::from_settings(&settings)?;
    let session = RenderSession::new();
    let job = session.start(rs.clone(), settings.clone(), Box::new(backend));
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
    forge_image_io::save_frame_png(&out, &frame, settings.tone_mapping, settings.exposure_ev)?;
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
        be.render_pass(&rs, &settings, 0, &mut pass, &cancel)?;
        let t0 = Instant::now();
        for s in 0..spp {
            be.render_pass(&rs, &settings, s, &mut pass, &cancel)?;
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
