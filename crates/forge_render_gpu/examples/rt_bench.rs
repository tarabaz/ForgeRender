//! Confronto di velocità RT core (ray query) / BVH dello shader sulla stessa GPU.
//!
//! cargo run --release -p forge_render_gpu --example rt_bench -- [campioni]

use forge_math::{Mat4, Vec3};
use forge_render_core::{Accumulator, RenderBackend, RenderScene, RenderSettings};
use forge_render_gpu::GpuBackend;
use forge_scene::{primitives, Camera, ComponentId, Environment, Light, Material, MaterialId, Scene};
use std::sync::atomic::AtomicBool;

/// Griglia di sfere (molti triangoli, come un assieme CAD) su un piano, con vetri e metalli.
fn heavy_scene(n: usize, seg: u32) -> Scene {
    let mut s = Scene::default();
    let floor = s.add_mesh(primitives::plane(40.0, 40.0));
    let fid = ComponentId("piano".into());
    s.add_node(None, fid.clone(), "Piano", Mat4::IDENTITY, Some(floor));
    for (id, base, metal, tr) in [("bianco", 0.8, 0.0, 0.0), ("metallo", 0.9, 1.0, 0.0), ("vetro", 1.0, 0.0, 1.0)] {
        s.materials.upsert(Material {
            id: MaterialId::new(id),
            base_color: [base; 3],
            metallic: metal,
            roughness: 0.25,
            transmission: tr,
            ..Default::default()
        });
    }
    s.assignments.insert(fid, MaterialId::new("bianco"));
    let mesh = s.add_mesh(primitives::uv_sphere(0.4, seg, seg / 2));
    for i in 0..n {
        for j in 0..n {
            let cid = ComponentId(format!("s{i}_{j}"));
            let x = (i as f32 - n as f32 / 2.0) * 1.0;
            let z = (j as f32 - n as f32 / 2.0) * 1.0;
            s.add_node(None, cid.clone(), "sfera", Mat4::from_translation(Vec3::new(x, 0.4, z)), Some(mesh));
            let m = ["bianco", "metallo", "vetro"][(i + j) % 3];
            s.assignments.insert(cid, MaterialId::new(m));
        }
    }
    s.environment = Environment::Uniform {
        color: [1.0; 3],
        strength: 0.3,
    };
    s.lights
        .push(Light::area("luce", Vec3::new(0.0, 8.0, 3.0), Vec3::ZERO, 4.0, 4.0, 20.0));
    s.camera = Camera {
        position: Vec3::new(0.0, 6.0, n as f32 * 0.9),
        target: Vec3::ZERO,
        up: Vec3::Y,
        vfov_deg: 50.0,
        ..Default::default()
    };
    s
}

fn time(be: &mut GpuBackend, rs: &RenderScene, st: &RenderSettings, spp: u32) -> anyhow::Result<f64> {
    let mut acc = Accumulator::new(st.width, st.height);
    let mut pass = vec![[0.0; 3]; acc.pixel_count()];
    let cancel = AtomicBool::new(false);
    // Riscaldamento: caricamento della scena, strutture, prima compilazione.
    be.render_pass(rs, st, 0, 1, &mut pass, &cancel)?;
    let t0 = std::time::Instant::now();
    while acc.samples < spp {
        let n = be.render_pass(rs, st, acc.samples + 1, spp - acc.samples, &mut pass, &cancel)?;
        acc.add_samples(&pass, n);
    }
    Ok(t0.elapsed().as_secs_f64())
}

fn main() -> anyhow::Result<()> {
    let spp: u32 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(256);
    let mut rt = GpuBackend::with_rt(true)?;
    anyhow::ensure!(rt.uses_rt_cores(), "RT core non disponibili su questa GPU");
    let mut bvh = GpuBackend::with_rt(false)?;
    let st = RenderSettings {
        width: 1280,
        height: 720,
        max_bounces: 12,
        ..Default::default()
    };
    for (name, scene) in [
        ("scena di prova", Scene::test_scene()),
        ("100 sfere, 1,6 M triangoli", heavy_scene(10, 128)),
        ("196 sfere, 3,8 M triangoli", heavy_scene(14, 140)),
    ] {
        let rs = RenderScene::compile(&scene);
        let tris = rs.geometry.prim_to_slot.len();
        let a = time(&mut bvh, &rs, &st, spp)?;
        let b = time(&mut rt, &rs, &st, spp)?;
        println!(
            "{name} ({tris} triangoli), {spp} campioni 1280×720: BVH {a:.2} s, RT core {b:.2} s → {:.2}×",
            a / b
        );
        let sp = RenderSettings {
            color_mode: forge_render_core::ColorMode::Spectral,
            ..st.clone()
        };
        let (c, d) = (time(&mut bvh, &rs, &sp, spp)?, time(&mut rt, &rs, &sp, spp)?);
        println!("  spettrale: BVH {c:.2} s, RT core {d:.2} s → {:.2}×", c / d);
    }
    Ok(())
}
