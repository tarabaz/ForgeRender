//! Lucciole: illuminano davvero la scena nel renderer, gli oggetti fermano la loro luce e da spente
//! non contribuiscono. L'alone non c'entra: qui si guarda solo la radianza del path tracer.

use forge_math::{Pcg32, Vec3};
use forge_render_core::{RenderScene, RenderSettings};
use forge_scene::{Camera, Environment, FireflyColor, Scene};

/// Scena buia con un pavimento e, a richiesta, un tetto tra lucciole e pavimento.
fn scene(fireflies: bool, roof: bool, time: f32) -> RenderScene {
    let mut s = Scene {
        environment: Environment::Uniform {
            color: [1.0; 3],
            strength: 0.0,
        },
        ..Default::default()
    };
    s.camera = Camera {
        position: Vec3::new(0.0, 1.0, 2.0),
        target: Vec3::ZERO,
        up: Vec3::Y,
        ..Default::default()
    };
    let mut add = |size: Vec3, at: Vec3| {
        let m = s.add_mesh(forge_scene::primitives::cuboid(size * 0.5));
        s.add_node(
            None,
            forge_scene::ComponentId(format!("{at}")),
            "pezzo",
            forge_math::Mat4::from_translation(at),
            Some(m),
        );
    };
    add(Vec3::new(8.0, 0.02, 8.0), Vec3::new(0.0, -0.01, 0.0));
    if roof {
        add(Vec3::new(8.0, 0.02, 8.0), Vec3::new(0.0, 0.3, 0.0));
    }
    let f = &mut s.fireflies;
    f.enabled = fireflies;
    f.count = 400;
    f.max_lights = 64;
    f.center = Vec3::new(0.0, 0.6, 0.0);
    f.size = Vec3::new(1.5, 0.3, 1.5);
    // Sempre accese, per un confronto pulito.
    f.off_s = [0.0, 0.0];
    f.rise_s = [0.01, 0.01];
    f.fall_s = [0.01, 0.01];
    f.on_s = [1000.0, 1000.0];
    f.intensity = [1.0, 1.0];
    f.scene_light = 1.0;
    f.palette = vec![FireflyColor {
        color: [1.0, 0.8, 0.2],
        share: 1.0,
    }];
    f.tint_share = 0.0;
    s.timeline.time = time;
    RenderScene::compile(&s)
}

fn floor_light(rs: &RenderScene) -> Vec3 {
    let st = RenderSettings::default();
    let mut rng = Pcg32::new(3, 9);
    let n = 3000;
    let mut sum = Vec3::ZERO;
    for _ in 0..n {
        let (a, b) = rng.next_2d();
        let d = (Vec3::new((a - 0.5) * 1.5, -0.5, (b - 0.5) * 1.5 - 0.5) - Vec3::ZERO).normalize();
        sum += forge_render_cpu::radiance(rs, &st, Vec3::new(0.0, 0.25, 0.5), d, &mut rng);
    }
    sum / n as f32
}

#[test]
fn fireflies_light_the_scene_and_objects_stop_it() {
    let dark = floor_light(&scene(false, false, 1.0));
    assert!(dark.max_element() < 1e-5, "senza lucciole è buio: {dark}");
    let lit = floor_light(&scene(true, false, 1.0));
    assert!(lit.x > 0.01, "il pavimento riceve luce: {lit}");
    // Il colore della luce è quello delle lucciole: rosso più del blu.
    assert!(lit.x > lit.z * 2.0, "colore: {lit}");
    // Sotto un tetto la luce non arriva.
    let shaded = floor_light(&scene(true, true, 1.0));
    assert!(shaded.x < lit.x * 0.02, "tetto: {shaded} contro {lit}");
}

/// Si muovono con il tempo: l'illuminazione nello stesso punto cambia da un istante all'altro.
#[test]
fn the_light_follows_the_timeline() {
    let a = scene(true, false, 1.0);
    let b = scene(true, false, 4.0);
    let (la, lb) = (a.lights.len(), b.lights.len());
    assert!((1..=64).contains(&la) && (1..=64).contains(&lb), "{la} {lb}");
    assert_ne!(format!("{:?}", a.lights), format!("{:?}", b.lights));
}

/// Misura del costo (non è una prova): `cargo test --release -p forge_render_cpu --test fireflies -- --ignored --nocapture`.
#[test]
#[ignore]
fn cost_with_more_lights() {
    for max in [1, 4, 12, 32, 64] {
        let mut sc = scene(true, false, 1.0);
        sc.lights.truncate(max);
        while sc.lights.len() < max {
            let again = sc.lights[0].clone();
            sc.lights.push(again);
        }
        let t = std::time::Instant::now();
        let v = floor_light(&sc);
        println!("{max} luci: {:.1} ms, media {:.3}", t.elapsed().as_secs_f64() * 1e3, v.x);
    }
}
