//! Simulazione fluida della nebbia da ghiaccio secco (fumo euleriano, Fedkiw et al. 2001).
//!
//! Griglia sfalsata (MAC): velocità sulle facce, grandezze al centro delle celle.
//! Ogni passo:
//! 1. sorgente: vapore visibile `ρ` e gas pesante `c` (CO₂ freddo) nel disco
//!    dell'emettitore, con una spinta iniziale verso l'alto;
//! 2. forze: peso del gas pesante (`−g_giù·c`), risalita del vapore caldo
//!    (`+g_su·risalita·ρ`), turbolenza da curl noise, confinamento della vorticità
//!    (riccioli e filamenti), attrito con l'aria;
//! 3. trasporto semi-lagrangiano della velocità;
//! 4. proiezione: pressione con Gauss–Seidel rosso/nero (SOR), il flusso non entra
//!    negli ostacoli né nel fondo; lati e cima aperti;
//! 5. trasporto di `ρ` e `c` con MacCormack limitato (filamenti più netti);
//! 6. dissolvenza: il gas pesante si mescola con l'aria più in fretta del vapore,
//!    così la nebbia "vecchia" diventa leggera e risale a sbuffi mentre svanisce.
//!
//! Tutti i passi leggono lo stato precedente e scrivono in un buffer nuovo (anche i
//! due colori del rosso/nero): il risultato non dipende dal numero di thread.

use crate::grid::GridSpec;
use crate::noise;
use forge_math::Vec3;
use rayon::prelude::*;

/// Accelerazione verso il basso del gas pesante puro (m/s² per unità di `c`).
const G_DOWN: f32 = 4.0;
/// Accelerazione verso l'alto del vapore caldo a risalita 1 (m/s² per unità di `ρ`).
const G_UP: f32 = 2.5;
/// Attrito con l'aria (1/s): dà velocità limite realistiche (~0.4 m/s in caduta).
const DRAG: f32 = 1.5;
/// Attrito delle superfici (1/s) sulle facce accanto a un ostacolo o al fondo: lo strato
/// a contatto rallenta, il tappeto di nebbia resta spesso e avanza lentamente.
const WALL_DRAG: f32 = 6.0;
/// Spessore dello strato limite (m): con celle più grandi l'attrito agisce solo su questa
/// frazione della cella a contatto.
const BOUNDARY_LAYER: f32 = 0.012;
/// Portata di nebbia piena per unità di intensità (m³/s).
const SOURCE_FLOW: f32 = 0.004;
/// Velocità verso l'alto con cui esce il vapore (m/s) a risalita 0 e 1.
const EXIT_SPEED: [f32; 2] = [0.03, 0.2];
/// Velocità della turbolenza a intensità 1 (m/s): scala usata anche dal dettaglio.
pub const TURB_SPEED: f32 = 0.3;
/// Concentrazione considerata "nebbia piena" (per il dettaglio procedurale).
pub const FULL: f32 = 0.6;
/// Iterazioni della pressione (rosso/nero).
const PRESSURE_ITERS: usize = 16;
const SOR: f32 = 1.6;
/// Margine (celle) del riquadro di lavoro attorno alla nebbia: fuori l'aria resta ferma.
const WORK_MARGIN: usize = 6;
/// Spostamento massimo per passo (celle). Il trasporto semi-lagrangiano è stabile anche
/// oltre 1; per non saltare le pareti sottili si controlla anche il punto intermedio.
const MAX_CFL: f32 = 2.0;
/// Velocità massima della nebbia (m/s): le colate reali non superano ~1 m/s.
const MAX_SPEED: f32 = 1.2;

/// Tempi per fase (ms), per log e benchmark: `lap(k)` attribuisce alla fase precedente
/// il tempo trascorso dall'ultimo `lap`.
struct Clock {
    last: std::time::Instant,
    phase: usize,
    ms: [f64; 7],
}

impl Clock {
    fn new() -> Self {
        Self {
            last: std::time::Instant::now(),
            phase: 0,
            ms: [0.0; 7],
        }
    }

    fn lap(&mut self, next: usize) {
        let now = std::time::Instant::now();
        self.ms[self.phase] += (now - self.last).as_secs_f64() * 1e3;
        self.last = now;
        self.phase = next;
    }
}

/// Due campi trasportati insieme: valori e intervallo dei vicini (limitatore MacCormack).
#[derive(Clone, Copy, Default)]
struct S2 {
    v: [f32; 2],
    lo: [f32; 2],
    hi: [f32; 2],
}

