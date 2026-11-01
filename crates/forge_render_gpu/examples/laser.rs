//! Laser bianco attraverso un prisma dispersivo, con il fascio visibile: sopra RGB, sotto
//! spettrale (il fascio si apre in un arcobaleno). Scrive `laser.png` nella cartella indicata.
//!
//! cargo run --release -p forge_render_gpu --example laser -- <cartella> [campioni]

use forge_math::{Mat4, Vec3};
use forge_render_core::{tonemap_to_rgba8, Accumulator, ColorMode, RenderBackend, RenderScene, RenderSettings, ToneMapping};
use forge_render_gpu::GpuBackend;
use forge_scene::{primitives, Camera, ComponentId, Environment, Light, LightKind, Material, MaterialId, Scene};
use std::sync::atomic::AtomicBool;

fn scene() -> Scene {
    let mut s = Scene::default();
    s.materials.upsert(Material {
        id: MaterialId::new("vetro"),
        base_color: [1.0; 3],
        transmission: 1.0,
        roughness: 0.0,
        ior: 1.5,
        abbe: 12.0,
        ..Default::default()
    });
    s.materials.upsert(Material {
        id: MaterialId::new("pavimento"),
        base_color: [0.6; 3],
        roughness: 0.9,
        ..Default::default()
    });
    let prism = s.add_mesh(primitives::prism(0.3, 0.3));
    let pid = ComponentId("prisma".into());
    s.add_node(None, pid.clone(), "Prisma", Mat4::IDENTITY, Some(prism));
    s.assignments.insert(pid, MaterialId::new("vetro"));
    let floor = s.add_mesh(primitives::plane(20.0, 20.0));
    let fid = ComponentId("piano".into());
    s.add_node(
        None,
        fid.clone(),
        "Piano",
        Mat4::from_translation(Vec3::new(0.0, -0.5, 0.0)),
        Some(floor),
    );
    s.assignments.insert(fid, MaterialId::new("pavimento"));
    s.environment = Environment::Uniform {
        color: [1.0; 3],
        strength: 0.01,
    };
    let mut laser = Light::point("laser", Vec3::new(-1.2, -0.336, 0.0), 0.5);
    laser.kind = LightKind::Laser { diameter_mm: 3.0 };
    laser.target = Vec3::new(-0.075, 0.0433, 0.0);
    laser.beam_density = 0.02;
    s.lights.push(laser);
    s.camera = Camera {
        position: Vec3::new(0.4, 0.35, 2.4),
        target: Vec3::new(0.45, -0.15, 0.0),
        up: Vec3::Y,
        vfov_deg: 48.0,
        ..Default::default()
    };
    s
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let dir = std::path::PathBuf::from(args.first().cloned().unwrap_or_else(|| ".".into()));
    let spp: u32 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1024);
    let rs = RenderScene::compile(&scene());
    let mut g = GpuBackend::new()?;
    let (w, h) = (960u32, 540u32);
    let mut out = Vec::new();
    for mode in [ColorMode::Rgb, ColorMode::Spectral] {
        let st = RenderSettings {
            width: w,
            height: h,
            max_bounces: 16,
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
        println!("{mode:?}: {spp} campioni in {:.1} s", t0.elapsed().as_secs_f64());
        out.extend(tonemap_to_rgba8(&acc.resolve(), ToneMapping::AcesFilmic, 0.0));
    }
    let path = dir.join("laser.png");
    image::save_buffer(&path, &out, w, h * 2, image::ColorType::Rgba8)?;
    println!("{}", path.display());
    Ok(())
}
