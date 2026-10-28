//! Test di correttezza fisica del path tracer CPU (con risultati analitici noti).

use forge_math::{Mat4, Vec3};
use forge_render_core::{Accumulator, RenderBackend, RenderScene, RenderSettings};
use forge_render_cpu::CpuBackend;
use forge_scene::{primitives, Camera, ComponentId, Environment, Light, Material, MaterialId, Scene};
use std::sync::atomic::AtomicBool;

fn render(scene: &Scene, settings: &RenderSettings, spp: u32, threads: usize) -> Vec<[f32; 3]> {
    let rs = RenderScene::compile(scene);
    let mut be = CpuBackend::new(threads).unwrap();
    let mut acc = Accumulator::new(settings.width, settings.height);
    let mut pass = vec![[0.0; 3]; acc.pixel_count()];
    let cancel = AtomicBool::new(false);
    for s in 0..spp {
        assert_eq!(be.render_pass(&rs, settings, s, 1, &mut pass, &cancel).unwrap(), 1);
        acc.add_pass(&pass);
    }
    acc.resolve()
}

fn settings(w: u32, h: u32) -> RenderSettings {
    RenderSettings {
        width: w,
        height: h,
        max_bounces: 64,
        ..Default::default()
    }
}

fn mean_region(img: &[[f32; 3]], w: u32, x0: u32, y0: u32, x1: u32, y1: u32) -> Vec3 {
    let mut s = Vec3::ZERO;
    let mut n = 0;
    for y in y0..y1 {
        for x in x0..x1 {
            s += Vec3::from_array(img[(y * w + x) as usize]);
            n += 1;
        }
    }
    s / n as f32
}

fn sphere_scene(mat: Material, env: f32) -> Scene {
    let mut s = Scene::default();
    let id = mat.id.clone();
    s.materials.upsert(mat);
    let m = s.add_mesh(primitives::uv_sphere(1.0, 96, 48));
    let cid = ComponentId("sfera".into());
    s.add_node(None, cid.clone(), "Sfera", Mat4::IDENTITY, Some(m));
    s.assignments.insert(cid, id);
    s.environment = Environment::Uniform {
        color: [1.0; 3],
        strength: env,
    };
    s.camera = Camera {
        position: Vec3::new(0.0, 0.0, 4.0),
        target: Vec3::ZERO,
        up: Vec3::Y,
        vfov_deg: 40.0,
    };
    s
}

fn test_material(base: f32, metallic: f32, roughness: f32, specular: f32, transmission: f32) -> Material {
    Material {
        id: MaterialId::new("test"),
        base_color: [base; 3],
        metallic,
        roughness,
        specular,
        transmission,
        ..Default::default()
    }
}

/// "White furnace": un oggetto convesso con albedo ρ in un ambiente uniforme di
/// radianza 1 deve apparire con radianza esattamente ρ (nessuna energia creata o persa).
#[test]
fn white_furnace_lambert() {
    for rho in [1.0f32, 0.5] {
        let s = sphere_scene(test_material(rho, 0.0, 0.5, 0.0, 0.0), 1.0);
        let img = render(&s, &settings(32, 32), 64, 4);
        let c = mean_region(&img, 32, 12, 12, 20, 20);
        assert!((c.x - rho).abs() < 0.01, "ρ={rho}: radianza {c}");
        let bg = mean_region(&img, 32, 0, 0, 2, 2);
        assert!((bg.x - 1.0).abs() < 1e-5);
    }
}

#[test]
fn white_furnace_glass_is_lossless() {
    let s = sphere_scene(test_material(1.0, 0.0, 0.0, 0.5, 1.0), 1.0);
    let img = render(&s, &settings(32, 32), 64, 4);
    let c = mean_region(&img, 32, 12, 12, 20, 20);
    assert!((c.x - 1.0).abs() < 0.02, "vetro: {c}");
}

/// Vetro satinato: sfera in ambiente uniforme. Il modello microfacet a singolo
/// scattering perde un po' di energia, ma non deve mai crearne.
#[test]
fn white_furnace_rough_glass() {
    for rough in [0.2, 0.5] {
        let s = sphere_scene(test_material(1.0, 0.0, rough, 0.5, 1.0), 1.0);
        let img = render(&s, &settings(24, 24), 128, 4);
        let c = mean_region(&img, 24, 8, 8, 16, 16);
        assert!(c.x <= 1.02 && c.x > 0.85, "vetro satinato roughness {rough}: {c}");
    }
}

