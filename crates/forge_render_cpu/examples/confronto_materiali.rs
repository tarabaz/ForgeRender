//! Render di confronto dei materiali predefiniti: sfera e cubo sopra un piano, luce da studio.
//! Per il "prima" si lancia con `FORGE_MATERIALS_DIR` su una cartella vuota (predefiniti senza
//! mappe), per il "dopo" normalmente.
//!
//! ```text
//! cargo run --release -p forge_render_cpu --example confronto_materiali -- <cartella> <suffisso> <id>...
//! ```

use forge_math::{Mat4, Vec3};
use forge_render_core::{Accumulator, RenderBackend, RenderScene, RenderSettings};
use forge_render_cpu::CpuBackend;
use forge_scene::{primitives, Camera, ComponentId, Environment, Light, MaterialId, Scene};
use std::sync::atomic::AtomicBool;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    anyhow::ensure!(args.len() >= 3, "uso: confronto_materiali <cartella> <suffisso> <id>...");
    let (out, suffix) = (std::path::PathBuf::from(&args[0]), &args[1]);
    std::fs::create_dir_all(&out)?;
    for id in &args[2..] {
        let scene = scene(id);
        let mut st = RenderSettings {
            width: 640,
            height: 420,
            ..Default::default()
        };
        st.target_samples = 256;
        let rs = RenderScene::compile(&scene);
        let mut be = CpuBackend::new(0)?;
        let mut acc = Accumulator::new(st.width, st.height);
        let mut pass = vec![[0.0; 3]; acc.pixel_count()];
        let cancel = AtomicBool::new(false);
        for s in 0..st.target_samples {
            be.render_pass(&rs, &st, s, 1, &mut pass, &cancel)?;
            acc.add_pass(&pass);
        }
        let rgba = forge_render_core::tonemap_to_rgba8(&acc.resolve(), st.tone_mapping, st.exposure_ev);
        let img = image::RgbaImage::from_raw(st.width, st.height, rgba).expect("dimensioni");
        let path = out.join(format!("{id}_{suffix}.png"));
        img.save(&path)?;
        println!("{}", path.display());
    }
    Ok(())
}

/// Sfera da 10 cm e cubo da 8 cm ruotato, su un piano grigio chiaro; softbox e controluce.
fn scene(id: &str) -> Scene {
    let mut s = Scene {
        environment: Environment::Uniform {
            color: [1.0; 3],
            strength: 0.25,
        },
        lights: vec![
            Light::area("Softbox", Vec3::new(0.35, 0.45, 0.35), Vec3::new(0.0, 0.05, 0.0), 0.35, 0.35, 18.0),
            Light::area("Controluce", Vec3::new(-0.3, 0.35, -0.4), Vec3::new(0.0, 0.05, 0.0), 0.3, 0.3, 10.0),
        ],
        ..Default::default()
    };
    let mat = MaterialId::new(id);
    let floor = s.add_mesh(primitives::plane(2.0, 2.0));
    s.add_node(None, ComponentId("piano".into()), "Piano", Mat4::IDENTITY, Some(floor));
    s.assignments
        .insert(ComponentId("piano".into()), MaterialId::new("vernice-opaca-bianca"));
    let sphere = s.add_mesh(primitives::uv_sphere(0.05, 96, 48));
    s.add_node(
        None,
        ComponentId("sfera".into()),
        "Sfera",
        Mat4::from_translation(Vec3::new(-0.06, 0.05, 0.0)),
        Some(sphere),
    );
    s.assignments.insert(ComponentId("sfera".into()), mat.clone());
    let cube = s.add_mesh(primitives::cuboid(Vec3::splat(0.08)));
    s.add_node(
        None,
        ComponentId("cubo".into()),
        "Cubo",
        Mat4::from_translation(Vec3::new(0.07, 0.04, -0.02)) * Mat4::from_rotation_y(0.6),
        Some(cube),
    );
    s.assignments.insert(ComponentId("cubo".into()), mat);
    s.camera = Camera {
        position: Vec3::new(0.12, 0.17, 0.32),
        target: Vec3::new(0.0, 0.04, 0.0),
        vfov_deg: 32.0,
        ..Default::default()
    };
    s
}
