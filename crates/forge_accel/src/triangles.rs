use crate::bvh::Bvh;
use forge_math::{Aabb, Vec3};

/// Triangolo precomputato per Möller–Trumbore (vertice + due spigoli).
#[derive(Clone, Copy, Debug)]
pub struct Triangle {
    pub v0: Vec3,
    pub e1: Vec3,
    pub e2: Vec3,
}

impl Triangle {
    pub fn new(a: Vec3, b: Vec3, c: Vec3) -> Self {
        Self {
            v0: a,
            e1: b - a,
            e2: c - a,
        }
    }

    /// Intersezione raggio-triangolo (Möller–Trumbore, doppia faccia).
    /// Restituisce `(t, u, v)` con coordinate baricentriche.
    #[inline(always)]
    pub fn intersect(&self, o: Vec3, d: Vec3, t_min: f32, t_max: f32) -> Option<(f32, f32, f32)> {
        let p = d.cross(self.e2);
        let det = self.e1.dot(p);
        if det.abs() < 1e-20 {
            return None;
        }
        let inv = 1.0 / det;
        let s = o - self.v0;
        let u = s.dot(p) * inv;
        if !(0.0..=1.0).contains(&u) {
            return None;
        }
        let q = s.cross(self.e1);
        let v = d.dot(q) * inv;
        if v < 0.0 || u + v > 1.0 {
            return None;
        }
        let t = self.e2.dot(q) * inv;
        if t > t_min && t < t_max {
            Some((t, u, v))
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TriangleHit {
    pub t: f32,
    /// Indice del triangolo nell'ordine originale passato a [`TriangleAccel::build`].
    pub prim: u32,
    pub u: f32,
    pub v: f32,
}

/// BVH su triangoli, con i triangoli riordinati nell'ordine delle foglie per
/// la località di cache.
#[derive(Clone, Debug, Default)]
pub struct TriangleAccel {
    pub bvh: Bvh,
    pub tris: Vec<Triangle>,
}

impl TriangleAccel {
    pub fn build(tris: &[[Vec3; 3]]) -> Self {
        let t0 = std::time::Instant::now();
        let bounds: Vec<Aabb> = tris.iter().map(|t| Aabb::from_points(t.iter())).collect();
        let centroids: Vec<Vec3> = bounds.iter().map(|b| b.center()).collect();
        let bvh = Bvh::build(&bounds, &centroids);
        let ordered = bvh
            .prim_indices
            .iter()
            .map(|&i| {
                let [a, b, c] = tris[i as usize];
                Triangle::new(a, b, c)
            })
            .collect();
        tracing::debug!(
            "BVH: {} triangoli, {} nodi in {:.1} ms",
            tris.len(),
            bvh.nodes.len(),
            t0.elapsed().as_secs_f64() * 1e3
        );
        Self { bvh, tris: ordered }
    }

    pub fn is_empty(&self) -> bool {
        self.tris.is_empty()
    }

    /// Intersezione più vicina.
    pub fn intersect(&self, o: Vec3, d: Vec3, t_min: f32, t_max: f32) -> Option<TriangleHit> {
        if self.tris.is_empty() {
            return None;
        }
        let mut best: Option<(f32, u32, f32, f32)> = None;
        self.bvh.traverse(o, d, t_min, t_max, |first, count, tmax| {
            for i in first..first + count {
                if let Some((t, u, v)) = self.tris[i as usize].intersect(o, d, t_min, *tmax) {
                    *tmax = t;
                    best = Some((t, i, u, v));
                }
            }
            false
        });
        best.map(|(t, i, u, v)| TriangleHit {
            t,
            prim: self.bvh.prim_indices[i as usize],
            u,
            v,
        })
    }

    /// Intersezione più vicina accettata dal filtro `accept(prim, t, u, v)` (es. ritaglio
    /// con mappa di opacità): le intersezioni rifiutate vengono attraversate.
    pub fn intersect_filtered(
        &self,
        o: Vec3,
        d: Vec3,
        t_min: f32,
        t_max: f32,
        accept: impl Fn(u32, f32, f32, f32) -> bool,
    ) -> Option<TriangleHit> {
        if self.tris.is_empty() {
            return None;
        }
        let mut best: Option<(f32, u32, f32, f32)> = None;
        self.bvh.traverse(o, d, t_min, t_max, |first, count, tmax| {
            for i in first..first + count {
                if let Some((t, u, v)) = self.tris[i as usize].intersect(o, d, t_min, *tmax) {
                    if accept(self.bvh.prim_indices[i as usize], t, u, v) {
                        *tmax = t;
                        best = Some((t, i, u, v));
                    }
                }
            }
            false
        });
        best.map(|(t, i, u, v)| TriangleHit {
            t,
            prim: self.bvh.prim_indices[i as usize],
            u,
            v,
        })
    }

    /// Come [`occluded`](Self::occluded), ma ignora le intersezioni rifiutate dal filtro.
    pub fn occluded_filtered(&self, o: Vec3, d: Vec3, t_min: f32, t_max: f32, accept: impl Fn(u32, f32, f32, f32) -> bool) -> bool {
        if self.tris.is_empty() {
            return false;
        }
        let mut hit = false;
        self.bvh.traverse(o, d, t_min, t_max, |first, count, tmax| {
            for i in first..first + count {
                if let Some((t, u, v)) = self.tris[i as usize].intersect(o, d, t_min, *tmax) {
                    if accept(self.bvh.prim_indices[i as usize], t, u, v) {
                        hit = true;
                        return true;
                    }
                }
            }
            false
        });
        hit
    }

    /// `true` se qualcosa interseca il segmento `(t_min, t_max)` (raggi d'ombra).
    pub fn occluded(&self, o: Vec3, d: Vec3, t_min: f32, t_max: f32) -> bool {
        if self.tris.is_empty() {
            return false;
        }
        let mut hit = false;
        self.bvh.traverse(o, d, t_min, t_max, |first, count, tmax| {
            for i in first..first + count {
                if self.tris[i as usize].intersect(o, d, t_min, *tmax).is_some() {
                    hit = true;
                    return true;
                }
            }
            false
        });
        hit
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_math::Pcg32;

    fn random_soup(n: usize, seed: u64) -> Vec<[Vec3; 3]> {
        let mut rng = Pcg32::new(seed, 3);
        let mut r = || rng.next_f32() * 2.0 - 1.0;
        (0..n)
            .map(|_| {
                let c = Vec3::new(r(), r(), r()) * 10.0;
                [
                    c + Vec3::new(r(), r(), r()),
                    c + Vec3::new(r(), r(), r()),
                    c + Vec3::new(r(), r(), r()),
                ]
            })
            .collect()
    }

    fn brute(tris: &[[Vec3; 3]], o: Vec3, d: Vec3) -> Option<(f32, u32)> {
        let mut best = None;
        let mut tmax = f32::INFINITY;
        for (i, t) in tris.iter().enumerate() {
            if let Some((t, _, _)) = Triangle::new(t[0], t[1], t[2]).intersect(o, d, 1e-4, tmax) {
                tmax = t;
                best = Some((t, i as u32));
            }
        }
        best
    }

    #[test]
    fn bvh_matches_brute_force() {
        let tris = random_soup(3000, 42);
        let accel = TriangleAccel::build(&tris);
        let stats = accel.bvh.stats();
        assert!(stats.max_leaf_size <= 8, "{stats:?}");
        let mut rng = Pcg32::new(9, 9);
        let mut hits = 0;
        for _ in 0..2000 {
            let o = Vec3::new(rng.next_f32() - 0.5, rng.next_f32() - 0.5, rng.next_f32() - 0.5) * 30.0;
            let (a, b) = rng.next_2d();
            let d = forge_math::sampling::uniform_sphere(a, b);
            let expected = brute(&tris, o, d);
            let got = accel.intersect(o, d, 1e-4, f32::INFINITY);
            match (expected, got) {
                (None, None) => {}
                (Some((t, i)), Some(h)) => {
                    hits += 1;
                    assert!((t - h.t).abs() < 1e-4, "t {t} vs {}", h.t);
                    // A parità di t (triangoli coincidenti) l'indice può differire.
                    if (t - h.t).abs() > 1e-6 {
                        assert_eq!(i, h.prim);
                    }
                }
                (e, g) => panic!("discordanza: brute {e:?}, bvh {g:?}"),
            }
            assert_eq!(accel.occluded(o, d, 1e-4, f32::INFINITY), expected.is_some());
        }
        assert!(hits > 100, "troppi pochi hit per un test significativo: {hits}");
    }

    #[test]
    fn occlusion_respects_tmax() {
        let tris = vec![[Vec3::new(-1.0, -1.0, 5.0), Vec3::new(1.0, -1.0, 5.0), Vec3::new(0.0, 1.0, 5.0)]];
        let a = TriangleAccel::build(&tris);
        assert!(a.occluded(Vec3::ZERO, Vec3::Z, 1e-4, 10.0));
        assert!(!a.occluded(Vec3::ZERO, Vec3::Z, 1e-4, 4.0));
    }

    #[test]
    fn empty_and_degenerate_inputs() {
        let a = TriangleAccel::build(&[]);
        assert!(a.intersect(Vec3::ZERO, Vec3::Z, 0.0, 1.0).is_none());
        // Molti triangoli identici: centroidi coincidenti, nessuno split SAH possibile.
        let t = [Vec3::ZERO, Vec3::X, Vec3::Y];
        let a = TriangleAccel::build(&vec![t; 100]);
        assert!(a.intersect(Vec3::new(0.2, 0.2, -1.0), Vec3::Z, 0.0, 10.0).is_some());
    }

    /// Oltre la soglia del parallelo: stesso risultato della ricerca esaustiva, ogni
    /// primitiva in una sola foglia, figli dentro il box del padre, qualità SAH come la
    /// costruzione seriale.
    #[test]
    fn parallel_build_is_correct() {
        let tris = random_soup(60_000, 7);
        let t0 = std::time::Instant::now();
        let accel = TriangleAccel::build(&tris);
        eprintln!(
            "BVH parallela di {} triangoli in {:.0} ms",
            tris.len(),
            t0.elapsed().as_secs_f64() * 1e3
        );
        let bvh = &accel.bvh;
        let mut seen = vec![0u8; tris.len()];
        for n in &bvh.nodes {
            if n.is_leaf() {
                for k in n.left_or_first..n.left_or_first + n.count {
                    seen[bvh.prim_indices[k as usize] as usize] += 1;
                }
            } else {
                let b = n.aabb();
                for c in [n.left_or_first, n.left_or_first + 1] {
                    let cb = bvh.nodes[c as usize].aabb();
                    assert!(cb.min.cmpge(b.min - Vec3::splat(1e-4)).all() && cb.max.cmple(b.max + Vec3::splat(1e-4)).all());
                }
            }
        }
        assert!(seen.iter().all(|&c| c == 1), "primitive mancanti o ripetute");
        let one = std::thread::scope(|_| {
            let pool = rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap();
            pool.install(|| TriangleAccel::build(&tris))
        });
        let (a, b) = (bvh.stats().sah_cost, one.bvh.stats().sah_cost);
        assert!((a - b).abs() < 1e-3 * b, "SAH {a} contro {b} (1 thread)");
        let mut rng = Pcg32::new(3, 3);
        for _ in 0..500 {
            let o = Vec3::new(rng.next_f32() - 0.5, rng.next_f32() - 0.5, rng.next_f32() - 0.5) * 30.0;
            let (u, v) = rng.next_2d();
            let d = forge_math::sampling::uniform_sphere(u, v);
            let got = accel.intersect(o, d, 0.0, f32::INFINITY).map(|h| h.t);
            let exp = brute(&tris, o, d).map(|h| h.0);
            match (got, exp) {
                (Some(g), Some(e)) => assert!((g - e).abs() < 1e-3, "{g} vs {e}"),
                (None, None) => {}
                x => panic!("risultato diverso: {x:?}"),
            }
        }
    }
}
