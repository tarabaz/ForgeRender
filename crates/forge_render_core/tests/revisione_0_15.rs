//! Verifiche della revisione 0.15.0 (proiezione Box sugli assi dell'oggetto e sfumata).
//!
//! I primi due test riproducevano problemi reali (soglia dei pesi a gradino, displacement con
//! un solo asse per triangolo): erano `#[ignore]` e fallivano; dalla 0.16.0 sono obbligatori.

use forge_math::{Mat4, Quat, Vec3};
use forge_render_core::displace::{self, box_weights, BoxFrame, Params};
use forge_render_core::texture::Texture;
use forge_render_core::RenderScene;
use forge_scene::{primitives, ComponentId, Scene, TextureMapping, UserTransform};
use std::collections::HashMap;
use std::sync::Arc;

/// Lato orientato, dai vertici quantizzati.
type EdgeKey = ([i64; 3], [i64; 3]);

fn key(v: Vec3) -> [i64; 3] {
    [v.x, v.y, v.z].map(|x| (x * 1e5).round() as i64)
}

/// Pesi della sfumatura lungo un arco da +X a +Y (passo 0,001°): massimo salto tra due passi.
fn max_weight_step() -> (f32, f32) {
    let (mut worst, mut at) = (0.0f32, 0.0f32);
    let mut prev = box_weights(Vec3::X);
    let steps = 90_000;
    for i in 1..=steps {
        let deg = 90.0 * i as f32 / steps as f32;
        let a = deg.to_radians();
        let w = box_weights(Vec3::new(a.cos(), a.sin(), 0.0));
        let d = (0..3).map(|k| (w[k] - prev[k]).abs()).fold(0.0, f32::max);
        if d > worst {
            (worst, at) = (d, deg);
        }
        prev = w;
    }
    (worst, at)
}

/// I pesi variano con continuità: con passi di 0,001° nessun salto oltre 0,002 (con la
/// soglia a gradino della 0.15.0 il salto era 0,0100 a 60,6°).
#[test]
fn box_weights_are_continuous() {
    let (worst, at) = max_weight_step();
    eprintln!("salto massimo dei pesi: {worst:.5} a {at:.3}°");
    assert!(worst < 0.002, "salto {worst} a {at}°");
}

/// Sulla sfera (normali lisce) la sfumatura copre tutte le transizioni: nessun punto ha i
/// pesi di una sola proiezione dove la normale è a 45° tra due assi.
#[test]
fn box_weights_blend_at_45_degrees() {
    let w = box_weights(Vec3::new(1.0, 1.0, 0.0).normalize());
    assert!((w[0] - 0.5).abs() < 1e-5 && (w[1] - 0.5).abs() < 1e-5, "{w:?}");
    let w = box_weights(Vec3::ONE.normalize());
    assert!(w.iter().all(|x| (x - 1.0 / 3.0).abs() < 1e-5), "{w:?}");
}

fn noise_tex() -> Texture {
    let n = 32;
    let data = (0..n * n)
        .map(|i| {
            let v = ((i * 7919 + (i / n) * 104729) % 251) as u8;
            [v, v, v]
        })
        .collect();
    Texture::from_rgb8(n, n, data, false)
}

