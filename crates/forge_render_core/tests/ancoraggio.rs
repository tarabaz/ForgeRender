//! Ancoraggio della proiezione Box (0.17.0): pezzo, assieme e mondo su un assieme vero, con
//! un pezzo ruotato nel file, uno spostato da solo e l'assieme ruotato e spostato.

use forge_math::{Mat4, Vec3};
use forge_render_core::displace::BoxFrame;
use forge_render_core::RenderScene;
use forge_scene::{primitives, ComponentId, Material, MaterialId, Scene, TextureAnchor, UserTransform};

struct Asm {
    s: Scene,
    root: usize,
    left: usize,
    right: usize,
    turned: usize,
}

/// Assieme: due cubi accostati orientati come l'assieme (sinistro e destro) e un terzo ruotato
/// di 30° nel file; tutti con lo stesso materiale, ancorato come indicato.
fn assembly(anchor: TextureAnchor) -> Asm {
    let mut s = Scene::default();
    s.materials.upsert(Material {
        id: MaterialId::new("legno"),
        mapping: forge_scene::TextureMapping {
            anchor,
            ..Default::default()
        },
        ..Default::default()
    });
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
    let turned = s.add_node(
        Some(root),
        ComponentId("a/t".into()),
        "T",
        Mat4::from_translation(Vec3::new(0.0, 0.0, 1.5)) * Mat4::from_rotation_y(0.52),
        Some(m),
    );
    for id in ["a/l", "a/r", "a/t"] {
        s.assignments.insert(ComponentId(id.into()), MaterialId::new("legno"));
    }
    s.refresh_pivots();
    Asm {
        s,
        root,
        left,
        right,
        turned,
    }
}

fn frames(s: &Scene) -> Vec<BoxFrame> {
    RenderScene::compile(s).node_frames
}

/// Punto del pezzo `i` (coordinate del suo cubo) nel mondo.
fn on_part(s: &Scene, i: usize, local: Vec3) -> Vec3 {
    s.world_transform(i).transform_point3(local)
}

/// Il punto sul lato in comune tra i due cubi accostati, visto dai due pezzi.
fn shared_gap(a: &Asm) -> f32 {
    let f = frames(&a.s);
    let p = on_part(&a.s, a.left, Vec3::new(0.5, 0.5, 0.2));
    (f[a.left].to_local(p) - f[a.right].to_local(p)).length()
}

/// Lo stesso punto del pezzo ruotato visto con il riferimento del pezzo ruotato e con quello
/// del pezzo sinistro: uguale se i pezzi condividono la proiezione.
fn turned_gap(a: &Asm) -> f32 {
    let f = frames(&a.s);
    let p = on_part(&a.s, a.turned, Vec3::new(0.3, 0.5, -0.1));
    (f[a.turned].to_local(p) - f[a.left].to_local(p)).length()
}

/// Coordinate di un punto della superficie del pezzo `i` (quanto "scorre" la texture se cambia).
fn coords_of(a: &Asm, i: usize) -> Vec3 {
    let f = frames(&a.s);
    f[i].to_local(on_part(&a.s, i, Vec3::new(0.2, 0.5, 0.1)))
}

fn move_root(a: &mut Asm) {
    a.s.nodes[a.root].user = UserTransform {
        translation: Vec3::new(0.4, 0.0, -0.7),
        rotation_deg: Vec3::new(0.0, 40.0, 10.0),
        scale: Vec3::ONE,
    };
}

fn move_right_alone(a: &mut Asm) {
    a.s.nodes[a.right].user.translation = Vec3::new(0.0, 0.0, 0.15);
}

