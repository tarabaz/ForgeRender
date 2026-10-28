//! Portali di luce: stessa immagine (non distorta), molto meno rumore negli interni.

use forge_math::{Mat4, Vec3};
use forge_render_core::{Accumulator, RenderBackend, RenderScene, RenderSettings};
use forge_render_cpu::CpuBackend;
use forge_scene::{primitives, Camera, ComponentId, Environment, Light, Scene};
use std::sync::atomic::AtomicBool;

pub fn render(rs: &RenderScene, st: &RenderSettings, spp: u32) -> Vec<[f32; 3]> {
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

/// Stanza 4×3×4 m chiusa, con una finestra 1×1 m nella parete +X; fuori un cielo uniforme.
/// La camera dentro guarda il pavimento e la parete di fronte alla finestra.
pub fn room(with_portal: bool) -> Scene {
    let mut s = Scene {
        environment: Environment::Uniform {
            color: [1.0; 3],
            strength: 3.0,
        },
        ..Default::default()
    };
    let quad = |s: &mut Scene, name: &str, w: f32, h: f32, xf: Mat4| {
        let m = s.add_mesh(primitives::plane(w, h));
        s.add_node(None, ComponentId(name.into()), name, xf, Some(m));
    };
    use std::f32::consts::FRAC_PI_2;
    quad(&mut s, "pavimento", 4.0, 4.0, Mat4::IDENTITY);
    quad(&mut s, "soffitto", 4.0, 4.0, Mat4::from_translation(Vec3::Y * 3.0));
    let wall_x = |x: f32| Mat4::from_translation(Vec3::new(x, 1.5, 0.0)) * Mat4::from_rotation_z(FRAC_PI_2);
    let wall_z = |z: f32| Mat4::from_translation(Vec3::new(0.0, 1.5, z)) * Mat4::from_rotation_x(FRAC_PI_2);
    quad(&mut s, "parete -x", 3.0, 4.0, wall_x(-2.0));
    quad(&mut s, "parete -z", 4.0, 3.0, wall_z(-2.0));
    quad(&mut s, "parete +z", 4.0, 3.0, wall_z(2.0));
    // Parete +X con il foro della finestra (y 1–2, z −0.5–0.5): quattro pezzi attorno.
    let piece = |y: f32, z: f32| Mat4::from_translation(Vec3::new(2.0, y, z)) * Mat4::from_rotation_z(FRAC_PI_2);
    quad(&mut s, "sotto", 1.0, 4.0, piece(0.5, 0.0));
    quad(&mut s, "sopra", 1.0, 4.0, piece(2.5, 0.0));
    quad(&mut s, "sinistra", 1.0, 1.5, piece(1.5, -1.25));
    quad(&mut s, "destra", 1.0, 1.5, piece(1.5, 1.25));
    if with_portal {
        s.lights.push(Light::portal(
            "Finestra",
            Vec3::new(2.0, 1.5, 0.0),
            Vec3::new(0.0, 1.5, 0.0),
            1.0,
            1.0,
        ));
    }
    s.camera = Camera {
        position: Vec3::new(1.5, 1.6, 1.5),
        target: Vec3::new(-2.0, 0.6, -1.0),
        up: Vec3::Y,
        vfov_deg: 70.0,
        ..Default::default()
    };
    s
}

fn settings() -> RenderSettings {
    RenderSettings {
        width: 24,
        height: 16,
        // Tanti rimbalzi: col limite basso la NEE dell'ultimo vertice porta luce che senza
        // portale andrebbe persa col troncamento, e le medie differirebbero di qualche %.
        max_bounces: 64,
        ..Default::default()
    }
}

fn mean(img: &[[f32; 3]]) -> f64 {
    img.iter().map(|p| p[0] as f64).sum::<f64>() / img.len() as f64
}

fn rmse(a: &[[f32; 3]], b: &[[f32; 3]]) -> f64 {
    (a.iter().zip(b).map(|(x, y)| ((x[0] - y[0]) as f64).powi(2)).sum::<f64>() / a.len() as f64).sqrt()
}

#[test]
fn portal_is_unbiased_and_reduces_noise() {
    let st = settings();
    let plain = RenderScene::compile(&room(false));
    let portal = RenderScene::compile(&room(true));
    assert_eq!(portal.portals.list.len(), 1);
    assert!(portal.lights.is_empty(), "il portale non è una luce che emette");
    // Riferimento: tanti campioni con il portale (converge in fretta), confrontato con
    // la media senza portale per la non distorsione.
    let reference = render(&portal, &st, 4096);
    let plain_ref = render(&plain, &RenderSettings { seed: 99, ..st.clone() }, 4096);
    let (mp, mr) = (mean(&plain_ref), mean(&reference));
    assert!((mp - mr).abs() < 0.03 * mr, "media senza portale {mp:.4}, con portale {mr:.4}");
    // A parità di campioni il portale ha molto meno rumore.
    let st2 = RenderSettings { seed: 7, ..st.clone() };
    let e_plain = rmse(&render(&plain, &st2, 64), &reference);
    let e_portal = rmse(&render(&portal, &st2, 64), &reference);
    eprintln!("errore a 64 campioni: senza portale {e_plain:.4}, con portale {e_portal:.4} (media {mr:.4})");
    assert!(e_portal < 0.5 * e_plain, "senza {e_plain:.4}, con {e_portal:.4}");
}
