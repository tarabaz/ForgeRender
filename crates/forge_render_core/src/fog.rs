//! Nebbie volumetriche nella scena compilata: voxelizzazione degli oggetti
//! (collisioni) dalla BVH e cache dei volumi calcolati.
//!
//! Il calcolo di una nebbia richiede da decimi di secondo a qualche secondo:
//! viene rifatto solo quando cambiano i suoi parametri o la geometria.

use crate::RenderGeometry;
use forge_math::Vec3;
use forge_scene::{FogVolume, Scene};
use forge_volume::{BakedFog, GridSpec, SimState};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// Celle della griglia attraversate da una superficie della scena: raggi lungo i
/// tre assi per ogni riga di celle, ogni intersezione marca la cella che la contiene.
/// Il guscio ottenuto separa interno ed esterno degli oggetti (connessione per facce).
pub fn voxelize(geo: &RenderGeometry, spec: &GridSpec) -> Vec<bool> {
    use rayon::prelude::*;
    let mut solid = vec![false; spec.len()];
    if geo.accel.is_empty() {
        return solid;
    }
    let max = spec.max();
    for axis in 0..3 {
        let (a1, a2) = ((axis + 1) % 3, (axis + 2) % 3);
        let rows: Vec<(usize, usize)> = (0..spec.res[a2]).flat_map(|j| (0..spec.res[a1]).map(move |i| (i, j))).collect();
        let hits: Vec<Vec<usize>> = rows
            .par_iter()
            .map(|&(i, j)| {
                let mut o = spec.min;
                o[a1] += (i as f32 + 0.5) * spec.cell;
                o[a2] += (j as f32 + 0.5) * spec.cell;
                o[axis] = spec.min[axis] - spec.cell;
                let mut d = Vec3::ZERO;
                d[axis] = 1.0;
                let t_end = max[axis] - o[axis] + spec.cell;
                let mut cells = Vec::new();
                let mut t = 0.0f32;
                let step = spec.cell * 1e-3;
                while let Some(h) = geo.accel.intersect(o, d, t, t_end) {
                    if let Some((x, y, z)) = spec.cell_of(o + d * h.t) {
                        cells.push(spec.index(x, y, z));
                    }
                    t = h.t + step;
                }
                cells
            })
            .collect();
        for c in hits.into_iter().flatten() {
            solid[c] = true;
        }
    }
    solid
}

type Cache<T> = Mutex<HashMap<(String, u64), Arc<T>>>;

/// Nebbie pronte (simulazione + dettaglio), per tutti i parametri.
fn cache() -> &'static Cache<BakedFog> {
    static C: OnceLock<Cache<BakedFog>> = OnceLock::new();
    C.get_or_init(Default::default)
}

/// Simulazioni, per i soli parametri che cambiano il moto: cambiare densità, colore,
/// anisotropia o scala del dettaglio rifà solo il dettaglio (frazioni di secondo).
fn sim_cache() -> &'static Cache<SimState> {
    static C: OnceLock<Cache<SimState>> = OnceLock::new();
    C.get_or_init(Default::default)
}

/// Superficie su cui si appoggia la nebbia sotto la sua zona: la più alta tra quelle che
/// stanno sotto il fondo della zona, cercata con raggi verso il basso su una griglia 3×3
/// attraverso la zona. `None` se la zona arriva già sulle superfici (o sotto non c'è nulla).
///
/// Non si usa il punto più basso della scena: in una scena con un bancone alto un metro
/// la zona si allungherebbe fino al pavimento e le celle diventerebbero grossolane.
pub fn ground_under(f: &FogVolume, geo: &RenderGeometry) -> Option<f32> {
    if geo.accel.is_empty() {
        return None;
    }
    let f = f.sanitized();
    let (lo, hi) = f.bounds();
    let mut ground: Option<f32> = None;
    for i in 0..3 {
        for j in 0..3 {
            let x = lo.x + (hi.x - lo.x) * (0.1 + 0.4 * i as f32);
            let z = lo.z + (hi.z - lo.z) * (0.1 + 0.4 * j as f32);
            let o = Vec3::new(x, f.emitter.y.max(lo.y), z);
            let Some(h) = geo.accel.intersect(o, -Vec3::Y, 0.0, f32::INFINITY) else {
                continue;
            };
            let y = o.y - h.t;
            if y < lo.y {
                ground = Some(ground.map_or(y, |g: f32| g.max(y)));
            }
        }
    }
    ground
}

