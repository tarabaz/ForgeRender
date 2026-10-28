//! Nebbie volumetriche nella scena compilata: voxelizzazione degli oggetti
//! (collisioni) dalla BVH e cache dei volumi calcolati.
//!
//! Il calcolo di una nebbia richiede da decimi di secondo a qualche secondo:
//! viene rifatto solo quando cambiano i suoi parametri o la geometria.

use crate::RenderGeometry;
use forge_math::Vec3;
use forge_scene::{FogVolume, Scene};
use forge_volume::{BakedFog, GridSpec};
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

type Cache = Mutex<HashMap<(String, u64), Arc<BakedFog>>>;

fn cache() -> &'static Cache {
    static C: OnceLock<Cache> = OnceLock::new();
    C.get_or_init(Default::default)
}

fn key(f: &FogVolume, geo: &RenderGeometry) -> (String, u64) {
    let f = f.sanitized();
    // Il nome non cambia la nebbia; la geometria conta solo con le collisioni.
    let k = format!(
        "{:?}",
        FogVolume {
            name: String::new(),
            ..f.clone()
        }
    );
    (k, if f.collisions { geo.id } else { 0 })
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
        let b = Arc::new(forge_volume::bake(f, Some(&solid)));
        let mut c = cache().lock().unwrap();
        if c.len() > 16 {
            c.clear();
        }
        c.insert(k, b.clone());
        out.push(b);
    }
    out
}
