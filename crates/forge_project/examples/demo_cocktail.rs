//! Genera il progetto demo "cocktail con ghiaccio secco" (non fa parte dell'app).
//! Uso: cargo run --release -p forge_project --example demo_cocktail -- CARTELLA
use forge_import::{import_file, ImportOptions, UpAxis};
use forge_math::Vec3;
use forge_render_core::RenderSettings;
use forge_scene::{Camera, Environment, FogVolume, LengthUnit, Light, LightKind, Material, MaterialId, Projection, Scene, FOG_PRESETS};
use std::path::PathBuf;

fn env(k: &str, d: f32) -> f32 {
    std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d)
}

fn main() -> anyhow::Result<()> {
    let dir = PathBuf::from(std::env::args().nth(1).expect("cartella"));
    let dir = std::path::absolute(&dir)?;
    let mut s = Scene::default();
    let mut sources = Vec::new();
    let mut import = |s: &mut Scene, file: &str| -> anyhow::Result<usize> {
        let path = dir.join("modelli").join(file);
        let opts = ImportOptions {
            unit: LengthUnit::Meter,
            up: UpAxis::Y,
            drop_to_floor: false,
            ..Default::default()
        };
        let before = s.nodes.len();
        import_file(s, &path, &opts)?;
        sources.push(forge_project::source_ref(Some(&dir), &path, opts));
        let root = (before..s.nodes.len()).find(|&i| s.nodes[i].parent.is_none()).expect("radice");
        let b = s.exact_bounds_of(root);
        eprintln!("{file}: {} → min {:?} max {:?}", s.nodes[root].id.0, b.min, b.max);
        Ok(root)
    };
    import(&mut s, "bar_cocktail.glb")?;
    let olives = import(&mut s, "IridescentDishWithOlives.glb")?;
    let candle = import(&mut s, "GlassHurricaneCandleHolder.glb")?;

    // Appoggia i modelli scaricati sul bancone (y = 0) nella posizione voluta.
    let place = |s: &mut Scene, root: usize, x: f32, z: f32, scale: f32, rot: f32| {
        s.nodes[root].user.scale = Vec3::splat(scale);
        s.nodes[root].user.rotation_deg = Vec3::new(0.0, rot, 0.0);
        let b = s.exact_bounds_of(root);
        let c = (b.min + b.max) * 0.5;
        s.nodes[root].user.translation += Vec3::new(x - c.x, -b.min.y, z - c.z);
    };
    // Il piatto di olive senza coperchio.
    if let Some(i) = s.nodes.iter().position(|n| n.name == "glassCover_animation") {
        s.nodes[i].visible = false;
    }
    place(&mut s, olives, env("OX", 0.19), env("OZ", -0.2), env("OS", 0.45), 20.0);
    place(&mut s, candle, env("CX", 0.2), env("CZ", -0.75), env("CS", 1.0), 0.0);
    for r in [olives, candle] {
        let b = s.exact_bounds_of(r);
        eprintln!("{} posato: min {:?} max {:?}", s.nodes[r].name, b.min, b.max);
    }

    // Legno del bancone con le texture procedurali (cartella textures del progetto).
    let tex = |n: &str| Some(dir.join("textures").join(n));
    let mut wood = Material {
        id: MaterialId::new("legno-noce-bancone"),
        name: "Legno di noce (bancone)".into(),
        base_color: [1.0; 3],
        roughness: 1.0,
        base_color_texture: tex("legno_noce_colore.png"),
        roughness_texture: tex("legno_noce_rugosita.png"),
        height_texture: tex("legno_noce_rilievo.png"),
        bump_mm: 0.4,
        category: "Legno".into(),
        ..Default::default()
    };
    wood.mapping.projection = Projection::Box;
    wood.mapping.scale = 1.2;
    s.materials.upsert(wood);
    for n in ["Bancone (mesh)", "Fronte bancone (mesh)", "Mensole (mesh)"] {
        let i = s.nodes.iter().position(|x| x.name == n).expect(n);
        let id = s.nodes[i].id.clone();
        s.assignments.insert(id, MaterialId::new("legno-noce-bancone"));
    }

    // Luci: controluce caldo dietro il bicchiere (fa brillare la nebbia), riempimento morbido,
    // luce della candela.
    let target = Vec3::new(0.0, 0.11, 0.0);
    let mut back = Light::point(
        "Controluce",
        Vec3::new(env("BX", 0.25), env("BY", 0.55), env("BZ", -0.75)),
        env("BI", 3.0),
    );
    back.kind = LightKind::Spot {
        inner_angle_deg: 18.0,
        outer_angle_deg: 34.0,
    };
    back.target = target;
    back.color = [1.0, 0.82, 0.62];
    s.lights.push(back);
    let mut key = Light::area("Softbox", Vec3::new(-0.55, 0.45, 0.45), target, 0.45, 0.35, env("KI", 1.2));
    key.color = [0.85, 0.9, 1.0];
    s.lights.push(key);
    let mut rim = Light::area("Taglio", Vec3::new(0.6, 0.35, -0.1), target, 0.15, 0.4, env("RI", 2.5));
    rim.color = [1.0, 0.75, 0.5];
    s.lights.push(rim);
    // Luce calda radente sul muro e sulle bottiglie dello scaffale.
    let mut wash = Light::area(
        "Luce scaffale",
        Vec3::new(0.0, 1.3, -1.7),
        Vec3::new(0.0, 0.3, -2.2),
        2.4,
        0.08,
        env("WI", 3.0),
    );
    wash.color = [1.0, 0.7, 0.45];
    s.lights.push(wash);
    let cb = s.exact_bounds_of(candle);
    let mut flame = Light::point(
        "Candela",
        Vec3::new(
            (cb.min.x + cb.max.x) * 0.5,
            cb.min.y + (cb.max.y - cb.min.y) * 0.45,
            (cb.min.z + cb.max.z) * 0.5,
        ),
        env("FI", 0.15),
    );
    flame.color = [1.0, 0.55, 0.2];
    s.lights.push(flame);
    s.environment = Environment::Uniform {
        color: [0.55, 0.45, 0.4],
        strength: env("AMB", 0.03),
    };

    // Ghiaccio secco nella coppa: preset "Cocktail", emettitore sul pelo del liquido.
    let mut fog = FogVolume {
        name: "Ghiaccio secco nel cocktail".into(),
        emitter: Vec3::new(0.0, env("EY", 0.142), 0.0),
        ..Default::default()
    };
    let p = FOG_PRESETS.iter().find(|p| p.name.starts_with("Cocktail")).expect("preset");
    p.apply(&mut fog);
    fog.size = Vec3::new(env("FSX", 0.6), env("FSY", 0.42), env("FSZ", 0.6));
    fog.sim_time = env("FT", fog.sim_time);
    fog.emission_rate = env("FR", fog.emission_rate);
    fog.density = env("FD", fog.density);
    fog.rise = env("RISE", fog.rise);
    s.fogs.push(fog);

    // Camera: obiettivo da ~60 mm, poco sopra il bancone, sfondo sfocato (bokeh delle lucine).
    let mut cam = Camera {
        position: Vec3::new(env("PX", 0.1), env("PY", 0.21), env("PZ", 0.85)),
        target: Vec3::new(0.0, env("TY", 0.12), 0.0),
        up: Vec3::Y,
        ..Default::default()
    };
    cam.set_focal_mm(env("MM", 42.0));
    cam.lens.dof = true;
    cam.lens.f_stop = env("F", 2.8);
    cam.lens.focus_distance = (cam.position - Vec3::new(0.0, 0.12, 0.0)).length();
    cam.lens.blades = 7;
    s.camera = cam.clone();
    s.render_cameras.push(forge_scene::RenderCamera {
        name: "Demo".into(),
        camera: cam,
    });
    s.active_camera = Some(0);
    s.refresh_pivots();

    let settings = RenderSettings {
        width: 1080,
        height: 1350,
        target_samples: 1024,
        max_bounces: 16,
        ..Default::default()
    };
    let meta = forge_project::ProjectMeta {
        path: None,
        sources,
        test_scene: false,
    };
    let out = dir.join("cocktail-ghiaccio-secco.frproject");
    forge_project::save(&out, &s, &meta, &settings)?;
    eprintln!("salvato {}", out.display());
    Ok(())
}
