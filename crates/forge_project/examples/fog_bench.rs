//! Misura i tempi della nebbia sulla scena del cocktail (con le sue collisioni vere): ostacoli,
//! simulazione (fase per fase, con `FORGE_FOG_PROFILE=1`) e dettaglio, a più qualità, con più
//! ripetizioni. Salva il campo simulato per confrontare la qualità prima e dopo un'ottimizzazione.
//!
//! Uso: cargo run --release -p forge_project --example fog_bench -- [CARTELLA_USCITA] [RIPETIZIONI]
//! Con `FOG_BENCH_REF=cartella` confronta il campo con quello salvato lì (RMSE, massa).
use forge_render_core::RenderScene;
use forge_scene::FOG_QUALITY;
use std::path::PathBuf;
use std::time::Instant;

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn main() -> anyhow::Result<()> {
    let out = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "fog_bench".into()));
    let runs: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(3);
    std::fs::create_dir_all(&out)?;
    let project =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples/scene/cocktail-ghiaccio-secco/cocktail-ghiaccio-secco.frproject");
    let loaded = forge_project::load(&project)?;
    let rs = RenderScene::compile(&loaded.scene);
    let geo = rs.geometry.clone();
    let base = loaded.scene.fogs.first().expect("nebbia nel progetto").clone();
    let reference = std::env::var_os("FOG_BENCH_REF").map(PathBuf::from);
    println!("qualità | griglia sim | passi | ostacoli ms | simulazione ms (mediana, tutte) | dettaglio ms | massa | RMSE");
    let only = std::env::var("FOG_BENCH_ONLY").ok();
    for (name, sim_cells, resolution) in FOG_QUALITY.iter().skip(1) {
        if only.as_ref().is_some_and(|o| !o.split(',').any(|q| q == name.replace(' ', "_"))) {
            continue;
        }
        let mut fog = base.clone();
        fog.sim_cells = *sim_cells;
        fog.resolution = *resolution;
        let ground = forge_render_core::fog::ground_under(&fog, &geo);
        let solid = |spec: &forge_volume::GridSpec| forge_render_core::fog::voxelize(&geo, spec);
        let (mut t_obs, mut t_sim, mut t_det) = (Vec::new(), Vec::new(), Vec::new());
        let mut last = None;
        for _ in 0..runs {
            let spec = forge_volume::grid_specs(&fog, ground).0;
            let t0 = Instant::now();
            let _ = solid(&spec);
            t_obs.push(t0.elapsed().as_secs_f64() * 1e3);
            let t1 = Instant::now();
            let s = forge_volume::simulate_fog(&fog, Some(&solid), ground);
            t_sim.push(t1.elapsed().as_secs_f64() * 1e3);
            let t2 = Instant::now();
            let _ = forge_volume::bake_detail(&fog, &s, Some(&solid), ground);
            t_det.push(t2.elapsed().as_secs_f64() * 1e3);
            last = Some(s);
        }
        let s = last.unwrap();
        let mass: f64 = s.conc.iter().map(|v| *v as f64).sum::<f64>() * (s.spec.cell as f64).powi(3);
        let file = out.join(format!("{}.bin", name.replace(' ', "_")));
        std::fs::write(&file, bytemuck_bytes(&s.conc))?;
        let rmse = reference
            .as_ref()
            .and_then(|r| std::fs::read(r.join(file.file_name().unwrap())).ok())
            .map(|b| {
                let r: Vec<f32> = b.as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes(*c)).collect();
                if r.len() != s.conc.len() {
                    return f64::NAN;
                }
                (r.iter().zip(&s.conc).map(|(a, b)| ((a - b) as f64).powi(2)).sum::<f64>() / r.len() as f64).sqrt()
            });
        println!(
            "{name} | {:?} | {} | {:.0} | {:.0} ({}) | {:.0} | {mass:.6} | {}",
            s.spec.res,
            s.steps,
            median(t_obs),
            median(t_sim.clone()),
            t_sim.iter().map(|v| format!("{v:.0}")).collect::<Vec<_>>().join(", "),
            median(t_det),
            rmse.map(|v| format!("{v:.5}")).unwrap_or_else(|| "-".into())
        );
    }
    Ok(())
}

fn bytemuck_bytes(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}