#[test]
fn white_furnace_metal_and_plastic_never_gain_energy() {
    for (metallic, rough, spec) in [(1.0, 0.05, 0.5), (1.0, 0.6, 0.5), (0.0, 0.3, 0.5), (0.0, 0.8, 1.0)] {
        let s = sphere_scene(test_material(1.0, metallic, rough, spec, 0.0), 1.0);
        let img = render(&s, &settings(24, 24), 64, 4);
        let c = mean_region(&img, 24, 8, 8, 16, 16);
        assert!(c.x <= 1.01, "guadagno di energia: {c} (metallic {metallic}, roughness {rough})");
        assert!(c.x >= 0.8, "perdita eccessiva: {c} (metallic {metallic}, roughness {rough})");
        if metallic == 1.0 {
            // Con la compensazione multi-scattering un metallo bianco conserva l'energia.
            assert!(c.x >= 0.95, "metallo ruvido troppo scuro: {c} (roughness {rough})");
        }
    }
}

fn plane_scene(rho: f32) -> Scene {
    let mut s = Scene::default();
    s.materials.upsert(test_material(rho, 0.0, 0.5, 0.0, 0.0));
    let m = s.add_mesh(primitives::plane(100.0, 100.0));
    let cid = ComponentId("piano".into());
    s.add_node(None, cid.clone(), "Piano", Mat4::IDENTITY, Some(m));
    s.assignments.insert(cid, MaterialId::new("test"));
    s.environment = Environment::Uniform {
        color: [0.0; 3],
        strength: 0.0,
    };
    // Camera molto stretta che guarda il punto (0,0,0) quasi dall'alto.
    s.camera = Camera {
        position: Vec3::new(0.0, 5.0, 0.01),
        target: Vec3::ZERO,
        up: Vec3::Y,
        vfov_deg: 0.5,
    };
    s
}

/// Luce puntiforme: L = ρ/π · I / h².
#[test]
fn point_light_matches_analytic() {
    let mut s = plane_scene(0.8);
    s.lights.push(Light::point("p", Vec3::new(0.0, 2.0, 0.0), 10.0));
    let img = render(&s, &settings(8, 8), 4, 2);
    let c = mean_region(&img, 8, 0, 0, 8, 8);
    let expected = 0.8 / std::f32::consts::PI * 10.0 / 4.0;
    assert!((c.x - expected).abs() / expected < 0.01, "{} vs {expected}", c.x);
}

/// Luce rettangolare parallela al piano, centrata sopra il punto: radianza
/// riflessa = ρ · Le · F, con F fattore di forma analitico punto-rettangolo.
/// Verifica la combinazione MIS di campionamento luce e BSDF.
#[test]
fn area_light_matches_form_factor() {
    let (a, b, h, le, rho) = (0.5f32, 0.75f32, 1.0f32, 3.0f32, 0.9f32);
    let mut s = plane_scene(rho);
    s.lights
        .push(Light::area("a", Vec3::new(0.0, h, 0.0), Vec3::ZERO, 2.0 * a, 2.0 * b, le));
    let img = render(&s, &settings(8, 8), 2048, 4);
    let c = mean_region(&img, 8, 0, 0, 8, 8);
    // Fattore di forma di un rettangolo a×b con un vertice sopra il punto (Howell, C-11).
    let corner = |x: f32, y: f32| {
        let (ax, by) = (x / h, y / h);
        let (sa, sb) = ((1.0 + ax * ax).sqrt(), (1.0 + by * by).sqrt());
        (ax / sa * (by / sa).atan() + by / sb * (ax / sb).atan()) / (2.0 * std::f32::consts::PI)
    };
    let f = 4.0 * corner(a, b);
    let expected = rho * le * f;
    assert!((c.x - expected).abs() / expected < 0.02, "{} vs {expected} (F={f})", c.x);
}

