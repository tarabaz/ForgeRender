//! Camera ortografica: i raggi primari partono metri indietro (`Camera::ortho_back`). Il punto
//! colpito calcolato come origine + direzione · t aveva un errore più grande dello scostamento
//! anti-autointersezione, e i triangoli lunghi e sottili (i ventagli attorno ai fori dei
//! modelli CAD) si facevano ombra da soli: striature nel render isometrico.

use forge_math::{offset_ray_origin, Mat4, Vec3};
use forge_render_core::RenderScene;
use forge_scene::{Camera, ComponentId, Mesh, Scene};

/// Disco piano di raggio 4 cm triangolato a ventaglio da un punto del bordo (triangoli
/// sottilissimi), inclinato e lontano qualche centimetro dall'origine come un pezzo CAD.
fn sliver_disc() -> Scene {
    let n = 400;
    let mut positions = Vec::new();
    for i in 0..n {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        positions.push(Vec3::new(0.04 * a.cos(), 0.0, 0.04 * a.sin()));
    }
    let indices = (1..n as u32 - 1).map(|i| [0, i + 1, i]).collect();
    let mesh = Mesh {
        name: "disco".into(),
        normals: vec![Vec3::Y; n],
        uvs: Vec::new(),
        positions,
        indices,
    };
    let mut s = Scene::default();
    let m = s.add_mesh(mesh);
    let xf = Mat4::from_translation(Vec3::new(0.12, 0.05, -0.08)) * Mat4::from_rotation_x(0.4) * Mat4::from_rotation_z(0.3);
    s.add_node(None, ComponentId("disco".into()), "disco", xf, Some(m));
    s
}

/// Raggi della camera ortografica sul disco, poi un raggio d'ombra verso una luce dallo
/// stesso lato: il disco è piano, quindi ogni ombra trovata è un'auto-intersezione.
/// Restituisce (punti colpiti, auto-ombre con il punto baricentrico, auto-ombre con o + d·t).
fn self_shadows(cam: &Camera) -> (usize, usize, usize) {
    let rs = RenderScene::compile(&sliver_disc());
    let geo = &rs.geometry;
    let (mut hits, mut new_bad, mut old_bad) = (0, 0, 0);
    let res = 300;
    for y in 0..res {
        for x in 0..res {
            let ndc = |k: usize| (k as f32 + 0.5) / res as f32 * 2.0 - 1.0;
            let (o, d) = cam.ray(ndc(x), ndc(y), 1.0);
            let Some(h) = geo.accel.intersect(o, d, 0.0, f32::INFINITY) else {
                continue;
            };
            hits += 1;
            let prim = h.prim as usize;
            let mut ng = geo.geometric_normals[prim];
            if ng.dot(d) > 0.0 {
                ng = -ng;
            }
            // Luce radente dallo stesso lato della camera.
            let tangent = ng.cross(Vec3::X).normalize();
            let l = (ng * 0.15 + tangent).normalize();
            let occluded = |p: Vec3| geo.accel.occluded(offset_ray_origin(p, ng), l, 0.0, f32::INFINITY);
            new_bad += occluded(geo.surface_point(prim, h.u, h.v)) as usize;
            old_bad += occluded(o + d * h.t) as usize;
        }
    }
    (hits, new_bad, old_bad)
}

#[test]
fn orthographic_view_has_no_self_shadowing() {
    let mut cam = Camera {
        position: Vec3::new(1.2, 1.0, 1.4),
        target: Vec3::new(0.12, 0.05, -0.08),
        ..Default::default()
    };
    cam.lens.ortho = true;
    cam.lens.ortho_height = 0.12;
    let (hits, new_bad, old_bad) = self_shadows(&cam);
    eprintln!("ortografica: {hits} punti, auto-ombre {new_bad} (baricentrico) contro {old_bad} (o + d·t)");
    assert!(hits > 10_000, "la camera deve vedere il disco: {hits}");
    assert_eq!(new_bad, 0, "auto-ombre con il punto baricentrico");
    // Il calcolo di prima riproduce il difetto (misurato: 7627 su 16244).
    assert!(old_bad > hits / 10, "o + d·t: {old_bad} auto-ombre su {hits}");
}
