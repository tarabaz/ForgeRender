//! Nebbie volumetriche: confronto con risultati analitici.

use forge_math::{Pcg32, Vec3};
use forge_render_core::{Accumulator, RenderBackend, RenderScene, RenderSettings};
use forge_render_cpu::CpuBackend;
use forge_scene::{Camera, Environment, Scene};
use forge_volume::BakedFog;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

fn render(rs: &RenderScene, st: &RenderSettings, spp: u32) -> Vec<[f32; 3]> {
    let mut be = CpuBackend::new(4).unwrap();
    let mut acc = Accumulator::new(st.width, st.height);
    let mut pass = vec![[0.0; 3]; acc.pixel_count()];
    let cancel = AtomicBool::new(false);
    for s in 0..spp {
        be.render_pass(rs, st, s, 1, &mut pass, &cancel).unwrap();
        acc.add_pass(&pass);
    }
    acc.resolve()
}

fn mean(img: &[[f32; 3]]) -> Vec3 {
    img.iter().map(|p| Vec3::from_array(*p)).sum::<Vec3>() / img.len() as f32
}

/// Scena vuota in un ambiente uniforme di radianza 1 con un cubo di nebbia di 1 m centrato
/// nell'origine; camera a teleobiettivo che guarda attraverso il cubo lungo −z.
fn furnace(density: f32, albedo: f32, g: f32) -> RenderScene {
    let mut s = Scene {
        environment: Environment::Uniform {
            color: [1.0; 3],
            strength: 1.0,
        },
        ..Default::default()
    };
    s.camera = Camera {
        position: Vec3::new(0.0, 0.0, 5.0),
        target: Vec3::ZERO,
        up: Vec3::Y,
        vfov_deg: 2.0,
        ..Default::default()
    };
    let mut rs = RenderScene::compile(&s);
    rs.fogs.push(Arc::new(BakedFog::uniform(
        Vec3::splat(-0.5),
        Vec3::splat(0.5),
        density,
        Vec3::splat(albedo),
        g,
    )));
    rs
}

fn settings() -> RenderSettings {
    RenderSettings {
        width: 8,
        height: 8,
        max_bounces: 10_000,
        ..Default::default()
    }
}

/// Mezzo solo assorbente (albedo 0): la radianza attraverso 1 m vale exp(−σ·1).
#[test]
fn beer_lambert_through_absorbing_fog() {
    for sigma in [0.5f32, 2.0, 4.0] {
        let rs = furnace(sigma, 0.0, 0.0);
        let c = mean(&render(&rs, &settings(), 512));
        let expect = (-sigma).exp();
        assert!(
            (c.x - expect).abs() < 0.02 * expect.max(0.05),
            "σ={sigma}: {} invece di {expect}",
            c.x
        );
    }
}

/// White furnace: mezzo che solo diffonde (albedo 1) in un ambiente uniforme di radianza 1
/// deve apparire con radianza 1, per qualsiasi densità e anisotropia (energia conservata).
#[test]
fn white_furnace_scattering_fog() {
    for (sigma, g) in [(3.0f32, 0.0f32), (6.0, 0.7), (4.0, -0.5)] {
        let rs = furnace(sigma, 1.0, g);
        let c = mean(&render(&rs, &settings(), 256));
        assert!((c.x - 1.0).abs() < 0.02, "σ={sigma} g={g}: {}", c.x);
    }
}

/// La trasmittanza stimata con il ratio tracking è non distorta.
#[test]
fn ratio_tracking_transmittance_is_unbiased() {
    let rs = furnace(3.0, 0.5, 0.0);
    let mut rng = Pcg32::new(11, 3);
    let n = 40_000;
    // Raggio diagonale nel piano xz: attraversa il cubo per √2 m.
    let d = Vec3::new(1.0, 0.0, 1.0).normalize();
    let o = Vec3::new(-0.5, 0.0, -0.5) - d * 0.3;
    let t: f64 = (0..n)
        .map(|_| forge_render_cpu::medium::transmittance(&rs, o, d, 10.0, &mut rng) as f64)
        .sum::<f64>()
        / n as f64;
    let expect = (-3.0f64 * 2f64.sqrt()).exp();
    assert!((t - expect).abs() < 0.03 * expect, "{t} invece di {expect}");
}

/// Ombra attraverso la nebbia: una luce puntiforme illumina un piano diffuso
/// attraverso uno strato assorbente; l'irradianza si riduce di exp(−σ·d).
#[test]
fn point_light_shadow_through_fog() {
    use forge_math::Mat4;
    use forge_scene::{primitives, ComponentId, Light, Material, MaterialId};
    let make = |sigma: f32| {
        let mut s = Scene::default();
        s.materials.upsert(Material {
            id: MaterialId::new("bianco"),
            base_color: [0.8; 3],
            ..Default::default()
        });
        let m = s.add_mesh(primitives::plane(20.0, 20.0));
        let cid = ComponentId("piano".into());
        s.add_node(None, cid.clone(), "Piano", Mat4::IDENTITY, Some(m));
        s.assignments.insert(cid, MaterialId::new("bianco"));
        s.lights.push(Light::point("P", Vec3::new(0.0, 2.0, 0.0), 10.0));
        s.camera = Camera {
            position: Vec3::new(0.0, 3.0, 0.001),
            target: Vec3::ZERO,
            up: Vec3::Z,
            vfov_deg: 0.5,
            ..Default::default()
        };
        let mut rs = RenderScene::compile(&s);
        // Strato da y = 0.6 a 1.6 (1 m) tra luce e piano, fuori dal raggio di camera? No:
        // la camera guarda attraverso lo strato, quindi si conta due volte (andata e ritorno).
        rs.fogs.push(Arc::new(BakedFog::uniform(
            Vec3::new(-0.5, 0.6, -0.5),
            Vec3::new(0.5, 1.6, 0.5),
            sigma,
            Vec3::ZERO,
            0.0,
        )));
        rs
    };
    let st = RenderSettings {
        max_bounces: 1,
        ..settings()
    };
    let clear = mean(&render(&make(0.0), &st, 256)).x;
    let fog = mean(&render(&make(1.0), &st, 1024)).x;
    // Camera → piano attraversa 1 m di nebbia, piano → luce un altro metro.
    let expect = clear * (-2.0f32).exp();
    assert!(
        (fog - expect).abs() < 0.03 * expect,
        "{fog} invece di {expect} (senza nebbia {clear})"
    );
}
