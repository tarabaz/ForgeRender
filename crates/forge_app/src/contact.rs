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
    /// movimento. Gli ostacoli nascosti non contano; il gruppo che si muove sì, anche se nascosto.
    /// Con `max_verts` i vertici di ogni lato vengono
    /// campionati a passo regolare (`usize::MAX` = tutti).
    pub fn new(scene: &Scene, moving: &[bool], obstacle: impl Fn(usize) -> bool, max_verts: usize) -> Contact {
        let (mut mt, mut ot) = (Vec::new(), Vec::new());
        let (mut mv, mut ov, mut oc) = (Vec::new(), Vec::new(), Vec::new());
        let moving_visible = (0..scene.nodes.len()).any(|i| moving[i] && scene.nodes[i].mesh.is_some() && scene.is_visible(i));
        for (i, n) in scene.nodes.iter().enumerate() {
            let Some(mi) = n.mesh else { continue };
            // Ostacoli: solo quelli visibili. Del gruppo che si muove contano i pezzi visibili; se è
            // tutto nascosto contano tutti (un pezzo nascosto scelto per "Appoggia" deve comunque
            // fermarsi sul suo appoggio).
            let usable = if moving[i] {
                !n.removed && (scene.is_visible(i) || !moving_visible)
            } else {
                obstacle(i) && scene.is_visible(i)
            };
            if !usable {
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

    /// Contatto tra i lati (spigolo contro spigolo) lungo `d`: due lati che si incrociano senza
    /// che un vertice tocchi una faccia (es. due cubi ruotati con gli spigoli incrociati). Nel
    /// piano perpendicolare al movimento si cercano gli incroci tra i lati del gruppo e quelli
    /// degli ostacoli; la distanza lungo `d` tra i due punti è lo spostamento possibile. Per non
    /// confrontare tutti con tutti, i lati degli ostacoli vanno in una griglia 2D.
    pub fn edge_hit(&self, d: Vec3) -> Option<f32> {
        use rayon::prelude::*;
        let len = d.length();
        if len < 1e-9 || !self.has_obstacles {
            return None;
        }
        let dir = d / len;
        let u = dir.any_orthonormal_vector();
        let w = dir.cross(u);
        // Lato proiettato: estremi 2D e profondità lungo `dir`.
        type Edge = ([f32; 2], [f32; 2], f32, f32);
        let edges = |tris: &[forge_accel::Triangle]| -> Vec<Edge> {
            let mut out = Vec::with_capacity(tris.len() * 3);
            for t in tris {
                let p = [t.v0, t.v0 + t.e1, t.v0 + t.e2];
                for k in 0..3 {
                    let (a, b) = (p[k], p[(k + 1) % 3]);
                    out.push(([a.dot(u), a.dot(w)], [b.dot(u), b.dot(w)], a.dot(dir), b.dot(dir)));
                }
            }
            out
        };
        let bounds = |e: &Edge| (e.0[0].min(e.1[0]), e.0[1].min(e.1[1]), e.0[0].max(e.1[0]), e.0[1].max(e.1[1]));
        let moving = edges(&self.moving.tris);
        let obstacles = edges(&self.obstacles.tris);
        if moving.is_empty() || obstacles.is_empty() {
            return None;
        }
        // Zona utile: dove le proiezioni si sovrappongono; davanti al gruppo lungo `dir`.
        let extent = |es: &[Edge]| {
            es.iter().fold((f32::MAX, f32::MAX, f32::MIN, f32::MIN), |b, e| {
                let (x0, y0, x1, y1) = bounds(e);
                (b.0.min(x0), b.1.min(y0), b.2.max(x1), b.3.max(y1))
            })
        };
        let (m, o) = (extent(&moving), extent(&obstacles));
        let zone = (m.0.max(o.0), m.1.max(o.1), m.2.min(o.2), m.3.min(o.3));
        if zone.0 > zone.2 || zone.1 > zone.3 {
            return None;
        }
        let min_depth = moving.iter().map(|e| e.2.min(e.3)).fold(f32::MAX, f32::min);
        let max_depth = moving.iter().map(|e| e.2.max(e.3)).fold(f32::MIN, f32::max);
        let inside = |e: &Edge| {
            let (x0, y0, x1, y1) = bounds(e);
            x1 >= zone.0 && x0 <= zone.2 && y1 >= zone.1 && y0 <= zone.3
        };
        let obstacles: Vec<Edge> = obstacles
            .into_iter()
            .filter(|e| inside(e) && e.2.max(e.3) >= min_depth - CONTACT_GAP && e.2.min(e.3) <= max_depth + len + CONTACT_GAP)
            .collect();
        let moving: Vec<Edge> = moving.into_iter().filter(inside).collect();
        if moving.is_empty() || obstacles.is_empty() {
            return None;
        }
        // Griglia: lato della cella dalla lunghezza media dei lati, al massimo 1024 celle per asse.
        let mean = |es: &[Edge]| {
            es.iter()
                .map(|e| ((e.1[0] - e.0[0]).powi(2) + (e.1[1] - e.0[1]).powi(2)).sqrt())
                .sum::<f32>()
                / es.len() as f32
        };
        let span = (zone.2 - zone.0).max(zone.3 - zone.1).max(1e-6);
        let cell = mean(&obstacles).max(mean(&moving)).max(span / 1024.0).max(1e-6);
        let nx = (((zone.2 - zone.0) / cell).ceil() as usize).clamp(1, 1024);
        let ny = (((zone.3 - zone.1) / cell).ceil() as usize).clamp(1, 1024);
        let cell_of = |x: f32, y: f32| {
            let cx = (((x - zone.0) / cell).floor().max(0.0) as usize).min(nx - 1);
            let cy = (((y - zone.1) / cell).floor().max(0.0) as usize).min(ny - 1);
            (cx, cy)
        };
        // Lati grandi (molte celle, es. pavimento): a parte, si provano con tutti.
        let mut grid: Vec<Vec<u32>> = vec![Vec::new(); nx * ny];
        let mut big: Vec<u32> = Vec::new();
        for (k, e) in obstacles.iter().enumerate() {
            let (x0, y0, x1, y1) = bounds(e);
            let (a, b) = (cell_of(x0, y0), cell_of(x1, y1));
            if (b.0 - a.0 + 1) * (b.1 - a.1 + 1) > 64 {
                big.push(k as u32);
                continue;
            }
            for cy in a.1..=b.1 {
                for cx in a.0..=b.0 {
                    grid[cy * nx + cx].push(k as u32);
                }
            }
        }
        // Incrocio nel piano: distanza lungo `dir` dal lato del gruppo a quello dell'ostacolo.
        let cross2 = |a: [f32; 2], b: [f32; 2]| a[0] * b[1] - a[1] * b[0];
        let travel = |m: &Edge, o: &Edge| -> Option<f32> {
            let r = [m.1[0] - m.0[0], m.1[1] - m.0[1]];
            let s = [o.1[0] - o.0[0], o.1[1] - o.0[1]];
            let den = cross2(r, s);
            if den.abs() <= 1e-12 * (r[0].abs() + r[1].abs() + 1e-9) * (s[0].abs() + s[1].abs() + 1e-9) {
                return None; // paralleli: li coprono i raggi dai vertici
            }
            let q = [o.0[0] - m.0[0], o.0[1] - m.0[1]];
            let (t, v) = (cross2(q, s) / den, cross2(q, r) / den);
            if !(0.0..=1.0).contains(&t) || !(0.0..=1.0).contains(&v) {
                return None;
            }
            let dm = m.2 + (m.3 - m.2) * t;
            let dob = o.2 + (o.3 - o.2) * v;
            let dist = dob - dm;
            (dist >= -CONTACT_GAP && dist <= len + CONTACT_GAP).then_some(dist)
        };
        let best = moving
            .par_iter()
            .map(|m| {
                let mut best = f32::INFINITY;
                let mut test = |k: u32| {
                    if let Some(t) = travel(m, &obstacles[k as usize]) {
                        best = best.min(t);
                    }
                };
                for &k in &big {
                    test(k);
                }
                let (x0, y0, x1, y1) = bounds(m);
                let (a, b) = (cell_of(x0, y0), cell_of(x1, y1));
                if (b.0 - a.0 + 1) * (b.1 - a.1 + 1) > 4096 {
                    (0..obstacles.len() as u32).for_each(&mut test);
                } else {
                    for cy in a.1..=b.1 {
                        for cx in a.0..=b.0 {
                            for &k in &grid[cy * nx + cx] {
                                test(k);
                            }
                        }
                    }
                }
                best
            })
            .reduce(|| f32::INFINITY, f32::min);
        best.is_finite().then(|| (best - CONTACT_GAP).clamp(0.0, len))
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

/// "Appoggia": ogni componente scelto scende finché tocca l'oggetto che ha subito sotto
/// (qualsiasi componente visibile: un altro pezzo, il fondale, il pavimento), come lo snap a
/// contatto verso il basso; con niente sotto si ferma a quota 0. Con più componenti scelti
/// ognuno scende per conto suo, dal più basso al più alto, e quelli già appoggiati fanno da
/// appoggio agli altri (pile comprese). I componenti dentro un assieme anch'esso scelto si
/// muovono con lui. Il contatto considera vertici contro facce e spigolo contro spigolo.
/// Restituisce lo spostamento verticale più grande applicato.
pub fn drop_onto_below(scene: &mut Scene, nodes: &[usize]) -> f32 {
    let mut roots: Vec<(usize, forge_math::Aabb)> = nodes
        .iter()
        .copied()
        .filter(|&i| {
            let mut p = scene.nodes[i].parent;
            while let Some(k) = p {
                if nodes.contains(&k) {
                    return false;
                }
                p = scene.nodes[k].parent;
            }
            true
        })
        .map(|i| (i, scene.exact_bounds_of(i)))
        .filter(|(_, b)| !b.is_empty())
        .collect();
    roots.sort_by(|a, b| a.1.min.y.total_cmp(&b.1.min.y));
    let mut largest = 0.0f32;
    for (r, _) in roots {
        // Posizione attuale: i pezzi già appoggiati possono averlo spostato (stesso assieme).
        let b = scene.exact_bounds_of(r);
        let moving = subtree_mask(scene, &[r]);
        let contact = Contact::new(scene, &moving, |i| !moving[i], usize::MAX);
        let lowest = scene.bounds().min.y.min(0.0);
        let d = Vec3::new(0.0, -(b.min.y - lowest + 1.0).max(1e-3), 0.0);
        let hit = match (contact.hit(d), contact.edge_hit(d)) {
            (Some(a), Some(e)) => Some(a.min(e)),
            (a, e) => a.or(e),
        };
        let dy = match hit {
            Some(t) => -t,
            None => -b.min.y,
        };
        if dy.abs() < 1e-7 {
            continue;
        }
        scene.translate_world(r, Vec3::new(0.0, dy, 0.0));
        if dy.abs() > largest.abs() {
            largest = dy;
        }
    }
    largest
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_math::Mat4;
    use forge_scene::{primitives, ComponentId};

    /// "Appoggia" si ferma sull'oggetto subito sotto (il cubo sopra l'altro cubo), sul
    /// pavimento se sotto c'è solo quello, e a quota 0 se sotto non c'è niente.
    #[test]
    fn drop_lands_on_the_object_below() {
        let mut s = Scene::default();
        let cube = s.add_mesh(primitives::cuboid(Vec3::ONE));
        let floor = s.add_mesh(primitives::plane(10.0, 10.0));
        let raised = Mat4::from_translation(Vec3::new(0.0, 0.2, 0.0));
        s.add_node(None, ComponentId("pavimento".into()), "Pavimento", raised, Some(floor));
        let a = s.add_node(
            None,
            ComponentId("a".into()),
            "A",
            Mat4::from_translation(Vec3::new(0.0, 0.7, 0.0)),
            Some(cube),
        );
        let b = s.add_node(
            None,
            ComponentId("b".into()),
            "B",
            Mat4::from_translation(Vec3::new(0.3, 3.0, 0.2)),
            Some(cube),
        );
        let c = s.add_node(
            None,
            ComponentId("c".into()),
            "C",
            Mat4::from_translation(Vec3::new(3.0, 2.0, 0.0)),
            Some(cube),
        );
        let far = s.add_node(
            None,
            ComponentId("d".into()),
            "D",
            Mat4::from_translation(Vec3::new(20.0, 2.0, 0.0)),
            Some(cube),
        );
        s.refresh_pivots();
        drop_onto_below(&mut s, &[b]);
        let top_a = s.exact_bounds_of(a).max.y;
        assert!(
            (s.exact_bounds_of(b).min.y - top_a).abs() < 1e-3,
            "B su A: {}",
            s.exact_bounds_of(b).min.y
        );
        drop_onto_below(&mut s, &[c]);
        assert!(
            (s.exact_bounds_of(c).min.y - 0.2).abs() < 1e-3,
            "C sul pavimento: {}",
            s.exact_bounds_of(c).min.y
        );
        drop_onto_below(&mut s, &[far]);
        assert!(s.exact_bounds_of(far).min.y.abs() < 1e-5, "D a quota 0");
    }

    /// Scena con pavimento a quota 0 e cubi da 1 m nei punti dati (centri).
    fn floor_and_cubes(centers: &[(Vec3, Mat4)]) -> (Scene, Vec<usize>) {
        let mut s = Scene::default();
        let cube = s.add_mesh(primitives::cuboid(Vec3::ONE));
        let floor = s.add_mesh(primitives::plane(20.0, 20.0));
        s.add_node(None, ComponentId("pavimento".into()), "Pavimento", Mat4::IDENTITY, Some(floor));
        let ids = centers
            .iter()
            .enumerate()
            .map(|(k, (c, r))| {
                let xf = Mat4::from_translation(*c) * *r;
                s.add_node(None, ComponentId(format!("c{k}")), "C", xf, Some(cube))
            })
            .collect();
        s.refresh_pivots();
        (s, ids)
    }

    /// Selezione multipla di oggetti separati: ognuno deve scendere fino al proprio appoggio.
    /// Oggi si muovono come un blocco unico: il primo che tocca ferma anche gli altri.
    #[test]
    fn multi_selection_drops_each_object() {
        let (mut s, ids) = floor_and_cubes(&[
            (Vec3::new(-3.0, 1.0, 0.0), Mat4::IDENTITY),
            (Vec3::new(3.0, 4.0, 0.0), Mat4::IDENTITY),
        ]);
        drop_onto_below(&mut s, &ids);
        for &i in &ids {
            let y = s.exact_bounds_of(i).min.y;
            assert!(y.abs() < 1e-3, "cubo {i} a quota {y}");
        }
    }

    /// Spigolo su spigolo (cubi ruotati di 45° con gli spigoli incrociati): il contatto è tra
    /// due lati, non tra un vertice e una faccia. Il cubo sopra deve fermarsi sullo spigolo
    /// di quello sotto senza compenetrarlo.
    #[test]
    fn edge_on_edge_contact_does_not_penetrate() {
        let q = std::f32::consts::FRAC_PI_4;
        let (mut s, ids) = floor_and_cubes(&[
            (Vec3::new(0.0, 0.7072, 0.0), Mat4::from_rotation_x(q)),
            (Vec3::new(0.0, 4.0, 0.0), Mat4::from_rotation_z(q)),
        ]);
        drop_onto_below(&mut s, &[ids[1]]);
        let top_low = s.exact_bounds_of(ids[0]).max.y;
        let low_high = s.exact_bounds_of(ids[1]).min.y;
        eprintln!("cresta sotto {top_low:.4}, spigolo sopra {low_high:.4}");
        assert!(low_high > top_low - 1e-3, "compenetrazione di {:.4} m", top_low - low_high);
    }

    /// Oggetto nascosto scelto: oggi non ha vertici per il contatto e va a quota 0 anche se
    /// sotto c'è un appoggio. Deve comportarsi come se fosse visibile (o non muoversi).
    #[test]
    fn hidden_selection_still_lands_on_support() {
        let (mut s, ids) = floor_and_cubes(&[
            (Vec3::new(0.0, 0.5, 0.0), Mat4::IDENTITY),
            (Vec3::new(0.0, 4.0, 0.0), Mat4::IDENTITY),
        ]);
        s.nodes[ids[1]].visible = false;
        drop_onto_below(&mut s, &[ids[1]]);
        let y = s.exact_bounds_of(ids[1]).min.y;
        assert!((y - 1.0).abs() < 1e-3, "quota {y}");
    }

    /// Superficie concava: una sfera dentro una tazza si ferma sul fondo interno; un
    /// oggetto sottile (piano senza spessore) sopra un cubo più piccolo si ferma sul cubo;
    /// assieme e figlio scelti insieme si muovono una volta sola; oggetto ruotato.
    #[test]
    fn concave_thin_hierarchy_and_rotated_cases() {
        // Tazza (fondo interno a quota `wall`) e sfera sopra il centro.
        let mut s = Scene::default();
        let wall = 0.01;
        let cup = s.add_mesh(primitives::cup(0.2, 0.3, wall, 64));
        let ball = s.add_mesh(primitives::uv_sphere(0.05, 32, 16));
        s.add_node(None, ComponentId("tazza".into()), "Tazza", Mat4::IDENTITY, Some(cup));
        let b = s.add_node(
            None,
            ComponentId("palla".into()),
            "Palla",
            Mat4::from_translation(Vec3::new(0.0, 1.0, 0.0)),
            Some(ball),
        );
        s.refresh_pivots();
        drop_onto_below(&mut s, &[b]);
        let y = s.exact_bounds_of(b).min.y;
        let inner_floor = s.exact_bounds_of(0).min.y + wall;
        assert!(
            y > inner_floor - 2e-3 && y < inner_floor + 0.02,
            "palla a {y}, fondo interno {inner_floor}"
        );

        // Piano sottile 2 × 2 m sopra un cubo da 0,5 m: tocca la faccia superiore del cubo.
        let mut s = Scene::default();
        let small = s.add_mesh(primitives::cuboid(Vec3::splat(0.5)));
        let sheet = s.add_mesh(primitives::plane(2.0, 2.0));
        let c = s.add_node(
            None,
            ComponentId("cubo".into()),
            "Cubo",
            Mat4::from_translation(Vec3::new(0.0, 0.25, 0.0)),
            Some(small),
        );
        let p = s.add_node(
            None,
            ComponentId("foglio".into()),
            "Foglio",
            Mat4::from_translation(Vec3::new(0.0, 2.0, 0.0)),
            Some(sheet),
        );
        s.refresh_pivots();
        drop_onto_below(&mut s, &[p]);
        let top = s.exact_bounds_of(c).max.y;
        assert!((s.exact_bounds_of(p).min.y - top).abs() < 1e-3);

        // Assieme e suo figlio scelti insieme: lo spostamento si applica una volta sola.
        let mut s = Scene::default();
        let cube = s.add_mesh(primitives::cuboid(Vec3::ONE));
        let asm = s.add_node(
            None,
            ComponentId("a".into()),
            "A",
            Mat4::from_translation(Vec3::new(0.0, 3.0, 0.0)),
            None,
        );
        let child = s.add_node(Some(asm), ComponentId("a/b".into()), "B", Mat4::IDENTITY, Some(cube));
        s.refresh_pivots();
        drop_onto_below(&mut s, &[asm, child]);
        assert!(s.exact_bounds_of(child).min.y.abs() < 1e-5);

        // Cubo ruotato su un vertice: il vertice più basso tocca il cubo sotto.
        let (mut s, ids) = floor_and_cubes(&[
            (Vec3::new(0.0, 0.5, 0.0), Mat4::IDENTITY),
            (
                Vec3::new(0.1, 4.0, 0.05),
                Mat4::from_quat(forge_math::Quat::from_euler(forge_math::glam::EulerRot::XYZ, 0.6, 0.3, 0.8)),
            ),
        ]);
        drop_onto_below(&mut s, &[ids[1]]);
        assert!(
            (s.exact_bounds_of(ids[1]).min.y - 1.0).abs() < 1e-3,
            "{}",
            s.exact_bounds_of(ids[1]).min.y
        );
    }

    /// Annulla: il pannello registra lo stato dei componenti scelti prima di "Appoggia" e
    /// Ctrl+Z rimette quelli. Va bene solo se l'operazione non tocca altro: qui, con assieme e
    /// figlio scelti, pile, pezzi nascosti e non scelti nella scena, cambiano solo i nodi scelti,
    /// e rimettendo il loro stato la scena torna identica (un solo Ctrl+Z).
    #[test]
    fn drop_changes_only_the_selection_so_one_undo_restores_it() {
        let (mut s, ids) = floor_and_cubes(&[
            (Vec3::new(0.0, 0.5, 0.0), Mat4::IDENTITY),
            (Vec3::new(0.2, 3.0, 0.0), Mat4::from_rotation_y(0.3)),
            (Vec3::new(3.0, 2.0, 0.0), Mat4::IDENTITY),
            (Vec3::new(-3.0, 4.0, 0.0), Mat4::IDENTITY),
        ]);
        s.nodes[ids[3]].visible = false;
        let cube = s.nodes[ids[0]].mesh.unwrap();
        let asm = s.add_node(
            None,
            ComponentId("asm".into()),
            "A",
            Mat4::from_translation(Vec3::new(6.0, 5.0, 0.0)),
            None,
        );
        let child = s.add_node(Some(asm), ComponentId("asm/b".into()), "B", Mat4::IDENTITY, Some(cube));
        s.refresh_pivots();
        let selection = [ids[1], ids[2], ids[3], asm, child];
        let before: Vec<_> = s.nodes.iter().map(|n| n.user).collect();
        drop_onto_below(&mut s, &selection);
        for (i, n) in s.nodes.iter().enumerate() {
            if !selection.contains(&i) {
                assert_eq!(n.user, before[i], "nodo {i} non scelto modificato");
            }
        }
        assert!(selection.iter().any(|&i| s.nodes[i].user != before[i]), "niente si è mosso");
        // Annulla: si rimette lo stato registrato dei nodi scelti.
        for &i in &selection {
            s.nodes[i].user = before[i];
        }
        assert!(s.nodes.iter().zip(&before).all(|(n, b)| n.user == *b));
    }

    /// Costo su una mesh densa (sfera da circa 1,2 milioni di triangoli sopra un pavimento).
    #[test]
    #[ignore = "misura di prestazioni (cargo test -- --ignored --nocapture)"]
    fn dense_mesh_drop_timing() {
        let mut s = Scene::default();
        let floor = s.add_mesh(primitives::plane(20.0, 20.0));
        let ball = s.add_mesh(primitives::uv_sphere(0.5, 1100, 550));
        s.add_node(None, ComponentId("pavimento".into()), "Pavimento", Mat4::IDENTITY, Some(floor));
        let b = s.add_node(
            None,
            ComponentId("palla".into()),
            "Palla",
            Mat4::from_translation(Vec3::new(0.0, 3.0, 0.0)),
            Some(ball),
        );
        s.refresh_pivots();
        let t = std::time::Instant::now();
        drop_onto_below(&mut s, &[b]);
        eprintln!(
            "{} triangoli: Appoggia in {:.0} ms",
            s.triangle_count(),
            t.elapsed().as_secs_f64() * 1e3
        );
        assert!(s.exact_bounds_of(b).min.y.abs() < 1e-3);
    }
}