/// L'errore rispetto a un riferimento cala con l'aumentare dei campioni
/// (Monte Carlo: RMSE ∝ 1/√N).
#[test]
fn progressive_convergence() {
    let s = Scene::test_scene();
    let st = RenderSettings {
        width: 24,
        height: 16,
        ..Default::default()
    };
    let reference = render(&s, &RenderSettings { seed: 999, ..st.clone() }, 1024, 4);
    let rmse = |img: &[[f32; 3]]| {
        let e: f64 = img
            .iter()
            .zip(&reference)
            .map(|(a, b)| (0..3).map(|k| ((a[k] - b[k]) as f64).powi(2)).sum::<f64>())
            .sum();
        (e / (img.len() * 3) as f64).sqrt()
    };
    let e8 = rmse(&render(&s, &st, 8, 4));
    let e128 = rmse(&render(&s, &st, 128, 4));
    assert!(e128 < e8 / 2.0, "RMSE 8spp {e8}, 128spp {e128}");
}

/// Il risultato non dipende dal numero di thread (RNG per pixel/campione).
#[test]
fn deterministic_across_thread_counts() {
    let s = Scene::test_scene();
    let st = RenderSettings {
        width: 20,
        height: 12,
        ..Default::default()
    };
    assert_eq!(render(&s, &st, 3, 1), render(&s, &st, 3, 3));
}

