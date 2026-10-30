//! Contatto tra un gruppo di componenti che si muove e il resto della scena: di quanto si
//! può spostare il gruppo in una direzione prima di toccare qualcosa. Lo usano lo snap a
//! contatto del gizmo e "Appoggia sulla base" (il prodotto scende fino al fondale).

use forge_math::Vec3;
use forge_scene::Scene;

/// Gruppo in movimento e ostacoli, in spazio mondo all'inizio dello spostamento: gli ostacoli
/// non si muovono, quindi le loro BVH restano valide per tutto il trascinamento.
pub struct Contact {
    /// Vertici dei componenti spostati.
    verts: Vec<Vec3>,
    /// Centro dei componenti spostati (per capire da che lato sta l'oggetto a contatto).
    moving_center: Vec3,
    /// Centro dell'ostacolo a cui appartiene ogni vertice di `obstacle_verts`.
    obstacle_centers: Vec<Vec3>,
    moving: forge_accel::TriangleAccel,
    moving_n: Vec<Vec3>,
    obstacles: forge_accel::TriangleAccel,
    obstacle_n: Vec<Vec3>,
    obstacle_verts: Vec<Vec3>,
    has_obstacles: bool,
}

/// Distanza di sicurezza lasciata tra le superfici a contatto (m).
pub const CONTACT_GAP: f32 = 2e-4;
/// Massimo di vertici usati per lato durante il trascinamento (oltre si campiona a passo regolare).
pub const DRAG_MAX_VERTS: usize = 40_000;

impl Contact {
    /// `moving[i]`: il nodo `i` si muove; `obstacle(i)`: il nodo `i` (fermo) può fermare il
    /// movimento. Si usano solo i nodi visibili. Con `max_verts` i vertici di ogni lato vengono
    /// campionati a passo regolare (`usize::MAX` = tutti).
    pub fn new(scene: &Scene, moving: &[bool], obstacle: impl Fn(usize) -> bool, max_verts: usize) -> Contact {
        let (mut mt, mut ot) = (Vec::new(), Vec::new());
        let (mut mv, mut ov, mut oc) = (Vec::new(), Vec::new(), Vec::new());
        for (i, n) in scene.nodes.iter().enumerate() {
            let Some(mi) = n.mesh else { continue };
            if !scene.is_visible(i) || !(moving[i] || obstacle(i)) {
                continue;
            }
            let m = &scene.meshes[mi];
            let xf = scene.world_transform(i);
            let wp: Vec<Vec3> = m.positions.iter().map(|p| xf.transform_point3(*p)).collect();
            let tris = if moving[i] { &mut mt } else { &mut ot };
            for idx in &m.indices {
                tris.push(idx.map(|k| wp[k as usize]));
            }
            if moving[i] {
                mv.extend(wp);
            } else {
                let c = forge_math::Aabb::from_points(&wp).center();
                oc.extend(std::iter::repeat_n(c, wp.len()));
                ov.extend(wp);
            }
        }
        let step = |n: usize| n.div_ceil(max_verts.max(1)).max(1);
        let (sm, so) = (step(mv.len()), step(ov.len()));
        let moving_center = forge_math::Aabb::from_points(&mv).center();
        let normals = |t: &[[Vec3; 3]]| t.iter().map(|t| (t[1] - t[0]).cross(t[2] - t[0]).normalize_or_zero()).collect();
        Contact {
            verts: mv.into_iter().step_by(sm).collect(),
            moving_center,
            moving_n: normals(&mt),
            moving: forge_accel::TriangleAccel::build(&mt),
            obstacle_n: normals(&ot),
            has_obstacles: !ot.is_empty(),
            obstacles: forge_accel::TriangleAccel::build(&ot),
            obstacle_verts: ov.into_iter().step_by(so).collect(),
            obstacle_centers: oc.into_iter().step_by(so).collect(),
        }
    }

    /// Distanza lungo `d` a cui il gruppo tocca un ostacolo, se lo tocca entro |d|.
    pub fn hit(&self, d: Vec3) -> Option<f32> {
        use rayon::prelude::*;
        let len = d.length();
        if len < 1e-9 || !self.has_obstacles {
            return None;
        }
        let dir = d / len;
        // Raggi da ogni punto lungo `dir`; i contatti già in essere (t ≈ 0) fermano solo se
        // il movimento entra nell'ostacolo (dal lato del proprio centro verso l'altro).
        let cast =
            |accel: &forge_accel::TriangleAccel, normals: &[Vec3], from: &[Vec3], centers: &(dyn Fn(usize) -> Vec3 + Sync), dir: Vec3| {
                from.par_iter()
                    .enumerate()
                    .filter_map(|(k, v)| {
                        let center = centers(k);
                        accel
                            .intersect_filtered(*v, dir, -CONTACT_GAP, len + CONTACT_GAP, |prim, t, _, _| {
                                let n = normals[prim as usize];
                                let along = n.dot(dir);
                                if along.abs() < 0.05 {
                                    return false; // superficie parallela al movimento: si scivola
                                }
                                if t > CONTACT_GAP {
                                    return true;
                                }
                                let side = n.dot(center - *v);
                                side != 0.0 && along * side.signum() < 0.0
                            })
                            .map(|h| h.t)
                    })
                    .reduce(|| f32::INFINITY, f32::min)
            };
        // Vertici dell'oggetto contro gli ostacoli, e vertici degli ostacoli contro l'oggetto.
        let mc = self.moving_center;
        let t = cast(&self.obstacles, &self.obstacle_n, &self.verts, &|_| mc, dir).min(cast(
            &self.moving,
            &self.moving_n,
            &self.obstacle_verts,
            &|k| self.obstacle_centers[k],
            -dir,
        ));
        t.is_finite().then(|| (t - CONTACT_GAP).clamp(0.0, len))
    }

    /// Lunghezza ammessa dello spostamento `d` (≤ |d|) prima di toccare un ostacolo.
    /// Le superfici quasi parallele al movimento non fermano (si può scivolare su un piano).
    pub fn limit(&self, d: Vec3) -> f32 {
        self.hit(d).unwrap_or(d.length())
    }
}

/// Nodi del sotto-albero delle radici date (con le radici).
pub fn subtree_mask(scene: &Scene, roots: &[usize]) -> Vec<bool> {
    let mut mask = vec![false; scene.nodes.len()];
    let mut stack: Vec<usize> = roots.to_vec();
    while let Some(i) = stack.pop() {
        mask[i] = true;
        stack.extend(scene.nodes[i].children.iter().copied());
    }
    mask
}
