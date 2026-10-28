//! Nebbia volumetrica procedurale per il path tracer.
//!
//! Pipeline (deterministica dal seed):
//! 1. [`sim::simulate`]: simulazione fluida (velocità, pressione, vorticità) su una
//!    griglia grossolana: il gas pesante esce dalla sorgente, riempie i contenitori,
//!    trabocca, cola lungo le pareti e forma il tappeto; la parte che si scalda risale
//!    a sbuffi; ostacoli dalla geometria della scena;
//! 2. [`bake`] su una griglia fine: concentrazione interpolata con deformazione e
//!    tessitura procedurale (grumi "a cavolfiore", bordi erosi in filamenti);
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
const SIM_MAX: usize = 128;

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

/// Risultato della simulazione (riusabile: cambiare solo l'aspetto non la rifà).
pub struct SimState {
    pub spec: GridSpec,
    pub conc: Vec<f32>,
    pub steps: u32,
    pub ms: f64,
}

/// Parametri che cambiano il moto della nebbia (chiave della cache della simulazione):
/// tutto tranne nome, densità, scala dei dettagli, colore e anisotropia.
pub fn sim_key(fog: &FogVolume) -> String {
    let f = fog.sanitized();
    format!(
        "{:?} {:?} {} {} {} {} {} {} {} {} {} {}",
        f.emitter,
        f.size,
        f.seed,
        f.sim_time,
        f.source_radius,
        f.emission_rate,
        f.dissipation,
        f.turbulence,
        f.rise,
        f.collisions,
        f.resolution,
        f.enabled
    )
}

/// Zona simulata effettiva (estesa fino al suolo se le collisioni sono attive).
fn zone(f: &FogVolume, ground: Option<f32>) -> (Vec3, Vec3) {
    f.bounds_over(if f.collisions { ground } else { None })
}

/// Simula la nebbia. `solid` restituisce le celle occupate dagli oggetti
/// (ignorato se le collisioni sono spente); `ground` è il punto più basso della scena:
/// la zona si estende fino a lì (vedi [`FogVolume::bounds_over`]).
pub fn simulate_fog(fog: &FogVolume, solid: Option<SolidFn>, ground: Option<f32>) -> SimState {
    let f = fog.sanitized();
    let (min, max) = zone(&f, ground);
    let t0 = std::time::Instant::now();
    // Simulazione al 70% della risoluzione del dettaglio (il tappeto e le colate devono
    // avere qualche cella di spessore per scorrere bene).
    let spec = GridSpec::covering(min, max, ((f.resolution as f32 * 0.7) as usize).clamp(32, SIM_MAX));
    let sim_solid = match (f.collisions, solid) {
        (true, Some(s)) => s(&spec),
        _ => vec![false; spec.len()],
    };
    // Vortici proporzionati alla zona (indipendenti dalla scala del dettaglio, che è solo aspetto).
    let swirl_scale = 0.12 * f.size.x.max(f.size.z);
    let sim = sim::simulate(&sim::SimInput {
        spec,
        solid: &sim_solid,
        emitter: f.emitter,
        sim_time: f.sim_time,
        emission_rate: f.emission_rate,
        source_radius: f.source_radius,
        dissipation: f.dissipation,
        turbulence: f.turbulence,
        rise: f.rise,
        swirl_scale,
        seed: f.seed,
    });
    SimState {
        spec,
        conc: sim.conc,
        steps: sim.steps,
        ms: t0.elapsed().as_secs_f64() * 1e3,
    }
}

/// Calcola la nebbia: simulazione e dettaglio.
pub fn bake(fog: &FogVolume, solid: Option<SolidFn>, ground: Option<f32>) -> BakedFog {
    let s = simulate_fog(fog, solid, ground);
    bake_detail(fog, &s, solid, ground)
}

/// Griglia fine con il dettaglio e l'aspetto, a partire da una simulazione già fatta.
pub fn bake_detail(fog: &FogVolume, sim_state: &SimState, solid: Option<SolidFn>, ground: Option<f32>) -> BakedFog {
    let f = fog.sanitized();
    let (min, max) = zone(&f, ground);
    let sim_spec = sim_state.spec;
    let sim_ms = sim_state.ms;
    let sim = sim_state;
    let solid_for = |spec: &GridSpec| match (f.collisions, solid) {
        (true, Some(s)) => s(spec),
        _ => vec![false; spec.len()],
    };

    let t1 = std::time::Instant::now();
    let spec = GridSpec::covering(min, max, f.resolution as usize);
    let fine_solid = solid_for(&spec);
    // Dettaglio sotto la risoluzione della simulazione:
    // - deformazione del campo (warp) di circa una cella: bordi a grumi e riccioli;
    // - "billow" (1 − |rumore|) a due scale: la tessitura a cavolfiore della nebbia densa;
    // - erosione dove la nebbia è sottile: filamenti e bordi sfrangiati.
    let warp_amp = sim_spec.cell * (0.6 + 1.4 * f.turbulence);
    let erosion = 0.12 + 0.35 * f.turbulence;
    let seed = f.seed;
    let values: Vec<f32> = (0..spec.len())
        .into_par_iter()
        .map(|i| {
            if fine_solid[i] {
                return 0.0;
            }
            let (x, y, z) = spec.coords(i);
            let p = spec.center(x, y, z);
            // Controllo rapido: niente nebbia nei dintorni → zero (la deformazione è < 2 celle).
            if sim_spec.sample(&sim.conc, p) < 1e-5 && sim_spec.sample(&sim.conc, p + Vec3::Y * warp_amp) < 1e-5 {
                let far = [Vec3::X, -Vec3::X, Vec3::Z, -Vec3::Z, -Vec3::Y]
                    .iter()
                    .all(|d| sim_spec.sample(&sim.conc, p + *d * warp_amp) < 1e-5);
                if far {
                    return 0.0;
                }
            }
            let q = p / f.scale;
            let w = Vec3::new(
                noise::fbm(q, seed.wrapping_add(1), 3),
                noise::fbm(q + Vec3::new(17.1, 3.3, 8.9), seed.wrapping_add(2), 3),
                noise::fbm(q + Vec3::new(-4.2, 11.7, 2.5), seed.wrapping_add(3), 3),
            );
            let c = sim_spec.sample(&sim.conc, p + w * warp_amp);
            if c < 1e-4 {
                return 0.0;
            }
            let cc = (c / sim::FULL).min(2.0);
            let b1 = 1.0 - noise::gradient(q * 1.7 + Vec3::splat(2.3), seed.wrapping_add(4)).abs();
            let b2 = 1.0 - noise::gradient(q * 4.1 + Vec3::splat(7.7), seed.wrapping_add(5)).abs();
            let billow = 0.6 * b1 + 0.4 * b2;
            let n = (noise::fbm(q * 0.8, seed.wrapping_add(6), 4) * 0.5 + 0.5).clamp(0.0, 1.0);
            let th = erosion * (1.0 - n);
            let shaped = ((cc - th) / (1.0 - th).max(1e-3)).clamp(0.0, 2.0);
            f.density * shaped * (0.35 + 0.9 * billow * billow)
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

#[cfg(test)]
mod tests;
