//! Volumi trasparenti: assorbimento (legge di Beer), volumi annidati con priorità.

use forge_math::{Mat4, Vec3};
use forge_render_core::{Accumulator, RenderBackend, RenderScene, RenderSettings};
use forge_render_cpu::CpuBackend;
use forge_scene::{primitives, Camera, ComponentId, Environment, Material, MaterialId, Scene};
use std::sync::atomic::AtomicBool;

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

/// Materiale trasparente con IOR 1 (nessuna rifrazione né riflesso): resta solo l'assorbimento.
fn clear(id: &str, volume_color: [f32; 3], priority: i32) -> Material {
    Material {
        id: MaterialId::new(id),
        base_color: [1.0; 3],
        roughness: 0.0,
        transmission: 1.0,
        ior: 1.0,
        specular: 0.0,
        volume_color,
        volume_depth_cm: 100.0,
        priority,
        ..Default::default()
    }
}

/// Ambiente uniforme bianco; camera a teleobiettivo lungo −Z attraverso i cubi dati
/// (lato, materiale), tutti centrati nell'origine.
fn scene(boxes: &[(f32, Material)]) -> RenderScene {
    let mut s = Scene {
        environment: Environment::Uniform {
            color: [1.0; 3],
            strength: 1.0,
        },
        ..Default::default()
    };
    for (i, (side, m)) in boxes.iter().enumerate() {
        s.materials.upsert(m.clone());
        let mesh = s.add_mesh(primitives::cuboid(Vec3::splat(*side)));
        let id = ComponentId(format!("cubo{i}"));
        s.add_node(None, id.clone(), "cubo", Mat4::IDENTITY, Some(mesh));
        s.assignments.insert(id, m.id.clone());
    }
    s.camera = Camera {
        position: Vec3::new(0.0, 0.0, 5.0),
        target: Vec3::ZERO,
        up: Vec3::Y,
        vfov_deg: 1.0,
        ..Default::default()
    };
    RenderScene::compile(&s)
}

fn settings() -> RenderSettings {
    RenderSettings {
        width: 4,
        height: 4,
        max_bounces: 32,
        // Niente roulette russa: con 4 superfici attraversate renderebbe il test rumoroso.
        russian_roulette_start: 64,
        ..Default::default()
    }
}

#[test]
fn beer_lambert_through_a_coloured_volume() {
    // 1 m di liquido che dopo 1 m lascia passare (0.5, 0.25, 0.8).
    let rs = scene(&[(1.0, clear("liquido", [0.5, 0.25, 0.8], 0))]);
    let c = mean(&render(&rs, &settings(), 16));
    for (got, exp) in [(c.x, 0.5), (c.y, 0.25), (c.z, 0.8)] {
        assert!((got - exp).abs() < 0.01, "{c} invece di (0.5, 0.25, 0.8)");
    }
    // Metà spessore: radice del colore.
    let rs = scene(&[(0.5, clear("liquido", [0.5, 0.25, 0.8], 0))]);
    let c = mean(&render(&rs, &settings(), 16));
    assert!((c.x - 0.5f32.sqrt()).abs() < 0.01 && (c.y - 0.5).abs() < 0.01, "{c}");
}

#[test]
fn nested_volumes_follow_priority() {
    // Liquido (0.6 m, assorbe) dentro un "vetro" più grande (1 m, trasparente).
    // Liquido con priorità più alta: nella parte comune vale il liquido → 0.6 m di assorbimento.
    let rs = scene(&[(1.0, clear("vetro", [1.0; 3], 0)), (0.6, clear("liquido", [0.5; 3], 1))]);
    let c = mean(&render(&rs, &settings(), 16));
    let exp = 0.5f32.powf(0.6);
    assert!((c.x - exp).abs() < 0.01, "liquido prioritario: {c} invece di {exp}");
    // Vetro con priorità più alta: il liquido sta tutto dentro il vetro e non conta.
    let rs = scene(&[(1.0, clear("vetro", [1.0; 3], 2)), (0.6, clear("liquido", [0.5; 3], 1))]);
    let c = mean(&render(&rs, &settings(), 16));
    assert!((c.x - 1.0).abs() < 0.01, "vetro prioritario: {c} invece di 1");
    // Vetro che assorbe, liquido trasparente prioritario: assorbe solo il guscio di vetro (0.4 m).
    let rs = scene(&[(1.0, clear("vetro", [0.5; 3], 0)), (0.6, clear("liquido", [1.0; 3], 1))]);
    let c = mean(&render(&rs, &settings(), 16));
    let exp = 0.5f32.powf(0.4);
    assert!((c.x - exp).abs() < 0.01, "guscio di vetro: {c} invece di {exp}");
}
