//! Luce laser: percorso del fascio (vetri, dispersione) e render della macchia e del fascio.

use forge_math::{Mat4, Vec3};
use forge_render_core::{Accumulator, ColorMode, RenderBackend, RenderScene, RenderSettings};
use forge_render_cpu::CpuBackend;
use forge_scene::{primitives, Camera, ComponentId, Environment, Light, LightKind, Material, MaterialId, Scene};
use std::sync::atomic::AtomicBool;

/// Prisma di vetro (spigolo in alto) davanti a un laser orizzontale, pavimento bianco sotto.
fn prism_scene(abbe: f32, beam: f32) -> Scene {
    let mut s = Scene::default();
    s.materials.upsert(Material {
        id: MaterialId::new("vetro"),
        base_color: [1.0; 3],
        transmission: 1.0,
        roughness: 0.0,
        ior: 1.5,
        abbe,
        ..Default::default()
    });
    s.materials.upsert(Material {
        id: MaterialId::new("bianco"),
        base_color: [0.8; 3],
        roughness: 0.8,
        ..Default::default()
    });
    let prism = s.add_mesh(primitives::prism(0.3, 0.3));
    let pid = ComponentId("prisma".into());
    s.add_node(None, pid.clone(), "Prisma", Mat4::IDENTITY, Some(prism));
    s.assignments.insert(pid, MaterialId::new("vetro"));
    let floor = s.add_mesh(primitives::plane(10.0, 10.0));
    let fid = ComponentId("piano".into());
    s.add_node(
        None,
        fid.clone(),
        "Piano",
        Mat4::from_translation(Vec3::new(0.0, -0.5, 0.0)),
        Some(floor),
    );
    s.assignments.insert(fid, MaterialId::new("bianco"));
    s.environment = Environment::Uniform {
        color: [1.0; 3],
        strength: 0.0,
    };
    // Deviazione minima (IOR 1,5): il fascio sale di 18,6° verso la faccia sinistra, dentro il
    // prisma viaggia parallelo alla base ed esce dalla faccia destra scendendo di 18,6°.
    let mut laser = Light::point("laser", Vec3::new(-1.0, -0.268, 0.0), 0.5);
    laser.kind = LightKind::Laser { diameter_mm: 10.0 };
    laser.target = Vec3::new(-0.075, 0.0433, 0.0);
    laser.beam_density = beam;
    s.lights.push(laser);
    s
}

#[test]
fn prism_faces_point_outwards() {
    let m = primitives::prism(1.0, 1.0);
    for t in &m.indices {
        let [a, b, c] = t.map(|i| m.positions[i as usize]);
        let n = (b - a).cross(c - a);
        let centre = (a + b + c) / 3.0;
        assert!(n.dot(centre) > 0.0, "faccia rivolta verso l'interno");
    }
}

/// Il fascio entra ed esce dal prisma, deviato verso la base, e arriva sul pavimento; senza
/// dispersione tutte le lunghezze d'onda arrivano nello stesso punto, con la dispersione il
/// violetto (più rifratto) è deviato più del rosso.
#[test]
fn laser_path_refracts_and_disperses() {
    let rs = RenderScene::compile(&prism_scene(0.0, 0.0));
    assert_eq!(rs.lasers.len(), 1);
    let l = &rs.lasers[0];
    assert!(l.rgb.segments.len() >= 3, "{:?}", l.rgb.segments);
    let spot = l.rgb.spot.expect("macchia sul pavimento");
    assert!((spot.point.y + 0.5).abs() < 1e-3, "{spot:?}");
    assert!(spot.dir.y < -0.1, "deviato verso la base del prisma: {spot:?}");
    // Due superfici di vetro: perdita di Fresnel di qualche percento.
    assert!(spot.throughput.x > 0.8 && spot.throughput.x < 0.99, "{spot:?}");
    let x_at = |rs: &RenderScene, lambda: f32| {
        rs.lasers[0].spectral[forge_render_core::laser::path_index(lambda)]
            .spot
            .expect("macchia")
            .point
            .x
    };
    assert!(
        (x_at(&rs, 420.0) - x_at(&rs, 680.0)).abs() < 1e-4,
        "senza dispersione: stesso punto"
    );
    let rs = RenderScene::compile(&prism_scene(20.0, 0.0));
    let (violet, red) = (x_at(&rs, 420.0), x_at(&rs, 680.0));
    eprintln!("macchia a 420 nm: x = {violet:.4}, a 680 nm: x = {red:.4}");
    // Il fascio va verso +x e scende: il violetto, deviato di più, tocca terra prima (x minore).
    assert!(red - violet > 0.01, "violetto {violet}, rosso {red}");
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

/// Render: la macchia illumina il pavimento solo dove arriva il fascio, il fascio visibile si
/// vede solo se acceso, e nel render spettrale con la dispersione la macchia è colorata.
#[test]
fn laser_spot_and_beam_render() {
    let camera = Camera {
        position: Vec3::new(1.3, 0.3, 1.0),
        target: Vec3::new(1.65, -0.5, 0.0),
        up: Vec3::Y,
        vfov_deg: 45.0,
        ..Default::default()
    };
    let st = RenderSettings {
        width: 160,
        height: 90,
        max_bounces: 8,
        ..Default::default()
    };
    let sum = |img: &[[f32; 3]]| img.iter().map(|p| (p[0] + p[1] + p[2]) as f64).sum::<f64>();
    let mut s = prism_scene(0.0, 0.0);
    s.camera = camera.clone();
    let dark = sum(&render(&s, &st, 16));
    assert!(dark > 0.0, "la macchia deve illuminare qualcosa");
    let mut s = prism_scene(0.0, 0.3);
    s.camera = camera.clone();
    let with_beam = sum(&render(&s, &st, 16));
    eprintln!("energia immagine: senza fascio {dark:.2}, con fascio {with_beam:.2}");
    assert!(with_beam > dark * 1.05, "il fascio visibile aggiunge luce");
    // Spettrale con dispersione: pixel saturi (colori separati) attorno alla macchia.
    let mut s = prism_scene(15.0, 0.0);
    s.camera = camera;
    let img = render(
        &s,
        &RenderSettings {
            color_mode: ColorMode::Spectral,
            ..st
        },
        256,
    );
    let bright: Vec<Vec3> = img
        .iter()
        .map(|p| Vec3::from_array(*p))
        .filter(|c| c.max_element() > 0.05)
        .collect();
    let sat = bright
        .iter()
        .map(|c| (c.max_element() - c.min_element()) / c.max_element())
        .sum::<f32>()
        / bright.len().max(1) as f32;
    eprintln!("pixel illuminati {}, saturazione media {sat:.3}", bright.len());
    assert!(!bright.is_empty() && sat > 0.2, "{sat}");
}