fn geo_key(f: &FogVolume, geo: &RenderGeometry) -> u64 {
    // La geometria conta solo con le collisioni.
    if f.collisions {
        geo.id
    } else {
        0
    }
}

fn key(f: &FogVolume, geo: &RenderGeometry) -> (String, u64) {
    let f = f.sanitized();
    // Il nome non cambia la nebbia.
    let k = format!(
        "{:?}",
        FogVolume {
            name: String::new(),
            ..f.clone()
        }
    );
    (k, geo_key(&f, geo))
}

/// Nebbie attive della scena già calcolate (senza calcolarne di nuove)?
pub fn fogs_ready(scene: &Scene, geo: &RenderGeometry) -> bool {
    let c = cache().lock().unwrap();
    scene.fogs.iter().filter(|f| f.enabled).all(|f| c.contains_key(&key(f, geo)))
}

/// Nebbie attive della scena, calcolate se non sono già in cache.
pub fn bake_fogs(scene: &Scene, geo: &RenderGeometry) -> Vec<Arc<BakedFog>> {
    let mut out = Vec::new();
    for f in scene.fogs.iter().filter(|f| f.enabled) {
        let k = key(f, geo);
        if let Some(b) = cache().lock().unwrap().get(&k) {
            out.push(b.clone());
            continue;
        }
        let solid = |spec: &GridSpec| voxelize(geo, spec);
        let ground = ground_under(f, geo);
        let sk = (forge_volume::sim_key(f), geo_key(&f.sanitized(), geo));
        let cached_sim = sim_cache().lock().unwrap().get(&sk).cloned();
        let sim = match cached_sim {
            Some(s) => s,
            None => {
                let s = Arc::new(forge_volume::simulate_fog(f, Some(&solid), ground));
                let mut c = sim_cache().lock().unwrap();
                if c.len() > 8 {
                    c.clear();
                }
                c.insert(sk, s.clone());
                s
            }
        };
        let b = Arc::new(forge_volume::bake_detail(f, &sim, Some(&solid), ground));
        let mut c = cache().lock().unwrap();
        if c.len() > 16 {
            c.clear();
        }
        c.insert(k, b.clone());
        out.push(b);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_math::Mat4;
    use forge_scene::{primitives, ComponentId};

    /// Bancone a y = 0 sopra un pavimento a y = −1: la nebbia sopra il bancone si appoggia
    /// al bancone, non al pavimento (altrimenti la zona si allunga e le celle ingrossano).
    #[test]
    fn fog_rests_on_the_surface_below_not_on_the_lowest_point() {
        let mut s = Scene::default();
        for (name, size, y) in [("bancone", 2.0, 0.0), ("pavimento", 10.0, -1.0)] {
            let m = s.add_mesh(primitives::plane(size, size));
            s.add_node(None, ComponentId(name.into()), name, Mat4::from_translation(Vec3::Y * y), Some(m));
        }
        let geo = RenderGeometry::build(&s);
        let high = FogVolume {
            emitter: Vec3::new(0.0, 0.5, 0.0),
            size: Vec3::splat(0.4),
            ..Default::default()
        };
        let g = ground_under(&high, &geo).expect("superficie sotto la zona");
        assert!(g.abs() < 1e-4, "{g}");
        // La zona arriva già sotto il bancone: non si estende.
        let low = FogVolume {
            emitter: Vec3::new(0.0, 0.1, 0.0),
            ..high
        };
        assert_eq!(ground_under(&low, &geo), None);
    }
}
