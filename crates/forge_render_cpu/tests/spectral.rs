//! Validazione del render spettrale contro risultati analitici e contro il render RGB.

use forge_math::{Mat4, Vec3};
use forge_render_core::{Accumulator, ColorMode, RenderBackend, RenderScene, RenderSettings};
use forge_render_cpu::CpuBackend;
use forge_scene::{primitives, Camera, ComponentId, Environment, Light, Material, MaterialId, Scene};
use std::sync::atomic::AtomicBool;

fn render(scene: &Scene, settings: &RenderSettings, spp: u32) -> Vec<[f32; 3]> {
    let rs = RenderScene::compile(scene);
    let mut be = CpuBackend::new(0).unwrap();
    let mut acc = Accumulator::new(settings.width, settings.height);
    let mut pass = vec![[0.0; 3]; acc.pixel_count()];
    let cancel = AtomicBool::new(false);
    for s in 0..spp {
        assert_eq!(be.render_pass(&rs, settings, s, 1, &mut pass, &cancel).unwrap(), 1);
        acc.add_pass(&pass);
    }
    acc.resolve()
}

fn settings(w: u32, h: u32, mode: ColorMode) -> RenderSettings {
    RenderSettings {
        width: w,
        height: h,
        max_bounces: 64,
        color_mode: mode,
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

fn sphere_scene(mat: Material, env: [f32; 3]) -> Scene {
    let mut s = Scene::default();
    let id = mat.id.clone();
    s.materials.upsert(mat);
    let m = s.add_mesh(primitives::uv_sphere(1.0, 96, 48));
    let cid = ComponentId("sfera".into());
    s.add_node(None, cid.clone(), "Sfera", Mat4::IDENTITY, Some(m));
    s.assignments.insert(cid, id);
    s.environment = Environment::Uniform { color: env, strength: 1.0 };
    s.camera = Camera {
        position: Vec3::new(0.0, 0.0, 4.0),
        target: Vec3::ZERO,
        up: Vec3::Y,
        vfov_deg: 40.0,
        ..Default::default()
    };
    s
}

fn material(base: [f32; 3], metallic: f32, roughness: f32, transmission: f32, abbe: f32) -> Material {
    Material {
        id: MaterialId::new("test"),
        base_color: base,
        metallic,
        roughness,
        transmission,
        abbe,
        ..Default::default()
    }
}

/// White furnace spettrale: un oggetto bianco senza perdite in un ambiente bianco uniforme ha
/// radianza 1 in ogni canale (diffuso, vetro liscio con e senza dispersione, vetro satinato).
#[test]
fn spectral_white_furnace() {
    for (name, m, tol) in [
        ("diffuso", material([1.0; 3], 0.0, 0.5, 0.0, 0.0), 0.03),
        ("vetro", material([1.0; 3], 0.0, 0.0, 1.0, 0.0), 0.03),
        ("vetro con dispersione", material([1.0; 3], 0.0, 0.0, 1.0, 30.0), 0.04),
        ("vetro satinato", material([1.0; 3], 0.0, 0.3, 1.0, 0.0), 0.04),
    ] {
        let mut m = m;
        m.specular = if m.transmission > 0.0 { 0.5 } else { 0.0 };
        let s = sphere_scene(m, [1.0; 3]);
        let st = settings(32, 32, ColorMode::Spectral);
        let img = render(&s, &st, 256);
        let c = mean_region(&img, 32, 12, 12, 20, 20);
        assert!((c - Vec3::ONE).abs().max_element() < tol, "{name}: {c}");
    }
}

/// Senza dispersione, materiali e luci colorati danno in media lo stesso colore del render RGB
/// per un solo rimbalzo (prodotto di due spettri lisci ≈ prodotto dei colori), e il bianco resta
/// bianco: la modalità spettrale non sposta i colori della scena.
#[test]
fn spectral_matches_rgb_on_simple_scenes() {
    for (base, env) in [
        ([0.8, 0.2, 0.1], [1.0; 3]),
        ([0.2, 0.6, 0.9], [1.0; 3]),
        ([0.7; 3], [1.0, 0.8, 0.6]),
    ] {
        let s = sphere_scene(material(base, 0.0, 0.5, 0.0, 0.0), env);
        let mut st = settings(24, 24, ColorMode::Rgb);
        st.max_bounces = 1;
        let rgb = mean_region(&render(&s, &st, 256), 24, 8, 8, 16, 16);
        st.color_mode = ColorMode::Spectral;
        let spec = mean_region(&render(&s, &st, 1024), 24, 8, 8, 16, 16);
        let rel = ((spec - rgb).abs() / rgb.max(Vec3::splat(0.02))).max_element();
        assert!(rel < 0.08, "base {base:?}, ambiente {env:?}: RGB {rgb}, spettrale {spec}");
    }
}

/// Dispersione: una luce bianca puntiforme vista attraverso una sfera di vetro molto dispersivo
/// produce bordi colorati (il rosso e il blu si separano), mentre senza dispersione l'immagine
/// spettrale resta acromatica.
#[test]
fn dispersion_separates_colours() {
    let chroma = |abbe: f32| {
        let mut s = sphere_scene(material([1.0; 3], 0.0, 0.0, 1.0, abbe), [0.0; 3]);
        s.materials.upsert(Material {
            ior: 1.7,
            abbe,
            ..material([1.0; 3], 0.0, 0.0, 1.0, abbe)
        });
        // Piano bianco dietro la sfera, illuminato da una luce piccola che passa nel vetro.
        let floor = s.add_mesh(primitives::plane(20.0, 20.0));
        let fid = ComponentId("piano".into());
        s.add_node(
            None,
            fid.clone(),
            "Piano",
            Mat4::from_translation(Vec3::new(0.0, 0.0, -2.0)) * Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2),
            Some(floor),
        );
        s.materials.upsert(Material {
            id: MaterialId::new("bianco"),
            base_color: [0.9; 3],
            ..Default::default()
        });
        s.assignments.insert(fid, MaterialId::new("bianco"));
        s.lights.push(Light::point("luce", Vec3::new(0.0, 0.0, 6.0), 400.0));
        s.camera = Camera {
            position: Vec3::new(2.5, 0.0, 3.0),
            target: Vec3::new(0.0, 0.0, -1.5),
            up: Vec3::Y,
            vfov_deg: 50.0,
            ..Default::default()
        };
        let st = settings(48, 48, ColorMode::Spectral);
        let img = render(&s, &st, 512);
        // Saturazione media dei pixel illuminati: (max − min) / max.
        let (mut sum, mut n) = (0.0f32, 0);
        for p in &img {
            let c = Vec3::from_array(*p);
            if c.max_element() > 0.05 {
                sum += (c.max_element() - c.min_element()) / c.max_element();
                n += 1;
            }
        }
        sum / n.max(1) as f32
    };
    let plain = chroma(0.0);
    let dispersive = chroma(12.0);
    eprintln!("saturazione media: senza dispersione {plain:.3}, con dispersione {dispersive:.3}");
    assert!(plain < 0.12, "{plain}");
    assert!(dispersive > plain * 1.5 && dispersive > 0.1, "{dispersive} vs {plain}");
}

/// La modalità RGB non cambia: stesso risultato con o senza il campo `abbe` (ignorato in RGB).
#[test]
fn rgb_ignores_dispersion() {
    let a = render(
        &sphere_scene(material([1.0; 3], 0.0, 0.0, 1.0, 0.0), [1.0; 3]),
        &settings(16, 16, ColorMode::Rgb),
        16,
    );
    let b = render(
        &sphere_scene(material([1.0; 3], 0.0, 0.0, 1.0, 30.0), [1.0; 3]),
        &settings(16, 16, ColorMode::Rgb),
        16,
    );
    assert_eq!(a, b);
}

/// Luce rettangolare sopra un piano diffuso, in modalità spettrale: radianza riflessa = ρ · Le · F
/// (fattore di forma analitico). Verifica il MIS tra campionamento della luce e della BSDF con le
/// quattro lunghezze d'onda.
#[test]
fn spectral_area_light_matches_form_factor() {
    let (a, b, h, le, rho) = (0.5f32, 0.75f32, 1.0f32, 3.0f32, 0.9f32);
    let mut s = Scene::default();
    s.materials.upsert(material([rho; 3], 0.0, 0.5, 0.0, 0.0));
    let m = s.add_mesh(primitives::plane(100.0, 100.0));
    let cid = ComponentId("piano".into());
    s.add_node(None, cid.clone(), "Piano", Mat4::IDENTITY, Some(m));
    s.assignments.insert(cid, MaterialId::new("test"));
    s.environment = Environment::Uniform {
        color: [0.0; 3],
        strength: 0.0,
    };
    s.camera = Camera {
        position: Vec3::new(0.0, 5.0, 0.01),
        target: Vec3::ZERO,
        up: Vec3::Y,
        vfov_deg: 0.5,
        ..Default::default()
    };
    s.lights
        .push(Light::area("a", Vec3::new(0.0, h, 0.0), Vec3::ZERO, 2.0 * a, 2.0 * b, le));
    let img = render(&s, &settings(8, 8, ColorMode::Spectral), 4096);
    let c = mean_region(&img, 8, 0, 0, 8, 8);
    let corner = |x: f32, y: f32| {
        let (ax, by) = (x / h, y / h);
        let (sa, sb) = ((1.0 + ax * ax).sqrt(), (1.0 + by * by).sqrt());
        (ax / sa * (by / sa).atan() + by / sb * (ax / sb).atan()) / (2.0 * std::f32::consts::PI)
    };
    let expected = rho * le * 4.0 * corner(a, b);
    let err = (c - Vec3::splat(expected)).abs().max_element() / expected;
    assert!(err < 0.03, "{c} vs {expected}");
}
