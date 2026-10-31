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
const SIM_MAX: usize = 256;

/// Nebbia pronta per il rendering.
#[derive(Clone, Debug)]
pub struct BakedFog {
    pub spec: GridSpec,
    /// Densità quantizzata: σt = `data[i] as f32 * unit`.
    pub data: Vec<u16>,
    pub unit: f32,
    /// Blocchi di [`MACRO_CELLS`]³ celle per lato (`res` diviso per eccesso).
    pub macro_res: [usize; 3],
    /// Majorant di σt per blocco (stesso ordine di [`GridSpec::index`]: x, poi z, poi y),
    /// valido per l'interpolazione trilineare in tutto il blocco (include le celle di bordo).
    pub macro_majorant: Vec<f32>,
    pub albedo: Vec3,
    pub anisotropy: f32,
    pub max_density: f32,
    /// Visibile solo ai raggi che vengono dalla camera (anche attraverso vetri e specchi), con
    /// una sola diffusione: i fasci delle spot. Non illumina la scena con rimbalzi (deboli e
    /// molto rumorosi con una luce puntiforme); le nebbie simulate restano fisiche (false).
    pub camera_only: bool,
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
        let data = vec![if density > 0.0 { 65535 } else { 0 }; n];
        let (macro_res, macro_majorant) = macro_majorants(&spec, &data, unit);
        BakedFog {
            spec,
            data,
            unit,
            macro_res,
            macro_majorant,
            albedo,
            anisotropy,
            max_density: density,
            camera_only: false,
            sim_steps: 0,
            sim_ms: 0.0,
            bake_ms: 0.0,
        }
    }

    /// Mezzo con densità data da una funzione (σt in 1/m), campionata ai centri delle celle di
    /// una griglia che copre [min, max] con `longest` celle sul lato più lungo. Serve per i
    /// volumi costruiti a mano (es. il fascio di una luce spot).
    pub fn from_fn(min: Vec3, max: Vec3, longest: usize, albedo: Vec3, anisotropy: f32, density: impl Fn(Vec3) -> f32 + Sync) -> BakedFog {
        use rayon::prelude::*;
        let t0 = std::time::Instant::now();
        let spec = GridSpec::covering(min, max, longest);
        let values: Vec<f32> = (0..spec.len())
            .into_par_iter()
            .map(|i| {
                let (x, y, z) = spec.coords(i);
                density(spec.center(x, y, z)).max(0.0)
            })
            .collect();
        let max_density = values.iter().copied().fold(0.0f32, f32::max);
        let unit = max_density.max(1e-12) / 65535.0;
        let data: Vec<u16> = values.iter().map(|v| (v / unit).round().clamp(0.0, 65535.0) as u16).collect();
        let (macro_res, macro_majorant) = macro_majorants(&spec, &data, unit);
        BakedFog {
            spec,
            data,
            unit,
            macro_res,
            macro_majorant,
            albedo,
            anisotropy,
            max_density,
            camera_only: false,
            sim_steps: 0,
            sim_ms: 0.0,
            bake_ms: t0.elapsed().as_secs_f64() * 1e3,
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

    /// Blocco della griglia dei majorant che contiene `p` (indici limitati alla griglia).
    fn macro_block(&self, p: Vec3) -> [usize; 3] {
        let q = (p - self.spec.min) / (self.spec.cell * MACRO_CELLS as f32);
        [0, 1, 2].map(|a| (q[a].floor().max(0.0) as usize).min(self.macro_res[a] - 1))
    }

    fn macro_index(&self, b: [usize; 3]) -> usize {
        (b[1] * self.macro_res[2] + b[2]) * self.macro_res[0] + b[0]
    }

    /// Majorant valido nel punto `p` (quello del suo blocco).
    pub fn majorant_at(&self, p: Vec3) -> f32 {
        self.macro_majorant[self.macro_index(self.macro_block(p))]
    }

    /// Tratto a majorant costante del raggio `o + d·t` a partire da `t` (al più fino a `t1`):
    /// restituisce (fine del tratto, majorant). Si percorre la griglia dei blocchi con un
    /// DDA: i blocchi vuoti si saltano in un colpo, quelli radi con pochi passi.
    pub fn majorant_segment(&self, o: Vec3, d: Vec3, t: f32, t1: f32) -> (f32, f32) {
        let side = self.spec.cell * MACRO_CELLS as f32;
        // Il blocco è quello in cui il raggio sta entrando: il punto un filo più avanti
        // evita di riprendere il blocco appena lasciato per arrotondamento (il majorant
        // copre comunque mezza cella oltre il bordo del blocco).
        let b = self.macro_block(o + d * (t + 1e-4 * side));
        let mut tb = t1;
        for a in 0..3 {
            let edge = if d[a] > 1e-9 {
                b[a] + 1
            } else if d[a] < -1e-9 {
                b[a]
            } else {
                continue;
            };
            tb = tb.min((self.spec.min[a] + edge as f32 * side - o[a]) / d[a]);
        }
        // Avanzamento minimo per non fermarsi sui confini per errori di arrotondamento.
        let tb = tb.max(t + 1e-6 * (1.0 + t.abs())).min(t1);
        (tb, self.macro_majorant[self.macro_index(b)])
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
        "{:?} {:?} {:?} {} {} {} {} {} {} {} {} {} {} {}",
        f.emitter,
        f.zone_offset,
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
        f.sim_cells,
        f.enabled
    )
}

/// Zona simulata effettiva (estesa fino al suolo se le collisioni sono attive).
fn zone(f: &FogVolume, ground: Option<f32>) -> (Vec3, Vec3) {
    f.bounds_over(if f.collisions { ground } else { None })
}

/// Lato dei blocchi della griglia dei majorant, in celle.
pub const MACRO_CELLS: usize = 8;

/// Majorant per blocchi di `MACRO_CELLS`³ celle: massimo di σt sulle celle del blocco più
/// una di bordo per lato, cioè su tutte le celle che l'interpolazione trilineare usa per i
/// punti del blocco (fino a mezza cella oltre il suo bordo).
fn macro_majorants(spec: &GridSpec, data: &[u16], unit: f32) -> ([usize; 3], Vec<f32>) {
    let b = MACRO_CELLS;
    let mres = spec.res.map(|r| r.div_ceil(b).max(1));
    let n = mres[0] * mres[1] * mres[2];
    let maj = (0..n)
        .into_par_iter()
        .map(|i| {
            let bx = i % mres[0];
            let bz = (i / mres[0]) % mres[2];
            let by = i / (mres[0] * mres[2]);
            let range = |k: usize, a: usize| (k * b).saturating_sub(1)..=((k + 1) * b).min(spec.res[a] - 1);
            let mut m = 0u16;
            for y in range(by, 1) {
                for z in range(bz, 2) {
                    for x in range(bx, 0) {
                        m = m.max(data[spec.index(x, y, z)]);
                    }
                }
            }
            m as f32 * unit * 1.0001
        })
        .collect();
    (mres, maj)
}

/// Griglie usate dalla nebbia: (simulazione, dettaglio). Gli ostacoli che contano sono
/// solo quelli voxelizzati su queste griglie: il renderer le usa per capire se una
/// modifica della scena tocca davvero la nebbia.
pub fn grid_specs(fog: &FogVolume, ground: Option<f32>) -> (GridSpec, GridSpec) {
    let f = fog.sanitized();
    let (min, max) = zone(&f, ground);
    // Simulazione al 70% della risoluzione del dettaglio (il tappeto e le colate devono
    // avere qualche cella di spessore per scorrere bene).
    let sim = GridSpec::covering(
        min,
        max,
        ((f.resolution as f32 * 0.7) as usize).clamp(32, (f.sim_cells as usize).min(SIM_MAX)),
    );
    (sim, GridSpec::covering(min, max, f.resolution as usize))
}

/// Filtro [1 2 1]/4 separabile sui tre assi (bordi replicati).
fn smooth_field(data: &[f32], spec: &GridSpec) -> Vec<f32> {
    let mut a = data.to_vec();
    let mut b = vec![0.0f32; a.len()];
    for axis in 0..3 {
        let n = spec.res[axis];
        b.par_iter_mut().enumerate().for_each(|(i, out)| {
            let (x, y, z) = spec.coords(i);
            let c = [x, y, z];
            let at = |k: usize| {
                let mut q = c;
                q[axis] = k;
                a[spec.index(q[0], q[1], q[2])]
            };
            let k = c[axis];
            *out = 0.25 * at(k.saturating_sub(1)) + 0.5 * at(k) + 0.25 * at((k + 1).min(n - 1));
        });
        std::mem::swap(&mut a, &mut b);
    }
    a
}

/// Simula la nebbia. `solid` restituisce le celle occupate dagli oggetti
/// (ignorato se le collisioni sono spente); `ground` è la superficie sotto la zona (vedi `ground_under` nel renderer):
/// la zona si estende fino a lì (vedi [`FogVolume::bounds_over`]).
pub fn simulate_fog(fog: &FogVolume, solid: Option<SolidFn>, ground: Option<f32>) -> SimState {
    let f = fog.sanitized();
    let t0 = std::time::Instant::now();
    let spec = grid_specs(&f, ground).0;
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
    let sim_spec = sim_state.spec;
    let sim_ms = sim_state.ms;
    // Campo della simulazione ammorbidito: l'interpolazione trilineare dei bordi netti
    // (nebbia piena accanto al vuoto) lascia gradini allineati alle celle, visibili da
    // vicino e messi in evidenza dal denoiser.
    let conc = smooth_field(&sim_state.conc, &sim_spec);
    let sim = SimState {
        conc,
        spec: sim_spec,
        steps: sim_state.steps,
        ms: sim_state.ms,
    };
    let sim = &sim;
    let solid_for = |spec: &GridSpec| match (f.collisions, solid) {
        (true, Some(s)) => s(spec),
        _ => vec![false; spec.len()],
    };

    let t1 = std::time::Instant::now();
    let spec = grid_specs(&f, ground).1;
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
    let (macro_res, macro_majorant) = macro_majorants(&spec, &data, unit);
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
        macro_res,
        macro_majorant,
        albedo: Vec3::from_array(f.albedo),
        anisotropy: f.anisotropy,
        max_density,
        camera_only: false,
        sim_steps: sim.steps,
        sim_ms,
        bake_ms,
    }
}

#[cfg(test)]
mod tests;
