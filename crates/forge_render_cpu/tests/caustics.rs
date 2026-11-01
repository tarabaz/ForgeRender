//! Caustiche con il photon mapping progressivo: sfera di vetro sopra un pavimento bianco.

use forge_math::{Mat4, Vec3};
use forge_render_core::{Accumulator, CausticSettings, ColorMode, RenderBackend, RenderScene, RenderSettings};
use forge_render_cpu::CpuBackend;
use forge_scene::{primitives, Camera, ComponentId, Environment, Light, Material, MaterialId, Scene};
use std::sync::atomic::AtomicBool;

/// Sfera di vetro (raggio 10 cm, centro a 15 cm) sopra un pavimento bianco: con IOR 1,5 il fuoco
/// cade sul pavimento, sotto la sfera.
fn sphere_scene(light: Light) -> Scene {
    let mut s = Scene::default();
    s.materials.upsert(Material {
        id: MaterialId::new("vetro"),
        base_color: [1.0; 3],
        transmission: 1.0,
        roughness: 0.0,
        ior: 1.5,
        ..Default::default()
    });
    s.materials.upsert(Material {
        id: MaterialId::new("bianco"),
        base_color: [0.8; 3],
        roughness: 1.0,
        ..Default::default()
    });
    let sphere = s.add_mesh(primitives::uv_sphere(0.1, 64, 32));
    let sid = ComponentId("sfera".into());
    s.add_node(
        None,
        sid.clone(),
        "Sfera",
        Mat4::from_translation(Vec3::new(0.0, 0.15, 0.0)),
        Some(sphere),
    );
    s.assignments.insert(sid, MaterialId::new("vetro"));
    let floor = s.add_mesh(primitives::plane(4.0, 4.0));
    let fid = ComponentId("piano".into());
    s.add_node(None, fid.clone(), "Piano", Mat4::IDENTITY, Some(floor));
    s.assignments.insert(fid, MaterialId::new("bianco"));
    s.environment = Environment::Uniform {
        color: [1.0; 3],
        strength: 0.0,
    };
    s.lights.push(light);
    // Dal lato, poco sopra il pavimento: si vede il pavimento sotto la sfera.
    s.camera = Camera {
        position: Vec3::new(1.0, 0.35, 0.0),
        target: Vec3::new(0.0, 0.0, 0.0),
        up: Vec3::Y,
        vfov_deg: 12.0,
        ..Default::default()
    };
    s
}

fn settings(caustics: bool) -> RenderSettings {
    RenderSettings {
        width: 64,
        height: 48,
        max_bounces: 8,
        caustics: CausticSettings {
            enabled: caustics,
            photons_k: 50,
            radius_mm: 4.0,
        },
        ..Default::default()
    }
}

fn render(scene: &Scene, st: &RenderSettings, spp: u32) -> Vec<[f32; 3]> {
    let rs = RenderScene::compile(scene);
    let mut be = CpuBackend::new(0).unwrap();
    let mut acc = Accumulator::new(st.width, st.height);
    let mut pass = vec![[0.0; 3]; acc.pixel_count()];
    let cancel = AtomicBool::new(false);
    for s in 0..spp {
        be.render_pass(&rs, st, s, 1, &mut pass, &cancel).unwrap();
        acc.add_pass(&pass);
    }
    acc.resolve()
}

fn energy(img: &[[f32; 3]]) -> f64 {
    img.iter().map(|p| (p[0] + p[1] + p[2]) as f64).sum()
}

/// Con una luce puntiforme il path tracing non può trovare la caustica (la luce non si colpisce):
/// solo con i fotoni compare la macchia concentrata sotto la sfera.
#[test]
fn point_light_caustic_appears_only_with_photons() {
    let s = sphere_scene(Light::point("p", Vec3::new(0.0, 1.0, 0.0), 2.0));
    let off = render(&s, &settings(false), 8);
    let on = render(&s, &settings(true), 8);
    let peak = |img: &[[f32; 3]]| img.iter().map(|p| p[0] + p[1] + p[2]).fold(0.0f32, f32::max);
    eprintln!(
        "energia senza {:.2} con {:.2}; picco senza {:.2} con {:.2}",
        energy(&off),
        energy(&on),
        peak(&off),
        peak(&on)
    );
    assert!(energy(&on) > energy(&off) * 1.05, "le caustiche aggiungono luce");
    assert!(peak(&on) > peak(&off) * 2.0, "la caustica è concentrata");
}

/// Con una luce ad area la caustica la trova anche il path tracing (lentamente): i fotoni devono
/// dare la stessa energia, così le unità sono verificate e la luce non si conta due volte.
#[test]
fn area_light_caustic_matches_path_tracing() {
    let light = Light::area("a", Vec3::new(0.0, 1.0, 0.0), Vec3::ZERO, 0.3, 0.3, 20.0);
    let s = sphere_scene(light);
    let reference = energy(&render(&s, &settings(false), 512));
    let photons = energy(&render(&s, &settings(true), 64));
    let rel = (photons - reference).abs() / reference;
    eprintln!("path tracing {reference:.2}, fotoni {photons:.2}, differenza {rel:.3}");
    assert!(rel < 0.08, "{rel}");
}

/// Spenti, il render non cambia; accesi nel render spettrale la caustica c'è ugualmente.
#[test]
fn caustics_off_is_unchanged_and_spectral_works() {
    let s = sphere_scene(Light::point("p", Vec3::new(0.0, 1.0, 0.0), 2.0));
    let plain = RenderSettings {
        caustics: CausticSettings::default(),
        ..settings(false)
    };
    let a = render(&s, &plain, 2);
    let b = render(&s, &settings(false), 2);
    assert_eq!(a, b);
    let sp = |c| RenderSettings {
        color_mode: ColorMode::Spectral,
        ..settings(c)
    };
    let off = energy(&render(&s, &sp(false), 8));
    let on = energy(&render(&s, &sp(true), 8));
    eprintln!("spettrale: senza {off:.2}, con {on:.2}");
    assert!(on > off * 1.05);
}