/// Scrive una HDRI di test in EXR: cielo uniforme + "sole" piccolo e intenso.
fn write_test_hdri(name: &str, sky: f32, sun: f32) -> std::path::PathBuf {
    let (w, h) = (128u32, 64u32);
    let mut img = image::Rgb32FImage::new(w, h);
    for (x, y, p) in img.enumerate_pixels_mut() {
        let v = if (20..23).contains(&y) && (80..83).contains(&x) { sun } else { sky };
        *p = image::Rgb([v, v, v]);
    }
    let dir = std::env::temp_dir().join(format!("forge_hdri_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    img.save(&path).unwrap();
    path
}

/// White furnace con HDRI uniforme: verifica la NEE sull'ambiente e il MIS con la BSDF.
#[test]
fn white_furnace_hdri() {
    let path = write_test_hdri("uniforme.exr", 1.0, 1.0);
    let mut s = sphere_scene(test_material(0.5, 0.0, 0.5, 0.0, 0.0), 1.0);
    s.environment = Environment::Hdri {
        path,
        strength: 1.0,
        rotation_deg: 0.0,
    };
    let img = render(&s, &settings(32, 32), 64, 4);
    let c = mean_region(&img, 32, 12, 12, 20, 20);
    assert!((c.x - 0.5).abs() < 0.01, "radianza {c}");
}

/// Piano diffuso illuminato da una HDRI con sole: L = ρ/π · E, con E calcolata
/// in modo indipendente integrando la mappa sull'emisfero superiore.
#[test]
fn hdri_with_sun_matches_reference_irradiance() {
    let path = write_test_hdri("sole.exr", 0.3, 2000.0);
    let rho = 0.7;
    let mut s = plane_scene(rho);
    s.environment = Environment::Hdri {
        path: path.clone(),
        strength: 1.0,
        rotation_deg: 30.0,
    };
    let img = render(&s, &settings(8, 8), 256, 4);
    let c = mean_region(&img, 8, 0, 0, 8, 8);

    // Riferimento: integrazione deterministica pixel per pixel della mappa (angolo solido esatto per pixel).
    let map = forge_render_core::EnvironmentMap::load(&path).unwrap();
    let (w, h) = (map.width, map.height);
    let mut e = 0.0f64;
    for y in 0..h {
        let (t0, t1) = (
            y as f64 / h as f64 * std::f64::consts::PI,
            (y + 1) as f64 / h as f64 * std::f64::consts::PI,
        );
        for x in 0..w {
            // Si integra cos θ sul pixel: ∫∫ cosθ sinθ dθ dφ, metà superiore (y > 0 ⇔ θ < π/2).
            let t1c = t1.min(std::f64::consts::FRAC_PI_2);
            if t0 >= t1c {
                continue;
            }
            let dphi = 2.0 * std::f64::consts::PI / w as f64;
            let cos_int = 0.5 * (t1c.sin().powi(2) - t0.sin().powi(2));
            let u = (x as f32 + 0.5) / w as f32;
            let v = (y as f32 + 0.5) / h as f32;
            let phi = (u - 0.5) * std::f32::consts::TAU + 30f32.to_radians();
            let th = v * std::f32::consts::PI;
            let d = Vec3::new(th.sin() * phi.sin(), th.cos(), -th.sin() * phi.cos());
            e += map.radiance(d, 30f32.to_radians()).x as f64 * cos_int * dphi;
        }
    }
    let expected = (rho as f64 / std::f64::consts::PI * e) as f32;
    assert!((c.x - expected).abs() / expected < 0.03, "{} vs {expected}", c.x);
}

fn write_png(name: &str, w: u32, h: u32, px: &[[u8; 3]]) -> std::path::PathBuf {
    let mut img = image::RgbImage::new(w, h);
    for (i, p) in img.pixels_mut().enumerate() {
        *p = image::Rgb(px[i]);
    }
    let dir = std::env::temp_dir().join(format!("forge_tex_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    img.save(&path).unwrap();
    path
}

/// Piano in white furnace con texture rosso|blu in proiezione box da 1 m:
/// la radianza deve essere il colore del texel nel punto inquadrato.
#[test]
fn box_mapped_texture_in_furnace() {
    let tex = write_png("rosso_blu.png", 2, 1, &[[255, 0, 0], [0, 0, 255]]);
    for (x, expect_red) in [(0.25f32, true), (0.75, false)] {
        let mut s = plane_scene(1.0);
        s.environment = Environment::Uniform {
            color: [1.0; 3],
            strength: 1.0,
        };
        let m = s.materials.get_mut(&MaterialId::new("test")).unwrap();
        m.base_color_texture = Some(tex.clone());
        m.mapping = forge_scene::TextureMapping {
            projection: forge_scene::Projection::Box,
            scale: 1.0,
            rotation_deg: 0.0,
            offset: [0.0; 2],
        };
        s.camera = Camera {
            position: Vec3::new(x, 5.0, 0.3),
            target: Vec3::new(x, 0.0, 0.3),
            up: Vec3::Z,
            vfov_deg: 0.3,
        };
        let img = render(&s, &settings(8, 8), 16, 2);
        let c = mean_region(&img, 8, 0, 0, 8, 8);
        let (main, other) = if expect_red { (c.x, c.z) } else { (c.z, c.x) };
        assert!(main > 0.95 && other < 0.05, "x={x}: {c}");
        assert!(c.y < 1e-4, "verde inatteso: {c}");
    }
}

/// Una normal map "piatta" non deve alterare l'immagine.
#[test]
fn flat_normal_map_is_neutral() {
    let flat = write_png("normale_piatta.png", 1, 1, &[[128, 128, 255]]);
    let mut s = Scene::test_scene();
    let st = RenderSettings {
        width: 24,
        height: 16,
        ..Default::default()
    };
    let reference = render(&s, &st, 16, 2);
    for m in &mut s.materials.materials {
        m.normal_texture = Some(flat.clone());
    }
    let with_map = render(&s, &st, 16, 2);
    let diff: f32 = reference
        .iter()
        .zip(&with_map)
        .map(|(a, b)| (0..3).map(|k| (a[k] - b[k]).abs()).sum::<f32>())
        .sum::<f32>()
        / reference.iter().map(|a| a.iter().sum::<f32>()).sum::<f32>();
    assert!(diff < 0.02, "differenza relativa {diff}");
}

/// Rettangolo emissivo (mesh, campionato con la NEE sugli emissivi + MIS) sopra un piano:
/// stesso risultato analitico della luce ad area equivalente (fattore di forma).
#[test]
fn emissive_mesh_matches_form_factor() {
    let (a, b, h, le, rho) = (0.5f32, 0.75f32, 1.0f32, 3.0f32, 0.9f32);
    let mut s = plane_scene(rho);
    s.materials.upsert(Material {
        id: MaterialId::new("luce"),
        base_color: [0.0; 3],
        emission: [1.0; 3],
        emission_strength: le,
        specular: 0.0,
        ..Default::default()
    });
    // Piano 1×1.5 m rivolto verso il basso, a quota h.
    let quad = s.add_mesh(primitives::plane(2.0 * a, 2.0 * b));
    let id = ComponentId("pannello".into());
    s.add_node(
        None,
        id.clone(),
        "Pannello",
        Mat4::from_translation(Vec3::new(0.0, h, 0.0)) * Mat4::from_rotation_x(std::f32::consts::PI),
        Some(quad),
    );
    s.assignments.insert(id, MaterialId::new("luce"));
    // Camera tra il piano e il pannello, che guarda il piano.
    s.camera = Camera {
        position: Vec3::new(0.0, 0.5, 0.01),
        target: Vec3::ZERO,
        up: Vec3::Y,
        vfov_deg: 0.5,
    };
    let img = render(&s, &settings(8, 8), 1024, 4);
    let c = mean_region(&img, 8, 0, 0, 8, 8);
    let corner = |x: f32, y: f32| {
        let (ax, by) = (x / h, y / h);
        let (sa, sb) = ((1.0 + ax * ax).sqrt(), (1.0 + by * by).sqrt());
        (ax / sa * (by / sa).atan() + by / sb * (ax / sb).atan()) / (2.0 * std::f32::consts::PI)
    };
    let expected = rho * le * 4.0 * corner(a, b);
    assert!((c.x - expected).abs() / expected < 0.02, "{} vs {expected}", c.x);
}

/// Scacchiera in scala di grigi (0/255) di `n`×`n` caselle.
fn checker_png(name: &str, n: u32) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("forge_cut_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join(name);
    let img = image::RgbImage::from_fn(n, n, |x, y| {
        if (x + y) % 2 == 0 {
            image::Rgb([255; 3])
        } else {
            image::Rgb([0; 3])
        }
    });
    img.save(&p).unwrap();
    p
}

/// Ritaglio con mappa di opacità: una lastra nera a scacchiera (metà trasparente)
/// sopra un pavimento bianco, sotto un cielo uniforme. Il pavimento riceve circa
/// metà della luce: il ritaglio vale anche per i raggi d'ombra e indiretti.
#[test]
fn opacity_cutout_lets_half_the_light_through() {
    let mut s = Scene {
        environment: Environment::Uniform {
            color: [1.0; 3],
            strength: 1.0,
        },
        ..Default::default()
    };
    let floor = Material {
        id: MaterialId::new("pavimento"),
        base_color: [1.0; 3],
        specular: 0.0,
        roughness: 1.0,
        ..Default::default()
    };
    let screen = Material {
        id: MaterialId::new("lastra"),
        base_color: [0.0; 3],
        specular: 0.0,
        roughness: 1.0,
        opacity_texture: Some(checker_png("scacchi.png", 64)),
        mapping: forge_scene::TextureMapping {
            scale: 0.64, // caselle da 1 cm
            ..Default::default()
        },
        ..Default::default()
    };
    for (m, name, y) in [(floor, "pavimento", 0.0f32), (screen, "lastra", 0.5)] {
        let id = m.id.clone();
        s.materials.upsert(m);
        let mesh = s.add_mesh(primitives::plane(200.0, 200.0));
        let cid = ComponentId(name.into());
        s.add_node(None, cid.clone(), name, Mat4::from_translation(Vec3::new(0.0, y, 0.0)), Some(mesh));
        s.assignments.insert(cid, id);
    }
    // Camera tra lastra e pavimento, che guarda il pavimento.
    s.camera = Camera {
        position: Vec3::new(0.003, 0.25, 0.0021),
        target: Vec3::new(0.003, 0.0, 0.002),
        up: Vec3::Z,
        vfov_deg: 1.0,
    };
    let st = settings(8, 8);
    let c = mean_region(&render(&s, &st, 1024, 4), 8, 0, 0, 8, 8);
    assert!((c.x - 0.5).abs() < 0.03, "radianza del pavimento {c} (attesa ≈ 0.5)");

    // Vista dall'alto attraverso la lastra: metà pavimento illuminato, metà lastra nera.
    s.camera.position = Vec3::new(0.0, 2.0, 0.001);
    s.camera.target = Vec3::ZERO;
    s.camera.vfov_deg = 20.0;
    let c = mean_region(&render(&s, &settings(32, 32), 64, 4), 32, 0, 0, 32, 32);
    assert!(c.x > 0.15 && c.x < 0.35, "vista attraverso la lastra {c} (≈ metà di ≈0.5)");
}
