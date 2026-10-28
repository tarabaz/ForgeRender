//! Simulazione semplificata della nebbia da ghiaccio secco su griglia 3D.
//!
//! La nebbia di CO₂ è più pesante dell'aria: esce dall'emettitore, cade, si
//! accumula sulle superfici (e sul fondo della zona) e si allarga come una
//! corrente di gravità, scorrendo attorno agli ostacoli; il bordo sottile si
//! dissolve. Modello per cella (concentrazione `c`, 1 = nebbia piena):
//!
//! - caduta: flusso verso il basso `v·c·(1 − c_sotto/c_piena)` (non entra in celle già piene);
//! - corrente di gravità: la "pressione" idrostatica di una cella è la nebbia che
//!   le sta sopra nello strato appoggiato (colonna continua che poggia su un
//!   ostacolo o sul fondo); la differenza di pressione tra celle vicine alla stessa
//!   quota spinge il flusso orizzontale (upwind, velocità limitata); la nebbia in
//!   caduta libera non si allarga;
//! - turbolenza: velocità orizzontale da curl noise aggiunta agli stessi flussi upwind;
//! - dissolvenza: decadimento esponenziale, più rapido dove la nebbia è sottile;
//! - ostacoli: le celle solide non scambiano nulla; il fondo della zona è chiuso,
//!   i lati e la cima sono aperti.
//!
//! Ogni passo calcola il nuovo stato dal precedente cella per cella (Jacobi) e i
//! flussi di una faccia sono calcolati allo stesso modo dalle due celle: la massa
//! si conserva e il risultato non dipende dal numero di thread.

use crate::grid::GridSpec;
use crate::noise;
use forge_math::Vec3;
use rayon::prelude::*;

/// Costanti "efficaci" (tarate sul comportamento visivo della nebbia reale).
const FALL_SPEED: f32 = 0.9; // m/s in aria libera
/// Velocità della corrente per unità di gradiente di spessore (m/s).
const GRAVITY_FLOW: f32 = 0.7;
const MAX_FLOW: f32 = 0.35; // m/s
const SMOOTH: f32 = 0.0015; // m²/s, diffusione residua (bordi morbidi)
/// Concentrazione oltre la quale una cella non accoglie altra nebbia dall'alto:
/// la nebbia si accumula in uno strato di spessore visibile.
pub const FULL: f32 = 0.35;
/// Concentrazione minima perché una cella faccia parte di uno strato appoggiato.
const SUPPORT: f32 = 0.03;
const SOURCE_FLOW: f32 = 0.04; // m³/s di nebbia piena per unità di intensità
const DISSIPATION: f32 = 0.05; // 1/s
/// Velocità della turbolenza a intensità 1 (m/s).
pub const TURB_SPEED: f32 = 0.3;

pub struct SimInput<'a> {
    pub spec: GridSpec,
    pub solid: &'a [bool],
    pub emitter: Vec3,
    pub sim_time: f32,
    pub emission_rate: f32,
    /// Raggio della sorgente (m).
    pub source_radius: f32,
    pub dissipation: f32,
    pub turbulence: f32,
    /// Dimensione dei vortici (metri).
    pub swirl_scale: f32,
    pub seed: u32,
}

pub struct SimResult {
    pub conc: Vec<f32>,
    /// Campo di turbolenza (m/s) sulla stessa griglia, riusato per deformare il dettaglio.
    pub velocity: Vec<Vec3>,
    pub steps: u32,
    pub dt: f32,
}

