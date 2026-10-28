//! Nebbia volumetrica procedurale per il path tracer.
//!
//! Pipeline (deterministica dal seed):
//! 1. [`sim::simulate`] su una griglia grossolana: caduta, deposito, espansione
//!    laterale attorno agli ostacoli, turbolenza, dissolvenza per il tempo simulato;
//! 2. [`bake`] su una griglia fine: concentrazione interpolata × dettaglio
//!    procedurale (fBm deformato dalla turbolenza, bordi erosi in filamenti);
//! 3. il risultato [`BakedFog`] è la densità σt(p) (1/m) quantizzata a 16 bit,
//!    con un majorant per strato orizzontale per il delta tracking.
//!
//! La simulazione riceve il tempo come parametro: un'animazione futura potrà
//! calcolare un [`BakedFog`] per fotogramma senza cambiare il renderer.

pub mod grid;
pub mod noise;
pub mod sim;

use forge_math::Vec3;
use forge_scene::FogVolume;
pub use grid::GridSpec;
use rayon::prelude::*;

/// Lato massimo (in celle) della griglia di simulazione.
const SIM_MAX: usize = 64;

/// Nebbia pronta per il rendering.
#[derive(Clone, Debug)]
pub struct BakedFog {
    pub spec: GridSpec,
    /// Densità quantizzata: σt = `data[i] as f32 * unit`.
    pub data: Vec<u16>,
    pub unit: f32,
    /// Majorant di σt tra i centri degli strati `k` e `k+1` (ultimo = strato in cima),
    /// valido per l'interpolazione trilineare.
    pub slab_majorant: Vec<f32>,
    pub albedo: Vec3,
    pub anisotropy: f32,
    pub max_density: f32,
    /// Statistiche (per log e benchmark).
    pub sim_steps: u32,
    pub sim_ms: f64,
    pub bake_ms: f64,
}

impl BakedFog {
    /// Mezzo omogeneo in un parallelepipedo (per test e confronti analitici).
    pub fn uniform(min: Vec3, max: Vec3, density: f32, albedo: Vec3, anisotropy: f32) -> BakedFog {
        let spec = GridSpec::covering(min, max, 8);
        // Griglia che copre esattamente il riquadro: celle non cubiche non servono,
        // basta che `max` coincida (si accetta una cella in più sui lati corti).
        let unit = density.max(1e-12) / 65535.0;
        let n = spec.len();
        BakedFog {
            spec,
            data: vec![if density > 0.0 { 65535 } else { 0 }; n],
            unit,
            slab_majorant: vec![density * 1.0001; spec.res[1]],
            albedo,
            anisotropy,
            max_density: density,
            sim_steps: 0,
            sim_ms: 0.0,
            bake_ms: 0.0,
        }
    }

    pub fn bounds(&self) -> (Vec3, Vec3) {
        (self.spec.min, self.spec.max())
    }

    /// σt (1/m) nel punto, con interpolazione trilineare.
    pub fn density(&self, p: Vec3) -> f32 {
        let g = &self.spec;
        let q = (p - g.min) / g.cell;
        let size = Vec3::new(g.res[0] as f32, g.res[1] as f32, g.res[2] as f32);
        if !(q.x >= 0.0 && q.y >= 0.0 && q.z >= 0.0 && q.x <= size.x && q.y <= size.y && q.z <= size.z) {
            return 0.0;
        }
        let q = q - Vec3::splat(0.5);
        let f = q.floor();
        let t = q - f;
        let i0 = [f.x as i64, f.y as i64, f.z as i64];
        let at = |dx: i64, dy: i64, dz: i64| {
            let c = |v: i64, a: usize| v.clamp(0, g.res[a] as i64 - 1) as usize;
            self.data[g.index(c(i0[0] + dx, 0), c(i0[1] + dy, 1), c(i0[2] + dz, 2))] as f32
        };
        let l = |a: f32, b: f32, t: f32| a + (b - a) * t;
        let x00 = l(at(0, 0, 0), at(1, 0, 0), t.x);
        let x10 = l(at(0, 1, 0), at(1, 1, 0), t.x);
        let x01 = l(at(0, 0, 1), at(1, 0, 1), t.x);
        let x11 = l(at(0, 1, 1), at(1, 1, 1), t.x);
        l(l(x00, x10, t.y), l(x01, x11, t.y), t.z) * self.unit
    }

    /// Majorant valido alla quota `y` (in coordinate del mondo).
    pub fn majorant_at_y(&self, y: f32) -> f32 {
        let k = ((y - self.spec.min.y) / self.spec.cell - 0.5).floor();
        let k = (k.max(0.0) as usize).min(self.slab_majorant.len() - 1);
        self.slab_majorant[k]
    }

    /// Volume di nebbia "piena" equivalente (m³): ∫σt / densità massima impostata.
    pub fn optical_volume(&self) -> f64 {
        let c3 = (self.spec.cell as f64).powi(3);
        self.data.iter().map(|&v| v as f64 * self.unit as f64).sum::<f64>() * c3
    }
}

/// Celle solide per una griglia (fornite dal renderer, che ha la geometria).
pub type SolidFn<'a> = &'a (dyn Fn(&GridSpec) -> Vec<bool> + Sync);

