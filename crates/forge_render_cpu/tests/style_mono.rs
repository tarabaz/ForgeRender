//! Stili in bianco e nero: anche quello che si vede attraverso un vetro colorato resta grigio.

use forge_math::{Mat4, Vec3};
use forge_render_core::{Accumulator, ColorMode, RenderBackend, RenderScene, RenderSettings, RenderStyle};
use forge_render_cpu::CpuBackend;
use forge_scene::{primitives, Camera, ComponentId, Environment, Light, Material, MaterialId, Scene};
use std::sync::atomic::AtomicBool;

/// Sfera di vetro arancione davanti a un pavimento colorato, vista dall'alto di sbieco.
fn scene() -> Scene {
    let mut s = Scene::default();
    s.materials.upsert(Material {
        id: MaterialId::new("vetro-arancio"),
        base_color: [1.0, 0.35, 0.05],
        transmission: 1.0,
        roughness: 0.0,
        ior: 1.5,
        ..Default::default()
    });
    s.materials.upsert(Material {
        id: MaterialId::new("legno"),
        base_color: [0.6, 0.35, 0.15],
        roughness: 0.8,
        ..Default::default()
    });
    let sphere = s.add_mesh(primitives::uv_sphere(0.1, 48, 24));
    let sid = ComponentId("sfera".into());
    s.add_node(
        None,
        sid.clone(),
        "Sfera",
        Mat4::from_translation(Vec3::new(0.0, 0.1, 0.0)),
        Some(sphere),
    );
    s.assignments.insert(sid, MaterialId::new("vetro-arancio"));
    let floor = s.add_mesh(primitives::plane(4.0, 4.0));
    let fid = ComponentId("piano".into());
    s.add_node(None, fid.clone(), "Piano", Mat4::IDENTITY, Some(floor));
    s.assignments.insert(fid, MaterialId::new("legno"));
    s.environment = Environment::Uniform {
        color: [0.6, 0.8, 1.0],
        strength: 0.5,
    };
    s.lights
        .push(Light::area("a", Vec3::new(0.3, 1.0, 0.3), Vec3::ZERO, 0.4, 0.4, 20.0));
    s.camera = Camera {
        position: Vec3::new(0.6, 0.5, 0.6),
        target: Vec3::new(0.0, 0.08, 0.0),
        up: Vec3::Y,
        vfov_deg: 20.0,
        ..Default::default()
    };
    s
}

fn render(style: RenderStyle, mono: bool, color_mode: ColorMode) -> Vec<[f32; 3]> {
    let st = RenderSettings {
        width: 48,
        height: 36,
        max_bounces: 8,
        style,
        style_mono: mono,
        color_mode,
        ..Default::default()
    };
    let rs = RenderScene::compile(&scene());
    let mut be = CpuBackend::new(0).unwrap();
    let mut acc = Accumulator::new(st.width, st.height);
    let mut pass = vec![[0.0; 3]; acc.pixel_count()];
    let cancel = AtomicBool::new(false);
    for s in 0..4 {
        be.render_pass(&rs, &st, s, 1, &mut pass, &cancel).unwrap();
        acc.add_pass(&pass);
    }
    acc.resolve()
}

/// Massima differenza tra i canali di un pixel (0 = grigio).
fn max_chroma(img: &[[f32; 3]]) -> f32 {
    img.iter()
        .map(|p| p[0].max(p[1]).max(p[2]) - p[0].min(p[1]).min(p[2]))
        .fold(0.0, f32::max)
}

#[test]
fn mono_styles_are_gray_even_through_coloured_glass() {
    // A colori il vetro arancione si vede (la prova che la scena è colorata).
    assert!(max_chroma(&render(RenderStyle::Toon, false, ColorMode::Rgb)) > 0.1);
    for style in [RenderStyle::Toon, RenderStyle::Technical] {
        for mode in [ColorMode::Rgb, ColorMode::Spectral] {
            let c = max_chroma(&render(style, true, mode));
            assert!(c < 1e-4, "{style:?} {mode:?}: colore {c}");
        }
    }
}