/// Puntatore condiviso per aggiornare in parallelo celle disgiunte (rosso/nero).
#[derive(Clone, Copy)]
struct Shared(*mut f32);
unsafe impl Send for Shared {}
unsafe impl Sync for Shared {}

impl Shared {
    /// Puntatore alla cella `i` (un metodo: la chiusura cattura tutta la struttura, che è Sync).
    #[inline]
    fn at(self, i: usize) -> *mut f32 {
        // SAFETY: `i` è un indice valido del buffer da cui è stato creato il puntatore.
        unsafe { self.0.add(i) }
    }
}

#[derive(Clone, Copy)]
struct Shared8(*mut u8);
unsafe impl Send for Shared8 {}
unsafe impl Sync for Shared8 {}

impl Shared8 {
    #[inline]
    fn at(self, i: usize) -> *mut u8 {
        // SAFETY: `i` è un indice valido del buffer da cui è stato creato il puntatore.
        unsafe { self.0.add(i) }
    }
}

/// Riquadro di lavoro (estremi inclusi) in celle.
#[derive(Clone, Copy)]
struct Bx {
    lo: [usize; 3],
    hi: [usize; 3],
}

impl Bx {
    #[inline]
    fn contains(&self, x: usize, y: usize, z: usize) -> bool {
        x >= self.lo[0] && x <= self.hi[0] && y >= self.lo[1] && y <= self.hi[1] && z >= self.lo[2] && z <= self.hi[2]
    }

    /// Celle del riquadro allargato di `extra` sul lato alto dell'asse `a` (per le facce).
    fn rows(&self, extra: Option<usize>) -> Vec<(usize, usize)> {
        let mut hi = self.hi;
        if let Some(a) = extra {
            hi[a] += 1;
        }
        (self.lo[1]..=hi[1])
            .flat_map(|y| (self.lo[2]..=hi[2]).map(move |z| (y, z)))
            .collect()
    }
}

/// Celle entro `r` celle (in ogni direzione, massimo separabile) da una cella con nebbia,
/// calcolate solo nel riquadro. Ogni passata lungo un asse usa somme progressive:
/// costo costante per cella qualunque sia `r`.
fn near_fog(g: &GridSpec, b: &Bx, rho: &[f32], r: usize) -> Vec<bool> {
    let [nx, _, nz] = g.res;
    let mut cur: Vec<u8> = vec![0; g.len()];
    // Linee del riquadro lungo un asse: (indice di partenza, passo, lunghezza).
    let lines = |axis: usize| -> Vec<(usize, usize, usize)> {
        let (a1, a2) = ((axis + 1) % 3, (axis + 2) % 3);
        let mut out = Vec::new();
        for u in b.lo[a1]..=b.hi[a1] {
            for v in b.lo[a2]..=b.hi[a2] {
                let mut c = [0usize; 3];
                c[axis] = b.lo[axis];
                c[a1] = u;
                c[a2] = v;
                let stride = [1, nx * nz, nx][axis];
                out.push((g.index(c[0], c[1], c[2]), stride, b.hi[axis] - b.lo[axis] + 1));
            }
        }
        out
    };
    let ptr = Shared8(cur.as_mut_ptr());
    lines(0).par_iter().for_each(|&(start, stride, len)| {
        for k in 0..len {
            let i = start + k * stride;
            // SAFETY: ogni linea scrive solo le proprie celle.
            unsafe { *ptr.at(i) = (rho[i] > 1e-6) as u8 };
        }
    });
    for axis in 0..3 {
        let ls = lines(axis);
        let ptr = Shared8(cur.as_mut_ptr());
        ls.par_iter().for_each_init(Vec::new, |pre: &mut Vec<u32>, &(start, stride, len)| {
            // SAFETY: le linee lungo lo stesso asse sono disgiunte.
            let get = |k: usize| unsafe { *ptr.at(start + k * stride) };
            pre.clear();
            pre.resize(len + 1, 0);
            for k in 0..len {
                pre[k + 1] = pre[k] + get(k) as u32;
            }
            for k in 0..len {
                let (lo, hi) = (k.saturating_sub(r), (k + r).min(len - 1));
                let v = (pre[hi + 1] - pre[lo] > 0) as u8;
                unsafe { *ptr.at(start + k * stride) = v };
            }
        });
    }
    cur.into_iter().map(|v| v != 0).collect()
}