/// Displacement su una superficie curva senza UV, con la proiezione box sfumata:
/// - stesso punto della superficie, stessi dati di shading (altezza e direzione) da tutti i
///   triangoli che lo toccano (nella 0.15.0: 796 vertici su 45 506 diversi, cuciture);
/// - il punto originale ricostruito dallo shading sta sulla sfera di partenza;
/// - l'altezza applicata è quella della proiezione sfumata in quel punto (displacement e
///   shading leggono le stesse coordinate);
/// - nessun lato aperto (niente crepe).
#[test]
fn displaced_curved_surface_has_continuous_texture_coordinates() {
    let mesh = primitives::uv_sphere(0.5, 48, 24);
    let params = Params {
        height: Arc::new(noise_tex()),
        mapping: TextureMapping {
            scale: 0.25,
            ..Default::default()
        },
        amount_m: 0.005,
        mid: 0.5,
        use_uv: false,
        box_unit_m: 1.0,
    };
    let frame = BoxFrame::default();
    let out = displace::tessellate(&mesh, &Mat4::IDENTITY, false, &params, 0.02, &frame);
    assert!(out.iter().all(|t| t.flags == displace::FLAG_DISPLACED_BOX));
    let mut seen: HashMap<[i64; 3], Vec<(f32, f32)>> = HashMap::new();
    let (mut off_sphere, mut exact, mut count) = (0.0f32, 0usize, 0usize);
    for t in &out {
        for k in 0..3 {
            let (h, code) = (t.uv[k].x, t.uv[k].y);
            seen.entry(key(t.p[k])).or_default().push((h, code));
            let d = displace::unpack_dir(code);
            let orig = t.p[k] - d * h;
            off_sphere = off_sphere.max((orig.length() - 0.5).abs());
            count += 1;
            if (params.height_box(&frame, orig, d) - h).abs() < 5e-5 {
                exact += 1;
            }
        }
    }
    let total = seen.len();
    let broken = seen.values().filter(|v| v.iter().any(|x| *x != v[0])).count();
    // Crepe: i lati che non hanno il gemello in verso opposto sono giunzioni a T (triangoli
    // vicini suddivisi diversamente). Senza crepe il punto medio di ognuno sta su un lato
    // dell'altra parte.
    let mut edges: HashMap<EdgeKey, (i32, Vec3, Vec3)> = HashMap::new();
    for t in &out {
        for e in 0..3 {
            let (pa, pb) = (t.p[e], t.p[(e + 1) % 3]);
            let (a, b) = (key(pa), key(pb));
            edges.entry((a, b)).or_insert((0, pa, pb)).0 += 1;
            edges.entry((b, a)).or_insert((0, pb, pa)).0 -= 1;
        }
    }
    let open: Vec<(Vec3, Vec3)> = edges.values().filter(|e| e.0 > 0).map(|e| (e.1, e.2)).collect();
    let on_segment = |p: Vec3, a: Vec3, b: Vec3| {
        let ab = b - a;
        let t = ((p - a).dot(ab) / ab.length_squared().max(1e-20)).clamp(0.0, 1.0);
        (a + ab * t - p).length()
    };
    let cracks = open
        .iter()
        .filter(|(a, b)| {
            let m = (*a + *b) * 0.5;
            !open.iter().any(|(c, d)| (*c, *d) != (*a, *b) && on_segment(m, *c, *d) < 1e-5)
        })
        .count();
    eprintln!(
        "{} triangoli; vertici con dati diversi: {broken} su {total}; punto originale fuori sfera \
         {off_sphere:.2e} m; altezza esatta in {exact} vertici su {count}; giunzioni a T {}, crepe {cracks}",
        out.len(),
        open.len()
    );
    assert_eq!(broken, 0);
    // La sfera a 48 × 24 è un poliedro: i punti interni dei triangoli stanno sotto la sfera
    // ideale di al massimo la freccia delle corde (circa 0,5 × (1 − cos 7,5°) = 4,3 mm).
    assert!(off_sphere < 5e-3, "{off_sphere}");
    // I vertici calcolati hanno l'altezza della proiezione sfumata nel loro punto; fanno
    // eccezione i punti di raccordo delle giunzioni a T, interpolati tra due campioni del lato
    // (su questa sfera sono fitti vicino ai poli, dove i triangoli cambiano dimensione: circa
    // l'11 % delle occorrenze). Lo shading li ricostruisce comunque sulla superficie originale.
    assert!(exact as f64 > 0.85 * count as f64, "{exact} su {count}");
    assert_eq!(cracks, 0, "{cracks} crepe");
}

/// Componente con un cubo da 0,8 m nella scena, con trasformazioni sorgente e utente.
fn cube_scene(src: Mat4, user: UserTransform) -> (Scene, usize) {
    let mut s = Scene::default();
    let m = s.add_mesh(primitives::cuboid(Vec3::splat(0.8)));
    let i = s.add_node(None, ComponentId("cubo".into()), "Cubo", src, Some(m));
    s.refresh_pivots();
    s.nodes[i].user = user;
    (s, i)
}

/// Ogni faccia del cubo usa una sola proiezione (normale lungo un asse dell'oggetto).
fn faces_use_one_axis(rs: &RenderScene) -> bool {
    let geo = &rs.geometry;
    (0..geo.tri_node.len()).all(|prim| {
        let f = rs.node_frames[geo.tri_node[prim] as usize];
        let w = box_weights(f.dir_to_local(geo.geometric_normals[prim]));
        w.iter().filter(|&&x| x > 0.0).count() == 1
    })
}

