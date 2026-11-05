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

/// Due colori: i granelli si dividono tra due nodi (e due materiali) secondo la quota scelta,
/// senza spostare quelli dei progetti di prima.
#[test]
fn second_color_splits_the_grains() {
    let mut d = forge_scene::Dust {
        enabled: true,
        per_m3: 2000.0,
        ..Default::default()
    };
    let (one, none) = d.build_meshes(Vec3::ZERO, Vec3::ONE);
    assert!(none.indices.is_empty(), "senza secondo colore niente secondo nodo");
    d.color2_share = 0.3;
    let (a, b) = d.build_meshes(Vec3::ZERO, Vec3::ONE);
    let share = b.indices.len() as f32 / (a.indices.len() + b.indices.len()) as f32;
    assert!((share - 0.3).abs() < 0.05, "quota del secondo colore {share}");
    assert_eq!(a.indices.len() + b.indices.len(), one.indices.len());
}

/// Variazione della dimensione: 0 = granelli tutti uguali, 1 = molto più varia di prima.
#[test]
fn size_spread_controls_the_variation() {
    let sizes = |spread: f32| {
        let d = forge_scene::Dust {
            enabled: true,
            per_m3: 500.0,
            size_spread: spread,
            ..Default::default()
        };
        let m = d.build_mesh(Vec3::ZERO, Vec3::ONE);
        // Icosaedro di 12 vertici: il diametro è la distanza tra due vertici opposti (0 e 3).
        m.positions.chunks(12).map(|g| (g[0] - g[3]).length()).collect::<Vec<f32>>()
    };
    let range = |v: &[f32]| v.iter().cloned().fold(f32::MIN, f32::max) / v.iter().cloned().fold(f32::MAX, f32::min);
    assert!(range(&sizes(0.0)) < 1.001, "tutti uguali");
    let old = range(&sizes(0.5));
    assert!(old > 2.5 && old <= 4.01, "da metà al doppio: {old}");
    assert!(range(&sizes(1.0)) > old * 2.0);
}

/// Scaglie: a parità di luce poche brillano molto (quelle girate giuste) e le altre restano
/// spente; i granelli tondi brillano tutti allo stesso modo.
#[test]
fn flakes_sparkle_unevenly() {
    use forge_render_core::DustLook;
    let grains = DustLook::default();
    let flakes = DustLook {
        flakes: true,
        sparkle: 0.9,
        roughness: 0.08,
        ..Default::default()
    };
    let wo = Vec3::new(0.0, 0.0, 1.0);
    let wi = Vec3::new(0.0, 0.6, -0.8).normalize();
    let mut rng = Pcg32::new(5, 1);
    let mut g = Vec::new();
    let mut f = Vec::new();
    for _ in 0..4000 {
        let (a, b) = rng.next_2d();
        let z = 2.0 * a - 1.0;
        let r = (1.0 - z * z).sqrt();
        let n = Vec3::new(r * (std::f32::consts::TAU * b).cos(), r * (std::f32::consts::TAU * b).sin(), z);
        g.push(grains.response(Vec3::ONE, n, wo, wi).x);
        f.push(flakes.response(Vec3::ONE, n, wo, wi).x);
    }
    let max = |v: &[f32]| v.iter().cloned().fold(0.0, f32::max);
    let mean = |v: &[f32]| v.iter().sum::<f32>() / v.len() as f32;
    assert!(
        max(&g) - g.iter().cloned().fold(f32::MAX, f32::min) < 1e-6,
        "i granelli sono tutti uguali"
    );
    assert!(
        max(&f) > 50.0 * mean(&f),
        "poche scaglie molto brillanti: max {} media {}",
        max(&f),
        mean(&f)
    );
    let dark = f.iter().filter(|&&x| x < mean(&f) * 0.5).count();
    assert!(dark > f.len() / 2, "la maggior parte delle scaglie è spenta: {dark}");
    // Opacità: tutto scala con lei.
    let half = DustLook { opacity: 0.5, ..grains };
    assert!((half.response(Vec3::ONE, Vec3::Z, wo, wi).x - 0.5 * g[0]).abs() < 1e-6);
}

/// Scaglie con il secondo colore: due nodi nella scena, il render li compila come polvere; con la
/// quota a zero il secondo nodo sparisce.
#[test]
fn flakes_with_second_color_compile() {
    let mut s = forge_scene::Scene::default();
    s.haze_dust.enabled = true;
    s.haze_dust.shape = forge_scene::DustShape::Flakes;
    s.haze_dust.color2_share = 0.5;
    s.haze_dust.region = Some((Vec3::ZERO, Vec3::splat(0.5)));
    s.sync_effects();
    assert!(s.find(&forge_scene::ComponentId(forge_scene::effects::DUST_ID_2.into())).is_some());
    let rs = RenderScene::compile(&s);
    assert!(rs.has_dust && rs.dust_look.flakes);
    s.haze_dust.color2_share = 0.0;
    s.sync_effects();
    assert!(s.find(&forge_scene::ComponentId(forge_scene::effects::DUST_ID_2.into())).is_none());
}
