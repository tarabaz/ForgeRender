//! Buffer guida del denoiser: albedo e normali coerenti con la scena.

use forge_math::{Mat4, Vec3};
use forge_render_core::{RenderScene, RenderSettings};
use forge_scene::{primitives, Camera, ComponentId, Environment, Material, MaterialId, Scene};
use std::sync::atomic::AtomicBool;

fn add_plane(s: &mut Scene, name: &str, size: f32, y: f32, mat: Material) {
    let id = mat.id.clone();
    s.materials.upsert(mat);
    let m = s.add_mesh(primitives::plane(size, size));
    let cid = ComponentId(name.into());
    s.add_node(None, cid.clone(), name, Mat4::from_translation(Vec3::new(0.0, y, 0.0)), Some(m));
    s.assignments.insert(cid, id);
}

#[test]
fn guides_see_albedo_and_normal_through_glass() {
    // Pavimento rosso visto dall'alto attraverso una lastra di vetro liscio.
    let mut s = Scene::default();
    add_plane(
        &mut s,
        "pavimento",
        8.0,
        0.0,
        Material {
            id: MaterialId::new("rosso"),
            base_color: [0.8, 0.1, 0.1],
            ..Default::default()
        },
    );
    add_plane(
        &mut s,
        "lastra",
        8.0,
        0.5,
        Material {
            id: MaterialId::new("vetro"),
            transmission: 1.0,
            roughness: 0.0,
            ..Default::default()
        },
    );
    s.environment = Environment::Uniform {
        color: [1.0; 3],
        strength: 1.0,
    };
    s.camera = Camera {
        position: Vec3::new(0.0, 3.0, 0.0),
        target: Vec3::ZERO,
        up: Vec3::Z,
        vfov_deg: 30.0,
        ..Default::default()
    };
    let rs = RenderScene::compile(&s);
    let settings = RenderSettings {
        width: 32,
        height: 32,
        ..Default::default()
    };
    let g = forge_render_cpu::render_guides(&rs, &settings, 8, &AtomicBool::new(false)).unwrap();
    let c = 16 * 32 + 16;
    let (a, n) = (g.albedo[c], g.normal[c]);
    // Attraverso il vetro si vede il rosso del pavimento (attenuato dal Fresnel).
    assert!(a[0] > 0.5 && a[1] < 0.2 && a[2] < 0.2, "albedo {a:?}");
    assert!(n[1] > 0.95, "normale {n:?}");
    // Annullamento: nessun risultato.
    assert!(forge_render_cpu::render_guides(&rs, &settings, 4, &AtomicBool::new(true)).is_none());
}