/// Riquadro delle celle con nebbia allargato di `margin` celle (None se non c'è nebbia).
fn fog_box(g: &GridSpec, rho: &[f32], margin: usize) -> Option<Bx> {
    let [nx, _, nz] = g.res;
    // Per riga (y, z): prima e ultima x con nebbia.
    let (lo, hi) = rho
        .par_chunks(nx)
        .enumerate()
        .filter_map(|(row, chunk)| {
            let first = chunk.iter().position(|v| *v > 1e-6)?;
            let last = chunk.iter().rposition(|v| *v > 1e-6)?;
            let (y, z) = (row / nz, row % nz);
            Some(([first, y, z], [last, y, z]))
        })
        .reduce_with(|a, b| ([0, 1, 2].map(|k| a.0[k].min(b.0[k])), [0, 1, 2].map(|k| a.1[k].max(b.1[k]))))?;
    Some(Bx {
        lo: [0, 1, 2].map(|k| lo[k].saturating_sub(margin)),
        hi: [0, 1, 2].map(|k| (hi[k] + margin).min(g.res[k] - 1)),
    })
}

/// Valori calcolati solo nelle celle del riquadro (fuori: `T::default()`).
fn fill_box<T: Copy + Default + Send + Sync>(g: &GridSpec, b: &Bx, f: impl Fn(usize, usize, usize, usize) -> T + Sync) -> Vec<T> {
    let mut out = vec![T::default(); g.len()];
    let [nx, _, nz] = g.res;
    out.par_chunks_mut(nx).enumerate().for_each(|(row, chunk)| {
        let (y, z) = (row / nz, row % nz);
        if y < b.lo[1] || y > b.hi[1] || z < b.lo[2] || z > b.hi[2] {
            return;
        }
        for (x, o) in chunk.iter_mut().enumerate().take(b.hi[0] + 1).skip(b.lo[0]) {
            *o = f(g.index(x, y, z), x, y, z);
        }
    });
    out
}

/// Aggiorna le facce del campo `a` nel riquadro (allargato di una faccia sull'asse `a`).
fn update_faces(m: &Mac, a: usize, b: &Bx, data: &mut [f32], f: impl Fn(usize, usize, usize, usize, &mut f32) + Sync) {
    let d = m.dims(Some(a));
    let mut hi = b.hi;
    hi[a] += 1;
    data.par_chunks_mut(d[0]).enumerate().for_each(|(row, chunk)| {
        let (y, z) = (row / d[2], row % d[2]);
        if y < b.lo[1] || y > hi[1] || z < b.lo[2] || z > hi[2] {
            return;
        }
        for (x, v) in chunk.iter_mut().enumerate().take(hi[0] + 1).skip(b.lo[0]) {
            f(Mac::idx(d, x, y, z), x, y, z, v);
        }
    });
}

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
    /// Tendenza della nebbia vecchia a risalire (0–1).
    pub rise: f32,
    /// Dimensione dei vortici (metri).
    pub swirl_scale: f32,
    pub seed: u32,
}

pub struct SimResult {
    /// Densità del vapore visibile al centro delle celle.
    pub conc: Vec<f32>,
    /// Velocità al centro delle celle (m/s), usata per orientare il dettaglio.
    pub velocity: Vec<Vec3>,
    pub steps: u32,
    pub dt: f32,
}

/// Griglia sfalsata: dimensioni e accesso ai campi.
#[derive(Clone, Copy)]
struct Mac {
    n: [usize; 3],
    h: f32,
    min: Vec3,
}

impl Mac {
    fn dims(&self, comp: Option<usize>) -> [usize; 3] {
        let mut d = self.n;
        if let Some(a) = comp {
            d[a] += 1;
        }
        d
    }

    #[inline]
    fn idx(d: [usize; 3], x: usize, y: usize, z: usize) -> usize {
        (y * d[2] + z) * d[0] + x
    }

    /// Posizione (in celle) del campione `comp` (None = centro cella) nel punto `p`.
    #[inline]
    fn local(&self, p: Vec3, comp: Option<usize>) -> Vec3 {
        let mut q = (p - self.min) / self.h - Vec3::splat(0.5);
        if let Some(a) = comp {
            q[a] += 0.5;
        }
        q
    }

    /// Posizione nel mondo del campione (x, y, z) del campo `comp`.
    #[inline]
    fn pos(&self, comp: Option<usize>, x: usize, y: usize, z: usize) -> Vec3 {
        let mut q = Vec3::new(x as f32, y as f32, z as f32) + Vec3::splat(0.5);
        if let Some(a) = comp {
            q[a] -= 0.5;
        }
        self.min + q * self.h
    }
}