/// Rotazioni arbitrarie, specchiatura nel file e scala non uniforme: le facce del cubo restano
/// su un solo asse e le coordinate locali sono finite.
#[test]
fn box_frame_with_rotation_mirror_and_non_uniform_scale() {
    let rot = Mat4::from_quat(Quat::from_euler(forge_math::glam::EulerRot::XYZ, 0.3, 1.1, -0.7));
    let cases = [
        ("ruotato", rot, UserTransform::default()),
        (
            "specchiato",
            rot * Mat4::from_scale(Vec3::new(-1.0, 1.0, 1.0)),
            UserTransform::default(),
        ),
        (
            "scala non uniforme",
            rot,
            UserTransform {
                rotation_deg: Vec3::new(20.0, -35.0, 10.0),
                scale: Vec3::new(2.0, 1.0, 0.5),
                ..Default::default()
            },
        ),
    ];
    for (name, src, user) in cases {
        let (s, _) = cube_scene(src, user);
        let rs = RenderScene::compile(&s);
        let geo = &rs.geometry;
        assert_eq!(geo.tri_node.len(), 12, "{name}");
        assert!(faces_use_one_axis(&rs), "{name}: una faccia mescola più proiezioni");
        for prim in 0..12 {
            let f = rs.node_frames[geo.tri_node[prim] as usize];
            assert!(geo.vertices(prim).iter().all(|v| f.to_local(*v).is_finite()), "{name}");
        }
    }
}

/// Assieme con due pezzi accostati e orientati allo stesso modo: lo stesso punto del lato in
/// comune ha le stesse coordinate locali dai due pezzi, anche dopo aver ruotato e spostato
/// l'assieme. Spostando solo un pezzo, la texture segue il pezzo (modalità attuale).
#[test]
fn assembly_parts_share_box_coordinates() {
    let mut s = Scene::default();
    let m = s.add_mesh(primitives::cuboid(Vec3::ONE));
    let root = s.add_node(
        None,
        ComponentId("a".into()),
        "Assieme",
        Mat4::from_translation(Vec3::new(2.0, 0.5, 0.0)),
        None,
    );
    let left = s.add_node(
        Some(root),
        ComponentId("a/l".into()),
        "L",
        Mat4::from_translation(Vec3::new(-0.5, 0.0, 0.0)),
        Some(m),
    );
    let right = s.add_node(
        Some(root),
        ComponentId("a/r".into()),
        "R",
        Mat4::from_translation(Vec3::new(0.5, 0.0, 0.0)),
        Some(m),
    );
    s.refresh_pivots();
    let shared = |s: &Scene| {
        // Punto sul lato in comune (x locale 0 dell'assieme, sulla faccia superiore).
        let p = s.world_transform(root).transform_point3(Vec3::new(0.0, 0.5, 0.2));
        let (rl, ol) = s.texture_frame(left);
        let (rr, or) = s.texture_frame(right);
        let (fl, fr) = (
            BoxFrame {
                rot: rl,
                origin: ol,
                ..Default::default()
            },
            BoxFrame {
                rot: rr,
                origin: or,
                ..Default::default()
            },
        );
        (fl.to_local(p) - fr.to_local(p)).length()
    };
    assert!(shared(&s) < 1e-5);
    s.nodes[root].user = UserTransform {
        translation: Vec3::new(0.3, 0.0, -1.0),
        rotation_deg: Vec3::new(0.0, 33.0, 12.0),
        scale: Vec3::ONE,
    };
    assert!(shared(&s) < 1e-5, "assieme ruotato: {}", shared(&s));
    // Pezzo spostato da solo: la sua texture lo segue, quindi sul lato in comune non combacia più.
    s.nodes[right].user.translation = Vec3::new(0.0, 0.0, 0.1);
    assert!(shared(&s) > 0.05, "{}", shared(&s));
}

/// Pezzi dello stesso assieme ruotati diversamente nel file usano ognuno i propri assi: le
/// venature non proseguono dall'uno all'altro (modalità attuale, da documentare).
#[test]
fn assembly_parts_with_different_source_rotation_use_their_own_axes() {
    let mut s = Scene::default();
    let m = s.add_mesh(primitives::cuboid(Vec3::ONE));
    let root = s.add_node(None, ComponentId("a".into()), "Assieme", Mat4::IDENTITY, None);
    let a = s.add_node(Some(root), ComponentId("a/1".into()), "1", Mat4::IDENTITY, Some(m));
    let b = s.add_node(Some(root), ComponentId("a/2".into()), "2", Mat4::from_rotation_y(0.4), Some(m));
    s.refresh_pivots();
    let (ra, _) = s.texture_frame(a);
    let (rb, _) = s.texture_frame(b);
    assert!(ra.angle_between(rb) > 0.3);
}