#[test]
fn component_anchor_follows_each_part() {
    let mut a = assembly(TextureAnchor::Component);
    assert!(shared_gap(&a) < 1e-5, "pezzi accostati continui");
    assert!(turned_gap(&a) > 0.05, "il pezzo ruotato ha i propri assi");
    let before = [coords_of(&a, a.left), coords_of(&a, a.right)];
    move_root(&mut a);
    assert!(shared_gap(&a) < 1e-5);
    assert!((coords_of(&a, a.left) - before[0]).length() < 1e-5, "segue l'assieme");
    move_right_alone(&mut a);
    assert!(
        (coords_of(&a, a.right) - before[1]).length() < 1e-5,
        "segue il pezzo spostato da solo"
    );
    assert!(shared_gap(&a) > 0.05, "e quindi sul lato in comune non combacia più");
}

#[test]
fn assembly_anchor_is_shared_by_all_parts() {
    let mut a = assembly(TextureAnchor::Assembly);
    assert!(shared_gap(&a) < 1e-5);
    assert!(turned_gap(&a) < 1e-5, "anche il pezzo ruotato nel file condivide la proiezione");
    let before = [coords_of(&a, a.left), coords_of(&a, a.right)];
    move_root(&mut a);
    assert!(shared_gap(&a) < 1e-5 && turned_gap(&a) < 1e-5);
    assert!((coords_of(&a, a.left) - before[0]).length() < 1e-5, "segue l'assieme");
    move_right_alone(&mut a);
    assert!(shared_gap(&a) < 1e-5, "la proiezione resta unica");
    assert!(
        (coords_of(&a, a.right) - before[1]).length() > 0.1,
        "sul pezzo spostato da solo scorre"
    );
}

#[test]
fn world_anchor_stays_in_the_scene() {
    let mut a = assembly(TextureAnchor::World);
    let f = frames(&a.s);
    assert!(f.iter().all(|fr| *fr == BoxFrame::default()));
    assert!(shared_gap(&a) < 1e-5 && turned_gap(&a) < 1e-5);
    let p = on_part(&a.s, a.left, Vec3::new(0.2, 0.5, 0.1));
    assert!((coords_of(&a, a.left) - p).length() < 1e-6, "coordinate = posizione nel mondo");
    let before = coords_of(&a, a.left);
    move_root(&mut a);
    assert!((coords_of(&a, a.left) - before).length() > 0.1, "l'oggetto scorre sulla texture");
}

/// "Adatta all'oggetto": la ripetizione segue la dimensione del pezzo. Una sfera da 5 cm e una
/// da 50 cm con lo stesso materiale hanno le stesse coordinate texture nei punti corrispondenti
/// (la texture appare uguale); in metri, invece, la grande ne mostra dieci volte di più.
#[test]
fn fit_object_follows_the_piece_size() {
    let coords = |fit: bool| {
        let mut s = Scene::default();
        s.materials.upsert(Material {
            id: MaterialId::new("m"),
            mapping: forge_scene::TextureMapping {
                fit_object: fit,
                scale: 1.0,
                ..Default::default()
            },
            ..Default::default()
        });
        let small = s.add_mesh(primitives::uv_sphere(0.025, 32, 16));
        let big = s.add_mesh(primitives::uv_sphere(0.25, 32, 16));
        let a = s.add_node(None, ComponentId("piccola".into()), "P", Mat4::IDENTITY, Some(small));
        let b = s.add_node(None, ComponentId("grande".into()), "G", Mat4::from_translation(Vec3::X), Some(big));
        for id in ["piccola", "grande"] {
            s.assignments.insert(ComponentId(id.into()), MaterialId::new("m"));
        }
        s.refresh_pivots();
        let f = frames(&s);
        let dir = Vec3::new(0.3, 0.8, 0.5).normalize();
        let pa = s.world_transform(a).transform_point3(dir * 0.025);
        let pb = s.world_transform(b).transform_point3(dir * 0.25);
        (f[a].to_local(pa), f[b].to_local(pb))
    };
    let (a, b) = coords(true);
    assert!((a - b).length() < 1e-4, "adattata: {a} vs {b}");
    let (a, b) = coords(false);
    assert!((b - a * 10.0).length() < 1e-4, "in metri: {a} vs {b}");
}