/// Trilineare su un campo di dimensioni `d` alle coordinate locali `q` (bordo esteso).
#[inline]
fn tri(data: &[f32], d: [usize; 3], q: Vec3) -> f32 {
    let f = q.floor();
    let t = q - f;
    let c = |v: f32, a: usize| (v as i64).clamp(0, d[a] as i64 - 1) as usize;
    let (x0, y0, z0) = (c(f.x, 0), c(f.y, 1), c(f.z, 2));
    let (x1, y1, z1) = (c(f.x + 1.0, 0), c(f.y + 1.0, 1), c(f.z + 1.0, 2));
    let at = |x, y, z| data[Mac::idx(d, x, y, z)];
    let l = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let tx = t.x.clamp(0.0, 1.0);
    let ty = t.y.clamp(0.0, 1.0);
    let tz = t.z.clamp(0.0, 1.0);
    l(
        l(l(at(x0, y0, z0), at(x1, y0, z0), tx), l(at(x0, y1, z0), at(x1, y1, z0), tx), ty),
        l(l(at(x0, y0, z1), at(x1, y0, z1), tx), l(at(x0, y1, z1), at(x1, y1, z1), tx), ty),
        tz,
    )
}

struct Vel {
    c: [Vec<f32>; 3],
}

impl Vel {
    fn sample(&self, m: &Mac, p: Vec3) -> Vec3 {
        Vec3::new(
            tri(&self.c[0], m.dims(Some(0)), m.local(p, Some(0))),
            tri(&self.c[1], m.dims(Some(1)), m.local(p, Some(1))),
            tri(&self.c[2], m.dims(Some(2)), m.local(p, Some(2))),
        )
    }
}

