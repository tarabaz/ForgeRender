//! Test del backend GPU: stessi controlli fisici del backend CPU, più un
//! confronto statistico CPU/GPU. Se non c'è una GPU Vulkan/DX12 (nemmeno
//! software, es. lavapipe) i test vengono saltati con un messaggio.

use forge_math::{Mat4, Vec3};
use forge_render_core::{Accumulator, RenderBackend, RenderScene, RenderSettings};
use forge_render_cpu::CpuBackend;
use forge_render_gpu::GpuBackend;
use forge_scene::{primitives, Camera, ComponentId, Environment, Light, Material, MaterialId, Scene};
use std::sync::atomic::AtomicBool;

fn gpu() -> Option<GpuBackend> {
    match GpuBackend::new() {
        Ok(g) => Some(g),
        Err(e) => {
            eprintln!("GPU non disponibile, test saltato: {e}");
            None
        }
    }
}

fn render_with(be: &mut dyn RenderBackend, scene: &Scene, st: &RenderSettings, spp: u32) -> Vec<[f32; 3]> {
    let rs = RenderScene::compile(scene);
    let mut acc = Accumulator::new(st.width, st.height);
    let mut pass = vec![[0.0; 3]; acc.pixel_count()];
    let cancel = AtomicBool::new(false);
    while acc.samples < spp {
        let n = be.render_pass(&rs, st, acc.samples, spp - acc.samples, &mut pass, &cancel).unwrap();
        assert!(n >= 1 && acc.samples + n <= spp, "campioni restituiti non validi: {n}");
        acc.add_samples(&pass, n);
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
    for y in y0..y1 {
        for x in x0..x1 {
            s += Vec3::from_array(img[(y * w + x) as usize]);
        }
    }
    s / ((x1 - x0) * (y1 - y0)) as f32
}

fn sphere_scene(mat: Material) -> Scene {
    let mut s = Scene::default();
    let id = mat.id.clone();
    s.materials.upsert(mat);
    let m = s.add_mesh(primitives::uv_sphere(1.0, 64, 32));
    let cid = ComponentId("sfera".into());
    s.add_node(None, cid.clone(), "Sfera", Mat4::IDENTITY, Some(m));
    s.assignments.insert(cid, id);
    s.environment = Environment::Uniform {
        color: [1.0; 3],
        strength: 1.0,
    };
    s.camera = Camera {
        position: Vec3::new(0.0, 0.0, 4.0),
        target: Vec3::ZERO,
        up: Vec3::Y,
        vfov_deg: 40.0,
        ..Default::default()
    };
    s
}

fn mat(base: f32, metallic: f32, roughness: f32, specular: f32, transmission: f32) -> Material {
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

#[test]
fn gpu_white_furnace() {
    let Some(mut g) = gpu() else { return };
    for (m, expected, tol) in [
        (mat(0.5, 0.0, 0.5, 0.0, 0.0), 0.5, 0.01),
        (mat(1.0, 0.0, 0.0, 0.5, 1.0), 1.0, 0.02),
        (mat(1.0, 1.0, 0.8, 0.5, 0.0), 1.0, 0.05),
    ] {
        let s = sphere_scene(m.clone());
        let img = render_with(&mut g, &s, &settings(24, 24), 64);
        let c = mean_region(&img, 24, 8, 8, 16, 16);
        assert!((c.x - expected).abs() < tol, "{m:?}: {c}");
    }
}

/// Luce rettangolare sopra un piano: risultato analitico (fattore di forma).
#[test]
fn gpu_area_light_form_factor() {
    let Some(mut g) = gpu() else { return };
    let (a, b, h, le, rho) = (0.5f32, 0.75f32, 1.0f32, 3.0f32, 0.9f32);
    let mut s = Scene::default();
    s.materials.upsert(mat(rho, 0.0, 0.5, 0.0, 0.0));
    let m = s.add_mesh(primitives::plane(100.0, 100.0));
    let cid = ComponentId("piano".into());
    s.add_node(None, cid.clone(), "Piano", Mat4::IDENTITY, Some(m));
    s.assignments.insert(cid, MaterialId::new("test"));
    s.environment = Environment::Uniform {
        color: [0.0; 3],
        strength: 0.0,
    };
    s.camera = Camera {
        position: Vec3::new(0.0, 0.5, 0.01),
        target: Vec3::ZERO,
        up: Vec3::Y,
        vfov_deg: 0.5,
        ..Default::default()
    };
    s.lights
        .push(Light::area("a", Vec3::new(0.0, h, 0.0), Vec3::ZERO, 2.0 * a, 2.0 * b, le));
    let img = render_with(&mut g, &s, &settings(8, 8), 1024);
    let c = mean_region(&img, 8, 0, 0, 8, 8);
    let corner = |x: f32, y: f32| {
        let (ax, by) = (x / h, y / h);
        let (sa, sb) = ((1.0 + ax * ax).sqrt(), (1.0 + by * by).sqrt());
        (ax / sa * (by / sa).atan() + by / sb * (ax / sb).atan()) / (2.0 * std::f32::consts::PI)
    };
    let expected = rho * le * 4.0 * corner(a, b);
    assert!((c.x - expected).abs() / expected < 0.02, "{} vs {expected}", c.x);
}

/// La GPU converge alla stessa immagine della CPU: lo scarto GPU−CPU è dello
/// stesso ordine del rumore tra due render CPU con seed diversi.
#[test]
fn gpu_matches_cpu_statistically() {
    let Some(mut g) = gpu() else { return };
    let s = Scene::test_scene();
    let st = RenderSettings {
        width: 32,
        height: 18,
        ..Default::default()
    };
    let spp = 256;
    let mut cpu = CpuBackend::new(4).unwrap();
    let a = render_with(&mut cpu, &s, &st, spp);
    let b = render_with(&mut cpu, &s, &RenderSettings { seed: 12345, ..st.clone() }, spp);
    let gpu = render_with(&mut g, &s, &st, spp);
    let rmse = |x: &[[f32; 3]], y: &[[f32; 3]]| {
        let e: f64 = x
            .iter()
            .zip(y)
            .map(|(p, q)| (0..3).map(|k| ((p[k] - q[k]) as f64).powi(2)).sum::<f64>())
            .sum();
        (e / (x.len() * 3) as f64).sqrt()
    };
    let mean = |x: &[[f32; 3]]| x.iter().map(|p| (p[0] + p[1] + p[2]) as f64).sum::<f64>() / (x.len() * 3) as f64;
    let noise = rmse(&a, &b);
    let diff = rmse(&a, &gpu);
    let (ma, mg) = (mean(&a), mean(&gpu));
    assert!((ma - mg).abs() / ma < 0.02, "media CPU {ma} vs GPU {mg}");
    assert!(diff < noise * 1.5, "RMSE GPU−CPU {diff} vs rumore CPU−CPU {noise}");
}

fn plane_scene(rho: f32) -> Scene {
    let mut s = Scene::default();
    s.materials.upsert(mat(rho, 0.0, 0.5, 0.0, 0.0));
    let m = s.add_mesh(primitives::plane(100.0, 100.0));
    let cid = ComponentId("piano".into());
    s.add_node(None, cid.clone(), "Piano", Mat4::IDENTITY, Some(m));
    s.assignments.insert(cid, MaterialId::new("test"));
    s.environment = Environment::Uniform {
        color: [0.0; 3],
        strength: 0.0,
    };
    s.camera = Camera {
        position: Vec3::new(0.0, 0.5, 0.01),
        target: Vec3::ZERO,
        up: Vec3::Y,
        vfov_deg: 0.5,
        ..Default::default()
    };
    s
}

fn tmp(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("forge_gpu_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d.join(name)
}

/// HDRI con sole: la GPU deve dare lo stesso valore della CPU (stessa scena, molti campioni).
#[test]
fn gpu_hdri_with_sun_matches_cpu() {
    let Some(mut g) = gpu() else { return };
    let (w, h) = (128u32, 64u32);
    let mut img = image::Rgb32FImage::new(w, h);
    for (x, y, p) in img.enumerate_pixels_mut() {
        let v = if (20..23).contains(&y) && (80..83).contains(&x) {
            2000.0
        } else {
            0.3
        };
        *p = image::Rgb([v, v, v]);
    }
    let path = tmp("sole.exr");
    img.save(&path).unwrap();
    let mut s = plane_scene(0.7);
    s.environment = Environment::Hdri {
        path,
        strength: 1.0,
        rotation_deg: 30.0,
    };
    let st = settings(8, 8);
    let gv = mean_region(&render_with(&mut g, &s, &st, 512), 8, 0, 0, 8, 8);
    let cv = mean_region(&render_with(&mut CpuBackend::new(4).unwrap(), &s, &st, 512), 8, 0, 0, 8, 8);
    assert!((gv.x - cv.x).abs() / cv.x < 0.03, "GPU {gv} vs CPU {cv}");
}

/// Pannello emissivo (mesh) campionato con la NEE sugli emissivi: fattore di forma analitico.
#[test]
fn gpu_emissive_mesh_form_factor() {
    let Some(mut g) = gpu() else { return };
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
    let img = render_with(&mut g, &s, &settings(8, 8), 1024);
    let c = mean_region(&img, 8, 0, 0, 8, 8);
    let corner = |x: f32, y: f32| {
        let (ax, by) = (x / h, y / h);
        let (sa, sb) = ((1.0 + ax * ax).sqrt(), (1.0 + by * by).sqrt());
        (ax / sa * (by / sa).atan() + by / sb * (ax / sb).atan()) / (2.0 * std::f32::consts::PI)
    };
    let expected = rho * le * 4.0 * corner(a, b);
    assert!((c.x - expected).abs() / expected < 0.02, "{} vs {expected}", c.x);
}

/// Texture rosso|blu in proiezione box su un piano in white furnace (come il test CPU).
#[test]
fn gpu_box_mapped_texture() {
    let Some(mut g) = gpu() else { return };
    let mut t = image::RgbImage::new(2, 1);
    t.put_pixel(0, 0, image::Rgb([255, 0, 0]));
    t.put_pixel(1, 0, image::Rgb([0, 0, 255]));
    let path = tmp("rosso_blu.png");
    t.save(&path).unwrap();
    for (x, red) in [(0.25f32, true), (0.75, false)] {
        let mut s = plane_scene(1.0);
        s.environment = Environment::Uniform {
            color: [1.0; 3],
            strength: 1.0,
        };
        let m = s.materials.get_mut(&MaterialId::new("test")).unwrap();
        m.base_color_texture = Some(path.clone());
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
            ..Default::default()
        };
        let c = mean_region(&render_with(&mut g, &s, &settings(8, 8), 16), 8, 0, 0, 8, 8);
        let (main, other) = if red { (c.x, c.z) } else { (c.z, c.x) };
        assert!(main > 0.95 && other < 0.05 && c.y < 1e-4, "x={x}: {c}");
    }
}

/// Texture procedurale deterministica (pattern diverso per ogni `seed`).
fn pattern_png(name: &str, seed: u32, rgb: bool) -> std::path::PathBuf {
    let path = tmp(name);
    let img = image::RgbImage::from_fn(64, 64, |x, y| {
        let h = |k: u32| {
            let v = (x * 7 + y * 13 + seed * 31 + k * 101).wrapping_mul(2_654_435_761) >> 24;
            // Onde morbide + un po' di rumore: gradienti non nulli per il rilievo.
            let w = ((x as f32 * 0.3 + seed as f32).sin() * (y as f32 * 0.2).cos() * 0.5 + 0.5) * 200.0;
            (w as u32 + v / 5).min(255) as u8
        };
        if rgb {
            image::Rgb([h(0), h(1), h(2)])
        } else {
            let g = h(0);
            image::Rgb([g, g, g])
        }
    });
    img.save(&path).unwrap();
    path
}

/// Tutte le mappe PBR insieme (colore, AO, gloss, metallicità, normal DirectX,
/// rilievo, opacità con ritaglio): la GPU converge alla stessa immagine della CPU.
#[test]
fn gpu_all_texture_maps_match_cpu() {
    let Some(mut g) = gpu() else { return };
    let mut s = Scene {
        environment: Environment::Uniform {
            color: [0.6, 0.7, 0.9],
            strength: 1.0,
        },
        ..Default::default()
    };
    s.lights.push(Light::point("P", Vec3::new(0.5, 2.0, 1.0), 40.0));
    let normal = {
        // Normal map DirectX: azzurrina con variazioni (componenti attorno a 0.5, 0.5, 1).
        let path = tmp("normal_dx.png");
        image::RgbImage::from_fn(64, 64, |x, y| {
            let a = (x as f32 * 0.4).sin() * 0.3;
            let b = (y as f32 * 0.25).cos() * 0.3;
            image::Rgb([((a * 0.5 + 0.5) * 255.0) as u8, ((b * 0.5 + 0.5) * 255.0) as u8, 230])
        })
        .save(&path)
        .unwrap();
        path
    };
    let floor = Material {
        id: MaterialId::new("pavimento"),
        base_color: [1.0; 3],
        roughness: 1.0,
        metallic: 1.0,
        base_color_texture: Some(pattern_png("col.png", 1, true)),
        ao_texture: Some(pattern_png("ao.png", 2, false)),
        ao_strength: 0.8,
        roughness_texture: Some(pattern_png("gloss.png", 3, false)),
        roughness_is_gloss: true,
        metallic_texture: Some(pattern_png("metal.png", 4, false)),
        normal_texture: Some(normal),
        normal_directx: true,
        height_texture: Some(pattern_png("height.png", 5, false)),
        bump_mm: 2.0,
        mapping: forge_scene::TextureMapping {
            scale: 0.5,
            ..Default::default()
        },
        ..Default::default()
    };
    let screen = Material {
        id: MaterialId::new("griglia"),
        base_color: [0.9, 0.3, 0.1],
        roughness: 0.4,
        opacity_texture: Some(pattern_png("opacity.png", 6, false)),
        mapping: forge_scene::TextureMapping {
            scale: 0.4,
            ..Default::default()
        },
        ..Default::default()
    };
    for (m, name, size, y) in [(floor, "pavimento", 4.0f32, 0.0f32), (screen, "griglia", 1.0, 0.3)] {
        let id = m.id.clone();
        s.materials.upsert(m);
        let mesh = s.add_mesh(primitives::plane(size, size));
        let cid = ComponentId(name.into());
        s.add_node(None, cid.clone(), name, Mat4::from_translation(Vec3::new(0.0, y, 0.0)), Some(mesh));
        s.assignments.insert(cid, id);
    }
    s.camera = Camera {
        position: Vec3::new(0.0, 1.6, 2.0),
        target: Vec3::new(0.0, 0.0, 0.0),
        up: Vec3::Y,
        vfov_deg: 50.0,
        ..Default::default()
    };
    let st = RenderSettings {
        width: 32,
        height: 24,
        ..Default::default()
    };
    let spp = 256;
    let mut cpu = CpuBackend::new(4).unwrap();
    let a = render_with(&mut cpu, &s, &st, spp);
    let b = render_with(&mut cpu, &s, &RenderSettings { seed: 999, ..st.clone() }, spp);
    let gpu_img = render_with(&mut g, &s, &st, spp);
    let rmse = |x: &[[f32; 3]], y: &[[f32; 3]]| {
        let e: f64 = x
            .iter()
            .zip(y)
            .map(|(p, q)| (0..3).map(|k| ((p[k] - q[k]) as f64).powi(2)).sum::<f64>())
            .sum();
        (e / (x.len() * 3) as f64).sqrt()
    };
    let mean = |x: &[[f32; 3]]| x.iter().map(|p| (p[0] + p[1] + p[2]) as f64).sum::<f64>() / (x.len() * 3) as f64;
    let (noise, diff) = (rmse(&a, &b), rmse(&a, &gpu_img));
    let (ma, mg) = (mean(&a), mean(&gpu_img));
    eprintln!("media CPU {ma:.4} GPU {mg:.4} | RMSE CPU−CPU {noise:.4} GPU−CPU {diff:.4}");
    assert!((ma - mg).abs() / ma < 0.02, "media CPU {ma} vs GPU {mg}");
    assert!(diff < noise * 1.5, "RMSE GPU−CPU {diff} vs rumore CPU−CPU {noise}");
}

/// Profondità di campo con diaframma a 6 lamelle e decentramento: la GPU converge
/// alla stessa immagine della CPU.
#[test]
fn gpu_depth_of_field_matches_cpu() {
    let Some(mut g) = gpu() else { return };
    let mut s = Scene::test_scene();
    s.camera.lens.dof = true;
    // Diaframma molto aperto e fuoco vicino: sfocato ben visibile anche a bassa risoluzione.
    s.camera.lens.f_stop = 0.7;
    // Fuoco a 0.44 m, oggetti a ~4.4 m: cerchio di confusione ≈ 14 % dell'altezza (≈ 5 px).
    s.camera.lens.focus_distance = (s.camera.target - s.camera.position).length() * 0.1;
    s.camera.lens.blades = 6;
    s.camera.lens.blade_rotation_deg = 15.0;
    s.camera.lens.shift_y = 0.08;
    let st = RenderSettings {
        width: 64,
        height: 36,
        ..Default::default()
    };
    let spp = 128;
    let mut cpu = CpuBackend::new(4).unwrap();
    let a = render_with(&mut cpu, &s, &st, spp);
    let b = render_with(&mut cpu, &s, &RenderSettings { seed: 4321, ..st.clone() }, spp);
    let gpu_img = render_with(&mut g, &s, &st, spp);
    // Senza DOF l'immagine è diversa: il confronto ha senso.
    let mut pin = s.clone();
    pin.camera.lens.dof = false;
    let p = render_with(&mut cpu, &pin, &st, spp);
    let rmse = |x: &[[f32; 3]], y: &[[f32; 3]]| {
        let e: f64 = x
            .iter()
            .zip(y)
            .map(|(p, q)| (0..3).map(|k| ((p[k] - q[k]) as f64).powi(2)).sum::<f64>())
            .sum();
        (e / (x.len() * 3) as f64).sqrt()
    };
    let (noise, diff, vs_pinhole) = (rmse(&a, &b), rmse(&a, &gpu_img), rmse(&a, &p));
    eprintln!("RMSE CPU−CPU {noise:.4} GPU−CPU {diff:.4} DOF−pinhole {vs_pinhole:.4}");
    assert!(vs_pinhole > 2.0 * noise, "la profondità di campo deve cambiare l'immagine");
    assert!(diff < noise * 1.5, "RMSE GPU−CPU {diff} vs rumore CPU−CPU {noise}");
}

/// Displacement vero (cubo suddiviso e spostato) con colore, normal map, bump e
/// metallicità: la GPU converge alla stessa immagine della CPU.
#[test]
fn gpu_displacement_matches_cpu() {
    let Some(mut g) = gpu() else { return };
    let mut s = Scene {
        environment: Environment::Uniform {
            color: [1.0; 3],
            strength: 0.3,
        },
        ..Default::default()
    };
    s.lights.push(Light::point("P", Vec3::new(3.0, 2.0, 1.0), 40.0));
    s.materials.upsert(Material {
        id: MaterialId::new("pietra"),
        base_color_texture: Some(pattern_png("d_col.png", 1, true)),
        height_texture: Some(pattern_png("d_height.png", 5, false)),
        normal_texture: Some(pattern_png("d_nor.png", 7, true)),
        normal_directx: true,
        metallic_texture: Some(pattern_png("d_metal.png", 4, false)),
        metallic: 1.0,
        roughness: 0.6,
        bump_mm: 2.0,
        displacement_mm: 60.0,
        mapping: forge_scene::TextureMapping {
            scale: 0.5,
            ..Default::default()
        },
        ..Default::default()
    });
    let mesh = s.add_mesh(primitives::cuboid(Vec3::ONE));
    let cid = ComponentId("cubo".into());
    s.add_node(None, cid.clone(), "Cubo", Mat4::IDENTITY, Some(mesh));
    s.assignments.insert(cid, MaterialId::new("pietra"));
    s.camera = Camera {
        position: Vec3::new(1.6, 1.3, 2.2),
        target: Vec3::ZERO,
        up: Vec3::Y,
        vfov_deg: 40.0,
        ..Default::default()
    };
    let st = RenderSettings {
        width: 48,
        height: 36,
        ..Default::default()
    };
    let rs = RenderScene::compile(&s);
    assert!(rs.triangle_count() > 10_000, "geometria non suddivisa: {}", rs.triangle_count());
    let spp = 128;
    let mut cpu = CpuBackend::new(4).unwrap();
    let a = render_with(&mut cpu, &s, &st, spp);
    let b = render_with(&mut cpu, &s, &RenderSettings { seed: 999, ..st.clone() }, spp);
    let gpu_img = render_with(&mut g, &s, &st, spp);
    let rmse = |x: &[[f32; 3]], y: &[[f32; 3]]| {
        let e: f64 = x
            .iter()
            .zip(y)
            .map(|(p, q)| (0..3).map(|k| ((p[k] - q[k]) as f64).powi(2)).sum::<f64>())
            .sum();
        (e / (x.len() * 3) as f64).sqrt()
    };
    let mean = |x: &[[f32; 3]]| x.iter().map(|p| (p[0] + p[1] + p[2]) as f64).sum::<f64>() / (x.len() * 3) as f64;
    let (noise, diff) = (rmse(&a, &b), rmse(&a, &gpu_img));
    let (ma, mg) = (mean(&a), mean(&gpu_img));
    eprintln!("media CPU {ma:.4} GPU {mg:.4} | RMSE CPU−CPU {noise:.4} GPU−CPU {diff:.4}");
    assert!((ma - mg).abs() / ma < 0.02, "media CPU {ma} vs GPU {mg}");
    assert!(diff < noise * 1.5, "RMSE GPU−CPU {diff} vs rumore CPU−CPU {noise}");
}

fn render_rs(be: &mut dyn RenderBackend, rs: &RenderScene, st: &RenderSettings, spp: u32) -> Vec<[f32; 3]> {
    let mut acc = Accumulator::new(st.width, st.height);
    let mut pass = vec![[0.0; 3]; acc.pixel_count()];
    let cancel = AtomicBool::new(false);
    while acc.samples < spp {
        let n = be.render_pass(rs, st, acc.samples, spp - acc.samples, &mut pass, &cancel).unwrap();
        acc.add_samples(&pass, n);
    }
    acc.resolve()
}

/// White furnace con nebbia che solo diffonde e Beer–Lambert con nebbia assorbente, sulla GPU.
#[test]
fn gpu_fog_furnace_and_beer_lambert() {
    let Some(mut g) = gpu() else { return };
    let make = |density: f32, albedo: f32, gg: f32| {
        let mut s = Scene {
            environment: Environment::Uniform {
                color: [1.0; 3],
                strength: 1.0,
            },
            ..Default::default()
        };
        s.camera = Camera {
            position: Vec3::new(0.0, 0.0, 5.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            vfov_deg: 2.0,
            ..Default::default()
        };
        let mut rs = RenderScene::compile(&s);
        rs.fogs.push(std::sync::Arc::new(forge_volume::BakedFog::uniform(
            Vec3::splat(-0.5),
            Vec3::splat(0.5),
            density,
            Vec3::splat(albedo),
            gg,
        )));
        rs
    };
    let st = RenderSettings {
        width: 8,
        height: 8,
        max_bounces: 10_000,
        ..Default::default()
    };
    let mean = |img: &[[f32; 3]]| img.iter().map(|p| p[0]).sum::<f32>() / img.len() as f32;
    for (sigma, gg) in [(3.0f32, 0.0f32), (6.0, 0.7)] {
        let c = mean(&render_rs(&mut g, &make(sigma, 1.0, gg), &st, 256));
        assert!((c - 1.0).abs() < 0.02, "furnace σ={sigma} g={gg}: {c}");
    }
    for sigma in [0.5f32, 2.0] {
        let c = mean(&render_rs(&mut g, &make(sigma, 0.0, 0.0), &st, 512));
        let e = (-sigma).exp();
        assert!((c - e).abs() < 0.02 * e, "Beer σ={sigma}: {c} invece di {e}");
    }
}

/// Nebbia da ghiaccio secco simulata (con collisioni) in una scena con luci e oggetti:
/// la GPU converge alla stessa immagine della CPU.
#[test]
fn gpu_dry_ice_fog_matches_cpu() {
    let Some(mut g) = gpu() else { return };
    // Pavimento scuro: la nebbia bianca deve risaltare.
    let dark = |s: &mut Scene| {
        s.materials.upsert(Material {
            id: MaterialId::new("scuro"),
            base_color: [0.03; 3],
            ..Default::default()
        });
        s.assignments
            .insert(ComponentId("test/Scena/pavimento".into()), MaterialId::new("scuro"));
    };
    let mut s = Scene::test_scene();
    dark(&mut s);
    s.fogs.push(forge_scene::FogVolume {
        // Davanti agli oggetti, in vista della camera: cade e si stende sul pavimento.
        emitter: Vec3::new(0.0, 0.5, 1.4),
        size: Vec3::new(2.4, 0.9, 2.4),
        resolution: 96,
        sim_time: 4.0,
        ..Default::default()
    });
    let rs = RenderScene::compile(&s);
    assert_eq!(rs.fogs.len(), 1);
    assert!(rs.fogs[0].max_density > 0.0);
    let st = RenderSettings {
        width: 32,
        height: 18,
        ..Default::default()
    };
    let spp = 256;
    let mut cpu = CpuBackend::new(4).unwrap();
    let a = render_rs(&mut cpu, &rs, &st, spp);
    let b = render_rs(&mut cpu, &rs, &RenderSettings { seed: 999, ..st.clone() }, spp);
    let gi = render_rs(&mut g, &rs, &st, spp);
    let rmse = |x: &[[f32; 3]], y: &[[f32; 3]]| {
        let e: f64 = x
            .iter()
            .zip(y)
            .map(|(p, q)| (0..3).map(|k| ((p[k] - q[k]) as f64).powi(2)).sum::<f64>())
            .sum();
        (e / (x.len() * 3) as f64).sqrt()
    };
    let mean = |x: &[[f32; 3]]| x.iter().map(|p| (p[0] + p[1] + p[2]) as f64).sum::<f64>() / (x.len() * 3) as f64;
    let (noise, diff) = (rmse(&a, &b), rmse(&a, &gi));
    let (ma, mg) = (mean(&a), mean(&gi));
    eprintln!("nebbia: media CPU {ma:.4} GPU {mg:.4} | RMSE CPU−CPU {noise:.4} GPU−CPU {diff:.4}");
    assert!((ma - mg).abs() / ma < 0.02, "media CPU {ma} vs GPU {mg}");
    assert!(diff < noise * 1.5, "RMSE GPU−CPU {diff} vs rumore CPU−CPU {noise}");
    // La nebbia cambia davvero l'immagine.
    let mut clear = Scene::test_scene();
    dark(&mut clear);
    let c = render_with(&mut cpu, &clear, &st, spp);
    eprintln!(
        "senza nebbia media {:.4}, con {:.4}; ottica {:.4} m³",
        mean(&c),
        ma,
        rs.fogs[0].optical_volume()
    );
    assert!(
        rmse(&a, &c) > 2.0 * noise,
        "la nebbia non si vede: {} vs rumore {noise}",
        rmse(&a, &c)
    );
}
