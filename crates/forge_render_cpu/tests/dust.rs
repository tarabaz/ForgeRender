//! Polvere in sospensione: invisibile con la sola luce dell'ambiente, brilla con la luce diretta.

use forge_math::{Pcg32, Vec3};
use forge_render_core::{RenderScene, RenderSettings};
use forge_scene::{Camera, Environment, Light, LightKind, Scene};

/// Ambiente uniforme di radianza 1, polvere fitta e grossa attorno all'origine, camera a 2 m.
fn scene(sun: bool) -> RenderScene {
    let mut s = Scene {
        environment: Environment::Uniform {
            color: [1.0; 3],
            strength: 1.0,
        },
        ..Default::default()
    };
    s.camera = Camera {
        position: Vec3::new(0.0, 0.0, 2.0),
        target: Vec3::ZERO,
        up: Vec3::Y,
        ..Default::default()
    };
    if sun {
        s.lights.push(Light {
            kind: LightKind::Directional,
            target: Vec3::ZERO,
            ..Light::point("Sole", Vec3::new(0.0, 5.0, -1.0), 3.0)
        });
    }
    // Due oggetti fuori dall'inquadratura danno alla polvere il riquadro della scena.
    for (size, at) in [
        (Vec3::new(4.0, 0.01, 4.0), Vec3::new(0.0, -1.0, 0.0)),
        (Vec3::splat(0.1), Vec3::new(0.0, 1.5, -3.0)),
    ] {
        let m = s.add_mesh(forge_scene::primitives::cuboid(size * 0.5));
        s.add_node(
            None,
            forge_scene::ComponentId(format!("{at}")),
            "pezzo",
            forge_math::Mat4::from_translation(at),
            Some(m),
        );
    }
    s.haze_dust.enabled = true;
    s.haze_dust.per_m3 = 30_000.0;
    s.haze_dust.size_mm = 3.0;
    s.sync_effects();
    RenderScene::compile(&s)
}

fn mean_radiance(rs: &RenderScene) -> Vec3 {
    let st = RenderSettings::default();
    let mut rng = Pcg32::new(3, 9);
    let n = 4000;
    let mut sum = Vec3::ZERO;
    for _ in 0..n {
        let (a, b) = rng.next_2d();
        let d = Vec3::new((a - 0.5) * 0.4, (b - 0.5) * 0.4, -1.0).normalize();
        sum += forge_render_cpu::radiance(rs, &st, Vec3::new(0.0, 0.0, 2.0), d, &mut rng);
    }
    sum / n as f32
}

#[test]
fn dust_is_invisible_without_direct_light() {
    let rs = scene(false);
    assert!(rs.has_dust, "la polvere c'è");
    let m = mean_radiance(&rs);
    assert!((m - Vec3::ONE).abs().max_element() < 1e-3, "solo ambiente: {m}");
}

#[test]
fn dust_glints_in_direct_light() {
    let rs = scene(true);
    let m = mean_radiance(&rs);
    // Senza polvere: l'ambiente (1) e nessuna superficie colpita dal sole.
    assert!(m.x > 1.01, "con il sole la polvere brilla: {m}");
}