pub fn simulate(inp: &SimInput) -> SimResult {
    let g = inp.spec;
    let m = Mac {
        n: g.res,
        h: g.cell,
        min: g.min,
    };
    let [nx, ny, nz] = g.res;
    let n = g.len();
    let h = g.cell;
    let solid = |x: i64, y: i64, z: i64| -> bool {
        // Fuori dalla griglia: aria (lati e cima), il fondo è chiuso.
        if y < 0 {
            return true;
        }
        if x < 0 || z < 0 || x >= nx as i64 || y >= ny as i64 || z >= nz as i64 {
            return false;
        }
        inp.solid[g.index(x as usize, y as usize, z as usize)]
    };

    // Sorgente: disco orizzontale (la bocca della ciotola) alto una cella.
    let r_src = inp.source_radius.max(0.75 * h);
    let mut source: Vec<usize> = (0..n)
        .filter(|&i| {
            let (x, y, z) = g.coords(i);
            let q = g.center(x, y, z) - inp.emitter;
            !inp.solid[i] && Vec3::new(q.x, 0.0, q.z).length() <= r_src && q.y.abs() <= 0.5 * h
        })
        .collect();
    if source.is_empty() {
        if let Some((x, y0, z)) = g.cell_of(inp.emitter.clamp(g.min, g.max() - Vec3::splat(1e-4))) {
            if let Some(y) = (y0..ny).find(|&y| !inp.solid[g.index(x, y, z)]) {
                source.push(g.index(x, y, z));
            }
        }
    }

    // Turbolenza: due campi di curl noise (a divergenza nulla) che si alternano nel tempo.
    let turb_field = |salt: u32| -> Vec<Vec3> {
        (0..n)
            .into_par_iter()
            .map(|i| {
                if inp.turbulence <= 0.0 {
                    return Vec3::ZERO;
                }
                let (x, y, z) = g.coords(i);
                let p = g.center(x, y, z) / inp.swirl_scale.max(1e-3);
                noise::curl(p, inp.seed ^ salt)
            })
            .collect()
    };
    let turb = [turb_field(0xa511_e9b3), turb_field(0x1b87_3593)];

    let rise = inp.rise.clamp(0.0, 1.0);
    // Portata di gas (m³/s) distribuita sulle celle della sorgente: divergenza (1/s).
    let src_div = SOURCE_FLOW * inp.emission_rate / (source.len().max(1) as f32 * h * h * h);
    let exit_speed = EXIT_SPEED[0] + (EXIT_SPEED[1] - EXIT_SPEED[0]) * rise;
    let eps = 0.3 + 1.5 * inp.turbulence; // confinamento della vorticità
    let k_turb = 0.5 * inp.turbulence; // m/s² della forza di turbolenza
                                       // Il gas pesante si mescola con l'aria più in fretta del vapore (con più risalita, ancora prima).
    let k_heavy = (0.04 + 0.6 * rise) * inp.dissipation.max(0.05);
    let k_vis = 0.06 * inp.dissipation;

    let mut vel = Vel {
        c: [0, 1, 2].map(|a| vec![0.0f32; m.dims(Some(a)).iter().product()]),
    };
    let mut rho = vec![0.0f32; n];
    let mut heavy = vec![0.0f32; n];
    let mut pressure = vec![0.0f32; n];

    // Vicini di ogni cella per la pressione: celle libere interne (indici) e numero di
    // facce non solide (le facce verso l'aria fuori dalla griglia contano, con p = 0).
    #[derive(Clone, Copy, Default)]
    struct Nb {
        idx: [u32; 6],
        n: u8,
        k: u8,
    }
    let neighbors: Vec<Nb> = (0..n)
        .into_par_iter()
        .map(|i| {
            let mut nb = Nb::default();
            if inp.solid[i] {
                return nb;
            }
            let (x, y, z) = g.coords(i);
            let (xi, yi, zi) = (x as i64, y as i64, z as i64);
            for (dx, dy, dz) in [(1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)] {
                let (qx, qy, qz) = (xi + dx, yi + dy, zi + dz);
                if solid(qx, qy, qz) {
                    continue;
                }
                nb.k += 1;
                if qx >= 0 && qz >= 0 && qx < nx as i64 && qy < ny as i64 && qz < nz as i64 {
                    nb.idx[nb.n as usize] = g.index(qx as usize, qy as usize, qz as usize) as u32;
                    nb.n += 1;
                }
            }
            nb
        })
        .collect();

    // Passo temporale: 1/30 s, ridotto se le velocità superano MAX_CFL celle per passo.
    let mut clock = Clock::new();
    let mut t = 0.0f32;
    let mut steps = 0u32;
    let mut last_dt = 1.0 / 30.0;
    while t < inp.sim_time - 1e-5 {
        let vmax = vel
            .c
            .iter()
            .map(|c| c.par_iter().fold(|| 0.0f32, |a, v| a.max(v.abs())).reduce(|| 0.0, f32::max))
            .fold(0.0f32, f32::max);
        let mut dt = (1.0f32 / 30.0).min(MAX_CFL * h / vmax.max(1e-6)).min(inp.sim_time - t);
        if dt < 1e-5 {
            dt = 1e-5;
        }
        last_dt = dt;

        clock.lap(0);
        // 1. Sorgente: gas puro appena prodotto (concentrazione 1) con una spinta verso l'alto.
        for &i in &source {
            rho[i] = rho[i].max(1.0);
            heavy[i] = heavy[i].max(1.0);
            let (x, y, z) = g.coords(i);
            let j = Mac::idx(m.dims(Some(1)), x, y + 1, z);
            vel.c[1][j] = vel.c[1][j].max(exit_speed);
        }

        // Riquadro di lavoro: nebbia più un margine. Fuori l'aria è considerata ferma e la
        // nebbia non può arrivarci in un passo (spostamento < 1 cella).
        let Some(bx) = fog_box(&g, &rho, WORK_MARGIN) else {
            t += dt;
            steps += 1;
            continue;
        };

        clock.lap(1);
        // Celle vicine alla nebbia (entro 3 celle): solo lì servono trasporto e forze.
        let near = near_fog(&g, &bx, &rho, 3);

        // 2. Forze (al centro delle celle, poi riportate sulle facce).
        let phase = 0.5 + 0.5 * (t * std::f32::consts::TAU / 6.0).sin();
        let center_vel: Vec<Vec3> = fill_box(&g, &bx, |i, x, y, z| {
            if !near[i] {
                return Vec3::ZERO;
            }
            let a = |c: usize, dx: usize, dy: usize, dz: usize| vel.c[c][Mac::idx(m.dims(Some(c)), x + dx, y + dy, z + dz)];
            Vec3::new(
                0.5 * (a(0, 0, 0, 0) + a(0, 1, 0, 0)),
                0.5 * (a(1, 0, 0, 0) + a(1, 0, 1, 0)),
                0.5 * (a(2, 0, 0, 0) + a(2, 0, 0, 1)),
            )
        });
        let at = |x: i64, y: i64, z: i64| -> Vec3 {
            let c = |v: i64, a: usize| v.clamp(0, g.res[a] as i64 - 1) as usize;
            center_vel[g.index(c(x, 0), c(y, 1), c(z, 2))]
        };
        let omega: Vec<Vec3> = fill_box(&g, &bx, |i, x, y, z| {
            if !near[i] {
                return Vec3::ZERO;
            }
            let (x, y, z) = (x as i64, y as i64, z as i64);
            let d = |dx: i64, dy: i64, dz: i64| (at(x + dx, y + dy, z + dz) - at(x - dx, y - dy, z - dz)) / (2.0 * h);
            let (dx, dy, dz) = (d(1, 0, 0), d(0, 1, 0), d(0, 0, 1));
            Vec3::new(dy.z - dz.y, dz.x - dx.z, dx.y - dy.x)
        });
        let force: Vec<Vec3> = fill_box(&g, &bx, |i, x, y, z| {
            if inp.solid[i] || rho[i] <= 0.0 {
                return Vec3::ZERO;
            }
            let mut f = Vec3::new(0.0, G_UP * rise * rho[i] - G_DOWN * heavy[i], 0.0);
            // Turbolenza solo dove c'è nebbia (l'aria ferma resta ferma).
            let presence = (rho[i] / 0.05).min(1.0);
            f += (turb[0][i] * phase + turb[1][i] * (1.0 - phase)) * (k_turb * presence);
            // Confinamento della vorticità: N × ω, con N = ∇|ω| normalizzato.
            if eps > 0.0 && presence > 0.0 {
                let mag = |dx: i64, dy: i64, dz: i64| {
                    let c = |v: i64, a: usize| v.clamp(0, g.res[a] as i64 - 1) as usize;
                    omega[g.index(c(x as i64 + dx, 0), c(y as i64 + dy, 1), c(z as i64 + dz, 2))].length()
                };
                let grad = Vec3::new(
                    mag(1, 0, 0) - mag(-1, 0, 0),
                    mag(0, 1, 0) - mag(0, -1, 0),
                    mag(0, 0, 1) - mag(0, 0, -1),
                );
                let nrm = grad.normalize_or_zero();
                f += nrm.cross(omega[i]) * (eps * h * presence);
            }
            f
        });
        let damp = (-DRAG * dt).exp();
        let wall_damp = (-(DRAG + WALL_DRAG * (BOUNDARY_LAYER / h).min(1.0)) * dt).exp();
        for a in 0..3 {
            update_faces(&m, a, &bx, &mut vel.c[a], |_, x, y, z, v| {
                let mut lo = [x as i64, y as i64, z as i64];
                lo[a] -= 1;
                let hi = [x as i64, y as i64, z as i64];
                if solid(lo[0], lo[1], lo[2]) || solid(hi[0], hi[1], hi[2]) {
                    *v = 0.0;
                    return;
                }
                // Lontano dalla nebbia nessuna forza (l'aria ferma resta ferma).
                if !near[g.index(x.min(nx - 1), y.min(ny - 1), z.min(nz - 1))] {
                    return;
                }
                let fa = |c: [i64; 3]| {
                    if c[0] < 0 || c[1] < 0 || c[2] < 0 || c[0] >= nx as i64 || c[1] >= ny as i64 || c[2] >= nz as i64 {
                        0.0
                    } else {
                        force[g.index(c[0] as usize, c[1] as usize, c[2] as usize)][a]
                    }
                };
                // Sopra una superficie d'appoggio il moto orizzontale è frenato dall'attrito
                // (la caduta lungo le pareti verticali no).
                let near_wall = a != 1 && (solid(lo[0], lo[1] - 1, lo[2]) || solid(hi[0], hi[1] - 1, hi[2]));
                let d = if near_wall { wall_damp } else { damp };
                *v = ((*v + dt * 0.5 * (fa(lo) + fa(hi))) * d).clamp(-MAX_SPEED, MAX_SPEED);
            });
        }

        clock.lap(2);
        // 3. Trasporto della velocità.
        let old = Vel { c: vel.c.clone() };
        for a in 0..3 {
            let d = m.dims(Some(a));
            update_faces(&m, a, &bx, &mut vel.c[a], |_, x, y, z, v| {
                let mut lo = [x as i64, y as i64, z as i64];
                lo[a] -= 1;
                if solid(lo[0], lo[1], lo[2]) || solid(x as i64, y as i64, z as i64) {
                    *v = 0.0;
                    return;
                }
                // Lontano dalla nebbia l'aria si muove poco: il trasporto si salta.
                if !near[g.index(x.min(nx - 1), y.min(ny - 1), z.min(nz - 1))] {
                    return;
                }
                let p = m.pos(Some(a), x, y, z);
                let back = p - old.sample(&m, p) * dt;
                let mid = (p + back) * 0.5;
                let blocked = [mid, back].iter().any(|q| match g.cell_of(*q) {
                    Some((cx, cy, cz)) => inp.solid[g.index(cx, cy, cz)],
                    None => false,
                });
                if !blocked {
                    *v = tri(&old.c[a], d, m.local(back, Some(a)));
                }
            });
        }

        clock.lap(3);
        // 4. Proiezione (pressione).
        let div: Vec<f32> = fill_box(&g, &bx, |i, x, y, z| {
            if inp.solid[i] {
                return 0.0;
            }
            let f = |a: usize, dx: usize, dy: usize, dz: usize| vel.c[a][Mac::idx(m.dims(Some(a)), x + dx, y + dy, z + dz)];
            (f(0, 1, 0, 0) - f(0, 0, 0, 0) + f(1, 0, 1, 0) - f(1, 0, 0, 0) + f(2, 0, 0, 1) - f(2, 0, 0, 0)) / h
        });
        // Il gas prodotto occupa volume: divergenza imposta nelle celle della sorgente.
        let mut div = div;
        for &i in &source {
            div[i] -= src_div;
        }
        let rhs_scale = h * h / dt;
        // Fuori dal riquadro la pressione è 0 (aria ferma, come un bordo aperto).
        pressure.par_chunks_mut(nx).enumerate().for_each(|(row, chunk)| {
            let (y, z) = (row / nz, row % nz);
            for (x, p) in chunk.iter_mut().enumerate() {
                if !bx.contains(x, y, z) {
                    *p = 0.0;
                }
            }
        });
        let rows = bx.rows(None);
        for _ in 0..PRESSURE_ITERS {
            for color in 0..2usize {
                // Gauss–Seidel rosso/nero sul posto: le celle di un colore leggono solo celle
                // dell'altro colore, quindi le scritture parallele non si sovrappongono e il
                // risultato non dipende dall'ordine.
                let ptr = Shared(pressure.as_mut_ptr());
                let (neighbors, div) = (&neighbors, &div);
                rows.par_iter().for_each(|&(y, z)| {
                    let start = bx.lo[0] + (bx.lo[0] + y + z + color) % 2;
                    for x in (start..=bx.hi[0]).step_by(2) {
                        let i = g.index(x, y, z);
                        let nb = &neighbors[i];
                        if nb.k == 0 {
                            continue;
                        }
                        // SAFETY: `i` ha il colore corrente; i vicini letti hanno l'altro colore e
                        // in questa mezza iterazione nessun thread li scrive. Ogni `i` è scritto
                        // da un solo thread (una riga a testa).
                        unsafe {
                            let sum: f32 = nb.idx[..nb.n as usize].iter().map(|&j| *ptr.at(j as usize)).sum();
                            let cur = *ptr.at(i);
                            let gs = (sum - rhs_scale * div[i]) / nb.k as f32;
                            *ptr.at(i) = cur + SOR * (gs - cur);
                        }
                    }
                });
            }
        }
        let p_at = |x: i64, y: i64, z: i64| -> f32 {
            // Fuori dalla griglia o dal riquadro: aria aperta, p = 0.
            let outside = x < 0 || y < 0 || z < 0 || x >= nx as i64 || y >= ny as i64 || z >= nz as i64;
            if outside || !bx.contains(x as usize, y as usize, z as usize) {
                0.0
            } else {
                pressure[g.index(x as usize, y as usize, z as usize)]
            }
        };
        for a in 0..3 {
            update_faces(&m, a, &bx, &mut vel.c[a], |_, x, y, z, v| {
                let mut lo = [x as i64, y as i64, z as i64];
                lo[a] -= 1;
                if solid(lo[0], lo[1], lo[2]) || solid(x as i64, y as i64, z as i64) {
                    *v = 0.0;
                    return;
                }
                *v -= dt / h * (p_at(x as i64, y as i64, z as i64) - p_at(lo[0], lo[1], lo[2]));
            });
        }

        clock.lap(4);
        // 5. Trasporto di vapore e gas pesante (MacCormack limitato), insieme: stessa traiettoria.
        let sl2 = |fa: &[f32], fb: &[f32], i: usize, x: usize, y: usize, z: usize, sign: f32| -> S2 {
            let p = g.center(x, y, z);
            let q = p - vel.sample(&m, p) * dt * sign;
            let keep = S2 {
                v: [fa[i], fb[i]],
                lo: [fa[i], fb[i]],
                hi: [fa[i], fb[i]],
            };
            // Tragitto attraverso un ostacolo (punto intermedio solido): resta il valore della cella.
            if let Some((cx, cy, cz)) = g.cell_of((p + q) * 0.5) {
                if inp.solid[g.index(cx, cy, cz)] {
                    return keep;
                }
            }
            let l = m.local(q, None);
            let f = l.floor();
            let mut lo = [f32::INFINITY; 2];
            let mut hi = [f32::NEG_INFINITY; 2];
            for k in 0..8 {
                let c = |v: f32, d: usize, a: usize| (v as i64 + d as i64).clamp(0, g.res[a] as i64 - 1) as usize;
                let j = g.index(c(f.x, k & 1, 0), c(f.y, (k >> 1) & 1, 1), c(f.z, (k >> 2) & 1, 2));
                if inp.solid[j] {
                    // Non si pesca attraverso gli ostacoli: cella più vicina se libera.
                    return match g.cell_of(q) {
                        Some((cx, cy, cz)) if !inp.solid[g.index(cx, cy, cz)] => {
                            let j = g.index(cx, cy, cz);
                            S2 {
                                v: [fa[j], fb[j]],
                                lo: [fa[j], fb[j]],
                                hi: [fa[j], fb[j]],
                            }
                        }
                        _ => keep,
                    };
                }
                lo = [lo[0].min(fa[j]), lo[1].min(fb[j])];
                hi = [hi[0].max(fa[j]), hi[1].max(fb[j])];
            }
            S2 {
                v: [tri(fa, g.res, l), tri(fb, g.res, l)],
                lo,
                hi,
            }
        };
        let fwd: Vec<S2> = fill_box(&g, &bx, |i, x, y, z| {
            if inp.solid[i] || !near[i] {
                S2::default()
            } else {
                sl2(&rho, &heavy, i, x, y, z, 1.0)
            }
        });
        let fa: Vec<f32> = fwd.par_iter().map(|s| s.v[0]).collect();
        let fb: Vec<f32> = fwd.par_iter().map(|s| s.v[1]).collect();
        let out: Vec<[f32; 2]> = fill_box(&g, &bx, |i, x, y, z| {
            if inp.solid[i] || !near[i] {
                return [0.0; 2];
            }
            let s = fwd[i];
            let back = sl2(&fa, &fb, i, x, y, z, -1.0);
            [
                (s.v[0] + 0.5 * (rho[i] - back.v[0])).clamp(s.lo[0], s.hi[0]).max(0.0),
                (s.v[1] + 0.5 * (heavy[i] - back.v[1])).clamp(s.lo[1], s.hi[1]).max(0.0),
            ]
        });
        rho = out.par_iter().map(|v| v[0]).collect();
        heavy = out.par_iter().map(|v| v[1]).collect();

        clock.lap(5);
        // 6. Dissolvenza.
        let fade_vis = |v: f32| {
            // Il velo sottile svanisce prima del corpo denso.
            let thin = 1.0 - (v / 0.15).min(1.0);
            v * (-(k_vis * (1.0 + 4.0 * thin)) * dt).exp()
        };
        rho.par_iter_mut().for_each(|v| *v = fade_vis(*v));
        let kh = (-k_heavy * dt).exp();
        heavy.par_iter_mut().zip(rho.par_iter()).for_each(|(c, r)| *c = (*c * kh).min(*r));

        clock.lap(6);
        t += dt;
        steps += 1;
    }
    if std::env::var_os("FORGE_FOG_PROFILE").is_some() {
        eprintln!("profilo nebbia: {steps} passi, ms {:?}", clock.ms.map(|v| v.round()));
    }
    tracing::info!(
        "simulazione nebbia: {steps} passi; ms per fase: passo {:.0}, sorgente/riquadro {:.0}, forze {:.0}, trasporto velocità {:.0}, pressione {:.0}, trasporto nebbia {:.0}, dissolvenza {:.0}",
        clock.ms[0], clock.ms[1], clock.ms[2], clock.ms[3], clock.ms[4], clock.ms[5], clock.ms[6]
    );

    let velocity: Vec<Vec3> = (0..n)
        .into_par_iter()
        .map(|i| {
            let (x, y, z) = g.coords(i);
            vel.sample(&m, g.center(x, y, z))
        })
        .collect();
    SimResult {
        conc: rho,
        velocity,
        steps,
        dt: last_dt,
    }
}