pub fn simulate(inp: &SimInput) -> SimResult {
    let g = inp.spec;
    let h = g.cell;
    let n = g.len();
    let [nx, ny, nz] = g.res;
    let u_turb = TURB_SPEED * inp.turbulence;
    // Stabilità: flusso uscente totale da una cella ≤ metà del contenuto.
    let mut dt = (0.5 * h / FALL_SPEED)
        .min(0.12 * h / (MAX_FLOW + u_turb * 1.8))
        .min(0.1 * h * h / SMOOTH)
        .min(1.0 / 30.0);
    let steps = (inp.sim_time / dt).ceil().max(0.0) as u32;
    if steps > 0 {
        dt = inp.sim_time / steps as f32;
    }

    // Campo di turbolenza statico a divergenza nulla.
    let velocity: Vec<Vec3> = (0..n)
        .into_par_iter()
        .map(|i| {
            if u_turb <= 0.0 {
                return Vec3::ZERO;
            }
            let (x, y, z) = g.coords(i);
            let p = g.center(x, y, z) / inp.swirl_scale.max(1e-3);
            let v = noise::curl(p, inp.seed ^ 0xa511_e9b3);
            // Solo orizzontale: i vortici della nebbia pesante sono quasi piatti, e un
            // trasporto verticale "pescherebbe" aria vuota sopra lo strato.
            Vec3::new(v.x, 0.0, v.z) * u_turb
        })
        .collect();

    // Scorrevolezza del fronte: variazione spaziale (dal seed) che lo divide in lobi
    // e lingue, come nelle correnti di gravità reali (instabilità "lobe and cleft").
    let mobility: Vec<f32> = (0..n)
        .into_par_iter()
        .map(|i| {
            let (x, y, z) = g.coords(i);
            let p = g.center(x, y, z);
            let q = Vec3::new(p.x, 0.0, p.z) / (inp.swirl_scale * 0.35).max(1e-3);
            let v = noise::fbm(q, inp.seed ^ 0x3c6e_f372, 3) * 0.5 + 0.5;
            let _ = y;
            0.25 + 1.5 * v * (0.3 + 0.7 * inp.turbulence) + 0.7 * (1.0 - inp.turbulence)
        })
        .collect();

    // Celle della sorgente: disco orizzontale (la ciotola) alto una cella, solo celle libere.
    let r_src = inp.source_radius.max(0.75 * h);
    let mut source: Vec<usize> = (0..n)
        .filter(|&i| {
            let (x, y, z) = g.coords(i);
            let q = g.center(x, y, z) - inp.emitter;
            !inp.solid[i] && Vec3::new(q.x, 0.0, q.z).length() <= r_src && q.y.abs() <= 0.5 * h
        })
        .collect();
    if source.is_empty() {
        // Emettitore dentro un oggetto: prima cella libera sopra.
        if let Some((x, y0, z)) = g.cell_of(inp.emitter.clamp(g.min, g.max() - Vec3::splat(1e-4))) {
            if let Some(y) = (y0..ny).find(|&y| !inp.solid[g.index(x, y, z)]) {
                source.push(g.index(x, y, z));
            }
        }
    }
    let add_per_step = if source.is_empty() {
        0.0
    } else {
        SOURCE_FLOW * inp.emission_rate * dt / (h * h * h) / source.len() as f32
    };

    let mut c = vec![0.0f32; n];
    let mut next = vec![0.0f32; n];
    // Pressione in ordine per colonne: (z·nx + x)·ny + y.
    let mut pressure = vec![0.0f32; n];
    let fall = FALL_SPEED * dt / h;
    let k_smooth = SMOOTH * dt / (h * h);
    let decay = DISSIPATION * inp.dissipation;
    let sy = nx * nz;
    let pidx = |x: usize, y: usize, z: usize| (z * nx + x) * ny + y;
    for _ in 0..steps {
        for &i in &source {
            c[i] += add_per_step;
        }
        // Pressione idrostatica: esiste solo nella nebbia appoggiata (colonna continua
        // che poggia su un ostacolo o sul fondo); quella in caduta libera non spinge.
        {
            let cur = &c;
            pressure.par_chunks_mut(ny).enumerate().for_each(|(col, out)| {
                let (x, z) = (col % nx, col / nx);
                let mut supported = true; // il fondo della zona è chiuso
                for (y, o) in out.iter_mut().enumerate() {
                    let i = g.index(x, y, z);
                    if inp.solid[i] {
                        supported = true;
                        *o = -1.0; // marcatore: ostacolo
                        continue;
                    }
                    supported = supported && cur[i] > SUPPORT;
                    *o = if supported { 0.0 } else { -2.0 }; // -2: non appoggiata
                }
                let mut acc = 0.0f32;
                for y in (0..ny).rev() {
                    let i = g.index(x, y, z);
                    if out[y] == 0.0 {
                        acc += cur[i].min(1.5) * h;
                        out[y] = acc;
                    } else {
                        acc = 0.0;
                        out[y] = 0.0;
                    }
                }
            });
        }

        let cur = &c;
        let pres = &pressure;
        next.par_chunks_mut(nx).enumerate().for_each(|(row, out_row)| {
            let (y, z) = (row / nz, row % nz);
            for (x, out) in out_row.iter_mut().enumerate() {
                let i = g.index(x, y, z);
                if inp.solid[i] {
                    *out = 0.0;
                    continue;
                }
                let c0 = cur[i];
                // Celle vuote con vicini vuoti: niente da fare (la maggior parte).
                if c0 == 0.0
                    && (x == 0 || cur[i - 1] == 0.0)
                    && (x + 1 == nx || cur[i + 1] == 0.0)
                    && (z == 0 || cur[i - nx] == 0.0)
                    && (z + 1 == nz || cur[i + nx] == 0.0)
                    && (y + 1 == ny || cur[i + sy] == 0.0)
                    && (y == 0 || cur[i - sy] == 0.0)
                {
                    *out = 0.0;
                    continue;
                }
                let p0 = pres[pidx(x, y, z)];
                let mut delta = 0.0;
                // Facce orizzontali: corrente di gravità (upwind) + diffusione residua.
                // Fuori dalla zona: aria pulita (la nebbia esce liberamente).
                let v0 = velocity[i];
                let mut face = |j: Option<(usize, usize, usize)>, dir: Vec3| {
                    let (cn, pn, vn) = match j {
                        Some((jx, jz, j)) => {
                            if inp.solid[j] {
                                return;
                            }
                            (cur[j], pres[pidx(jx, y, jz)], velocity[j])
                        }
                        None => (0.0, 0.0, v0),
                    };
                    // Velocità sulla faccia dalla cella verso la vicina (positiva = esce):
                    // corrente di gravità + turbolenza.
                    let mob = 0.5 * (mobility[i] + j.map(|(_, _, j)| mobility[j]).unwrap_or(mobility[i]));
                    let u = (GRAVITY_FLOW * mob * (p0 - pn) / h).clamp(-MAX_FLOW, MAX_FLOW) + 0.5 * (v0 + vn).dot(dir);
                    let flux = if u > 0.0 { u * c0 } else { u * cn };
                    delta -= flux * dt / h;
                    delta += k_smooth * (cn - c0);
                };
                face((x > 0).then(|| (x - 1, z, i - 1)), -Vec3::X);
                face((x + 1 < nx).then(|| (x + 1, z, i + 1)), Vec3::X);
                face((z > 0).then(|| (x, z - 1, i - nx)), -Vec3::Z);
                face((z + 1 < nz).then(|| (x, z + 1, i + nx)), Vec3::Z);
                // Sotto: fondo chiuso.
                if y > 0 && !inp.solid[i - sy] {
                    let cb = cur[i - sy];
                    delta -= fall * c0 * (1.0 - cb / FULL).clamp(0.0, 1.0);
                    delta += k_smooth * (cb - c0);
                }
                // Sopra: cima aperta.
                if y + 1 < ny {
                    if !inp.solid[i + sy] {
                        let ca = cur[i + sy];
                        delta += fall * ca * (1.0 - c0 / FULL).clamp(0.0, 1.0);
                        delta += k_smooth * (ca - c0);
                    }
                } else {
                    delta -= k_smooth * c0;
                }
                let v = (c0 + delta).max(0.0);
                // Il bordo sottile si mescola con l'aria e sparisce prima.
                let k = decay * (0.6 + 0.8 * (1.0 - (v / FULL).min(1.0)));
                *out = v * (-k * dt).exp();
            }
        });
        std::mem::swap(&mut c, &mut next);
    }
    SimResult {
        conc: c,
        velocity,
        steps,
        dt,
    }
}
