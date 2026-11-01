//! Confronto RGB / spettrale sulla GPU: sfere di vetro (crown, flint, "diamante") su un piano
//! bianco, una luce piccola e intensa che crea le caustiche. Scrive `spettrale.png` (sopra RGB,
//! sotto spettrale) nella cartella indicata.
//!
//! cargo run --release -p forge_render_gpu --example spettrale -- <cartella> [campioni]

use forge_math::{Mat4, Vec3};
use forge_render_core::{tonemap_to_rgba8, Accumulator, ColorMode, RenderBackend, RenderScene, RenderSettings, ToneMapping};
use forge_render_gpu::GpuBackend;
use forge_scene::{primitives, Camera, ComponentId, Environment, Light, Material, MaterialId, Scene};
use std::sync::atomic::AtomicBool;

fn scene() -> Scene {
    let mut s = Scene::default();
    let floor = s.add_mesh(primitives::plane(30.0, 30.0));
    let fid = ComponentId("piano".into());
    s.add_node(None, fid.clone(), "Piano", Mat4::IDENTITY, Some(floor));
    s.materials.upsert(Material {
        id: MaterialId::new("bianco"),
        base_color: [0.85; 3],
        roughness: 0.6,
        ..Default::default()
    });
    s.assignments.insert(fid, MaterialId::new("bianco"));
    let sphere = s.add_mesh(primitives::uv_sphere(0.5, 128, 64));
    for (i, (name, ior, abbe)) in [("crown", 1.517f32, 64.2f32), ("flint", 1.75, 25.0), ("diamante", 2.417, 15.0)]
        .into_iter()
        .enumerate()
    {
        let id = MaterialId::new(name);
        s.materials.upsert(Material {
            id: id.clone(),
            base_color: [1.0; 3],
            transmission: 1.0,
            roughness: 0.0,
            ior,
            abbe,
            ..Default::default()
        });
        let cid = ComponentId(name.into());
        s.add_node(
            None,
            cid.clone(),
            name,
            Mat4::from_translation(Vec3::new(-1.3 + 1.3 * i as f32, 0.5, 0.0)),
            Some(sphere),
        );
        s.assignments.insert(cid, id);
    }
    s.environment = Environment::Uniform {
        color: [1.0; 3],
        strength: 0.02,
    };
    // Luce piccola e alta dietro le sfere: le caustiche cadono davanti, verso la camera.
    s.lights
        .push(Light::area("sole", Vec3::new(0.0, 6.0, -4.0), Vec3::ZERO, 0.15, 0.15, 6000.0));
    s.camera = Camera {
        position: Vec3::new(0.0, 3.2, 4.2),
        target: Vec3::new(0.0, 0.2, 0.6),
        up: Vec3::Y,
        vfov_deg: 42.0,
        ..Default::default()
    };
    s
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dir = std::path::PathBuf::from(args.first().cloned().unwrap_or_else(|| ".".into()));
    let spp: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1024);
    let s = scene();
    let rs = RenderScene::compile(&s);
    let mut g = GpuBackend::new()?;
    let (w, h) = (800u32, 450u32);
    let mut out = Vec::new();
    for mode in [ColorMode::Rgb, ColorMode::Spectral] {
        let st = RenderSettings {
            width: w,
            height: h,
            max_bounces: 24,
            color_mode: mode,
            ..Default::default()
        };
        let mut acc = Accumulator::new(w, h);
        let mut pass = vec![[0.0; 3]; acc.pixel_count()];
        let cancel = AtomicBool::new(false);
        let t0 = std::time::Instant::now();
        while acc.samples < spp {
            let n = g.render_pass(&rs, &st, acc.samples, spp - acc.samples, &mut pass, &cancel)?;
            acc.add_samples(&pass, n);
        }
        println!("{:?}: {spp} campioni in {:.1} s", mode, t0.elapsed().as_secs_f64());
        out.extend(tonemap_to_rgba8(&acc.resolve(), ToneMapping::AcesFilmic, 0.0));
    }
    let path = dir.join("spettrale.png");
    image::save_buffer(&path, &out, w, h * 2, image::ColorType::Rgba8)?;
    println!("{}", path.display());
    Ok(())
}