/// Calcola la nebbia. `solid` restituisce le celle occupate dagli oggetti
/// (ignorato se le collisioni sono spente).
pub fn bake(fog: &FogVolume, solid: Option<SolidFn>) -> BakedFog {
    let f = fog.sanitized();
    let (min, max) = f.bounds();
    let empty = |spec: &GridSpec| vec![false; spec.len()];
    let solid_for = |spec: &GridSpec| match (f.collisions, solid) {
        (true, Some(s)) => s(spec),
        _ => empty(spec),
    };

    let t0 = std::time::Instant::now();
    let sim_spec = GridSpec::covering(min, max, (f.resolution as usize).min(SIM_MAX));
    let sim_solid = solid_for(&sim_spec);
    let sim = sim::simulate(&sim::SimInput {
        spec: sim_spec,
        solid: &sim_solid,
        emitter: f.emitter,
        sim_time: f.sim_time,
        emission_rate: f.emission_rate,
        source_radius: f.source_radius,
        dissipation: f.dissipation,
        turbulence: f.turbulence,
        swirl_scale: f.scale * 2.5,
        seed: f.seed,
    });
    let sim_ms = t0.elapsed().as_secs_f64() * 1e3;

    let t1 = std::time::Instant::now();
    let spec = GridSpec::covering(min, max, f.resolution as usize);
    let fine_solid = solid_for(&spec);
    // Deformazione del dettaglio lungo la turbolenza (la velocità vale già ~turbolenza·TURB_SPEED):
    // spostamento fino a ~0.6 dettagli a turbolenza piena.
    let vel_scale = 0.6 / sim::TURB_SPEED;
    let contrast = 0.4 + 0.6 * f.turbulence;
    let seed = f.seed;
    let erosion = 0.08 + 0.3 * f.turbulence;
    let values: Vec<f32> = (0..spec.len())
        .into_par_iter()
        .map(|i| {
            if fine_solid[i] {
                return 0.0;
            }
            let (x, y, z) = spec.coords(i);
            let p = spec.center(x, y, z);
            let c = sim_spec.sample(&sim.conc, p);
            if c < 1e-4 {
                return 0.0;
            }
            // Dettagli più schiacciati in verticale: la nebbia pesante si stratifica.
            let mut q = p / f.scale;
            q.y *= 1.8;
            q += sample_vec(&sim_spec, &sim.velocity, p) * vel_scale;
            let n = (noise::fbm(q, seed.wrapping_add(17), 5) * 0.5 + 0.5).clamp(0.0, 1.0);
            let n2 = (noise::fbm(q * 2.7 + Vec3::splat(5.3), seed.wrapping_add(91), 3) * 0.5 + 0.5).clamp(0.0, 1.0);
            let cc = (c / sim::FULL).min(1.5);
            // Dove la nebbia è sottile il rumore la "erode" in filamenti.
            let th = erosion * (1.0 - n);
            let shaped = ((cc - th) / (1.0 - th).max(1e-3)).clamp(0.0, 1.5);
            // Variazione interna (media ~1): sbuffi più densi e più radi.
            let variation = 1.0 + contrast * (0.55 + 0.9 * n2 - 1.0);
            f.density * shaped * variation
        })
        .collect();
    let max_density = values.iter().cloned().fold(0.0f32, f32::max);
    let unit = if max_density > 0.0 { max_density / 65535.0 } else { 1.0 };
    let data: Vec<u16> = values.iter().map(|v| (v / unit).round().clamp(0.0, 65535.0) as u16).collect();
    // Majorant per strato: massimo degli strati k e k+1 (l'interpolazione resta sotto).
    let layer = spec.res[0] * spec.res[2];
    let layer_max: Vec<f32> = (0..spec.res[1])
        .map(|y| data[y * layer..(y + 1) * layer].iter().copied().max().unwrap_or(0) as f32 * unit)
        .collect();
    let slab_majorant: Vec<f32> = (0..spec.res[1])
        .map(|k| layer_max[k].max(*layer_max.get(k + 1).unwrap_or(&0.0)) * 1.0001)
        .collect();
    let bake_ms = t1.elapsed().as_secs_f64() * 1e3;
    tracing::info!(
        "nebbia «{}»: simulazione {}×{}×{} celle, {} passi in {:.0} ms; densità {}×{}×{} in {:.0} ms",
        f.name,
        sim_spec.res[0],
        sim_spec.res[1],
        sim_spec.res[2],
        sim.steps,
        sim_ms,
        spec.res[0],
        spec.res[1],
        spec.res[2],
        bake_ms
    );
    BakedFog {
        spec,
        data,
        unit,
        slab_majorant,
        albedo: Vec3::from_array(f.albedo),
        anisotropy: f.anisotropy,
        max_density,
        sim_steps: sim.steps,
        sim_ms,
        bake_ms,
    }
}

fn sample_vec(spec: &GridSpec, v: &[Vec3], p: Vec3) -> Vec3 {
    // Il campo di turbolenza è liscio: il valore della cella più vicina basta.
    match spec.cell_of(p) {
        Some((x, y, z)) => v[spec.index(x, y, z)],
        None => Vec3::ZERO,
    }
}

#[cfg(test)]
mod tests;
