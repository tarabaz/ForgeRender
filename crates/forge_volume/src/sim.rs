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

/// Il riquadro di lavoro come griglia a sé: i campi di un passo sono grandi quanto il riquadro,
/// non quanto tutta la griglia (la nebbia ne occupa spesso una piccola parte). Stesso ordine
/// delle celle della griglia (x più veloce, poi z, poi y).
#[derive(Clone, Copy)]
struct Local {
    b: Bx,
    d: [usize; 3],
}

impl Local {
    fn new(b: Bx) -> Self {
        Self {
            b,
            d: [0, 1, 2].map(|k| b.hi[k] - b.lo[k] + 1),
        }
    }

    fn len(&self) -> usize {
        self.d[0] * self.d[1] * self.d[2]
    }

    /// Indice nel riquadro della cella (x, y, z) della griglia, che deve esserci dentro.
    #[inline]
    fn idx(&self, x: usize, y: usize, z: usize) -> usize {
        ((y - self.b.lo[1]) * self.d[2] + (z - self.b.lo[2])) * self.d[0] + (x - self.b.lo[0])
    }

    /// Indice nel riquadro, se la cella è dentro.
    #[inline]
    fn get(&self, x: usize, y: usize, z: usize) -> Option<usize> {
        self.b.contains(x, y, z).then(|| self.idx(x, y, z))
    }

    /// Valori calcolati in tutte le celle del riquadro (coordinate della griglia).
    fn fill<T: Copy + Default + Send + Sync>(&self, f: impl Fn(usize, usize, usize) -> T + Sync) -> Vec<T> {
        let mut out = vec![T::default(); self.len()];
        let (lo, d) = (self.b.lo, self.d);
        out.par_chunks_mut(d[0]).enumerate().for_each(|(row, chunk)| {
            let (y, z) = (lo[1] + row / d[2], lo[2] + row % d[2]);
            for (k, o) in chunk.iter_mut().enumerate() {
                *o = f(lo[0] + k, y, z);
            }
        });
        out
    }

    /// Righe (y, z) del riquadro.
    fn rows(&self) -> Vec<(usize, usize)> {
        self.b.rows(None)
    }
}

/// Celle del riquadro entro `r` celle (in ogni direzione, massimo separabile) da una cella con
/// nebbia. Una passata per asse con una finestra scorrevole di conteggi (costo costante per cella
/// qualunque sia `r`); lungo y e z si fanno scorrere righe intere di x, così la memoria si legge
/// in ordine invece che a salti.
fn near_fog(g: &GridSpec, l: &Local, rho: &[f32], r: usize) -> Vec<u8> {
    dilate_mask(l, l.fill(|x, y, z| (rho[g.index(x, y, z)] > 1e-6) as u8), r)
}

/// Maschera 0/1 del riquadro allargata di `r` celle (vedi [`near_fog`]).
fn dilate_mask(l: &Local, a: Vec<u8>, r: usize) -> Vec<u8> {
    let [d0, d1, d2] = l.d;
    // Asse x: ogni riga per conto suo.
    let mut b = vec![0u8; a.len()];
    b.par_chunks_mut(d0)
        .zip(a.par_chunks(d0))
        .for_each(|(out, inp)| dilate(inp, out, d0, 1, r));
    // Asse z: in ogni strato y, finestra sulle righe z (ogni riga è una fila di x contigue).
    let plane = d0 * d2;
    let mut c = vec![0u8; a.len()];
    c.par_chunks_mut(plane)
        .zip(b.par_chunks(plane))
        .for_each(|(out, inp)| dilate(inp, out, d2, d0, r));
    // Asse y: finestra sugli strati, a blocchi di colonne (x, z) contigue.
    const COLS: usize = 4096;
    let out = Shared8(b.as_mut_ptr());
    (0..plane.div_ceil(COLS)).into_par_iter().for_each(|k| {
        let (s, e) = (k * COLS, ((k + 1) * COLS).min(plane));
        let w = e - s;
        let mut cnt = vec![0u16; w];
        let add = |cnt: &mut [u16], y: usize, sign: i32| {
            for (j, v) in cnt.iter_mut().enumerate() {
                *v = (*v as i32 + sign * c[y * plane + s + j] as i32) as u16;
            }
        };
        for y in 0..=r.min(d1 - 1) {
            add(&mut cnt, y, 1);
        }
        for y in 0..d1 {
            for (j, v) in cnt.iter().enumerate() {
                // SAFETY: ogni blocco di colonne scrive solo le sue celle.
                unsafe { *out.at(y * plane + s + j) = (*v > 0) as u8 };
            }
            if y + r + 1 < d1 {
                add(&mut cnt, y + r + 1, 1);
            }
            if y >= r {
                add(&mut cnt, y - r, -1);
            }
        }
    });
    b
}

/// Dilatazione di raggio `r` lungo un asse: `inp` e `out` sono `n` file di `w` valori (0/1)
/// consecutive; il valore di uscita di una fila è 1 dove almeno una fila entro `r` vale 1.
fn dilate(inp: &[u8], out: &mut [u8], n: usize, w: usize, r: usize) {
    let mut cnt = vec![0u16; w];
    let add = |cnt: &mut [u16], k: usize, sign: i32| {
        for (j, v) in cnt.iter_mut().enumerate() {
            *v = (*v as i32 + sign * inp[k * w + j] as i32) as u16;
        }
    };
    for k in 0..=r.min(n - 1) {
        add(&mut cnt, k, 1);
    }
    for k in 0..n {
        for (j, v) in cnt.iter().enumerate() {
            out[k * w + j] = (*v > 0) as u8;
        }
        if k + r + 1 < n {
            add(&mut cnt, k + r + 1, 1);
        }
        if k >= r {
            add(&mut cnt, k - r, -1);
        }
    }
}

/// Riquadro delle celle con nebbia allargato di `margin` celle (None se non c'è nebbia). Si
/// cerca solo in `search` quando è noto dove può essere la nebbia (il riquadro del passo prima:
/// in un passo la nebbia si sposta meno del margine).
fn fog_box(g: &GridSpec, rho: &[f32], search: Option<Bx>, margin: usize) -> Option<Bx> {
    let [nx, ny, nz] = g.res;
    let s = search.unwrap_or(Bx {
        lo: [0; 3],
        hi: [nx - 1, ny - 1, nz - 1],
    });
    let (lo, hi) = s
        .rows(None)
        .into_par_iter()
        .filter_map(|(y, z)| {
            let row = &rho[g.index(s.lo[0], y, z)..=g.index(s.hi[0], y, z)];
            let first = row.iter().position(|v| *v > 1e-6)? + s.lo[0];
            let last = row.iter().rposition(|v| *v > 1e-6)? + s.lo[0];
            Some(([first, y, z], [last, y, z]))
        })
        .reduce_with(|a, b| ([0, 1, 2].map(|k| a.0[k].min(b.0[k])), [0, 1, 2].map(|k| a.1[k].max(b.1[k]))))?;
    Some(Bx {
        lo: [0, 1, 2].map(|k| lo[k].saturating_sub(margin)),
        hi: [0, 1, 2].map(|k| (hi[k] + margin).min(g.res[k] - 1)),
    })
}

/// Azzera, nel campo di dimensioni `d`, le celle del riquadro `prev` (allargato di `grow`) che non
/// sono nel riquadro `cur`: fuori dal riquadro i campi restano a zero (aria ferma, p = 0), anche
/// quando la nebbia si ritira.
fn clear_outside(data: &mut [f32], d: [usize; 3], prev: Bx, cur: Option<Bx>, grow: usize) {
    let ptr = Shared(data.as_mut_ptr());
    let hi = [0, 1, 2].map(|k| (prev.hi[k] + grow).min(d[k] - 1));
    let lo = [0, 1, 2].map(|k| prev.lo[k].saturating_sub(grow));
    let keep = cur.map(|c| Bx {
        lo: [0, 1, 2].map(|k| c.lo[k].saturating_sub(grow)),
        hi: [0, 1, 2].map(|k| (c.hi[k] + grow).min(d[k] - 1)),
    });
    let rows: Vec<(usize, usize)> = (lo[1]..=hi[1]).flat_map(|y| (lo[2]..=hi[2]).map(move |z| (y, z))).collect();
    rows.par_iter().for_each(|&(y, z)| {
        for x in lo[0]..=hi[0] {
            if keep.is_some_and(|k| k.contains(x, y, z)) {
                continue;
            }
            // SAFETY: ogni riga è scritta da un solo thread.
            unsafe { *ptr.at(Mac::idx(d, x, y, z)) = 0.0 };
        }
    });
}

/// Tratto di una riga di celle: (y, z, prima x, ultima x).
type Span = (usize, usize, usize, usize);

/// Puntatore condiviso a un campo qualsiasi, per scrivere in parallelo righe disgiunte.
#[derive(Clone, Copy)]
struct SharedT<T>(*mut T);
unsafe impl<T> Send for SharedT<T> {}
unsafe impl<T> Sync for SharedT<T> {}

impl<T> SharedT<T> {
    #[inline]
    fn at(self, i: usize) -> *mut T {
        // SAFETY: `i` è un indice valido del buffer da cui è stato creato il puntatore.
        unsafe { self.0.add(i) }
    }
}

/// Tratti delle righe del riquadro che contengono celle di `mask` (dalla prima all'ultima).
fn spans_of(l: &Local, mask: &[u8]) -> Vec<Span> {
    let (lo, d) = (l.b.lo, l.d);
    mask.par_chunks(d[0])
        .enumerate()
        .filter_map(|(row, m)| {
            let first = m.iter().position(|v| *v != 0)?;
            let last = m.iter().rposition(|v| *v != 0)?;
            Some((lo[1] + row / d[2], lo[2] + row % d[2], lo[0] + first, lo[0] + last))
        })
        .collect()
}

/// Righe delle facce del campo `a` che toccano le celle dei tratti: la faccia tra una cella e
/// quella prima lungo `a` ha l'indice della cella, quindi servono anche le righe spostate di uno.
fn face_rows(spans: &[Span], a: usize) -> Vec<Span> {
    if a == 0 {
        return spans.iter().map(|&(y, z, x0, x1)| (y, z, x0, x1 + 1)).collect();
    }
    let shift = |&(y, z, x0, x1): &Span| if a == 1 { (y + 1, z, x0, x1) } else { (y, z + 1, x0, x1) };
    let mut all: Vec<Span> = spans.iter().copied().chain(spans.iter().map(shift)).collect();
    all.sort_unstable_by_key(|s| (s.0, s.1));
    // Una riga sola per (y, z): due thread non scrivono mai la stessa faccia.
    let mut out: Vec<Span> = Vec::with_capacity(all.len());
    for s in all {
        match out.last_mut() {
            Some(l) if l.0 == s.0 && l.1 == s.1 => {
                l.2 = l.2.min(s.2);
                l.3 = l.3.max(s.3);
            }
            _ => out.push(s),
        }
    }
    out
}

/// Aggiorna le facce del campo `a` sulle righe `rows` (righe tutte diverse).
fn update_rows(m: &Mac, a: usize, rows: &[Span], data: &mut [f32], f: impl Fn(usize, usize, usize, usize, &mut f32) + Sync) {
    let d = m.dims(Some(a));
    let ptr = Shared(data.as_mut_ptr());
    rows.par_iter().for_each(|&(y, z, x0, x1)| {
        for x in x0..=x1.min(d[0] - 1) {
            let i = Mac::idx(d, x, y, z);
            // SAFETY: le righe sono diverse, ogni faccia è scritta da un solo thread.
            f(i, x, y, z, unsafe { &mut *ptr.at(i) });
        }
    });
}

/// Come [`update_rows`], e restituisce il modulo massimo delle facce aggiornate.
fn update_rows_max(m: &Mac, a: usize, rows: &[Span], data: &mut [f32], f: impl Fn(usize, usize, usize, &mut f32) + Sync) -> f32 {
    let d = m.dims(Some(a));
    let ptr = Shared(data.as_mut_ptr());
    rows.par_iter()
        .map(|&(y, z, x0, x1)| {
            let mut vm = 0.0f32;
            for x in x0..=x1.min(d[0] - 1) {
                // SAFETY: le righe sono diverse, ogni faccia è scritta da un solo thread.
                let v = unsafe { &mut *ptr.at(Mac::idx(d, x, y, z)) };
                f(x, y, z, v);
                vm = vm.max(v.abs());
            }
            vm
        })
        .reduce(|| 0.0, f32::max)
}

impl Local {
    /// Come [`Local::fill`], ma solo nelle celle dei tratti, nel buffer `out` riusato da un passo
    /// all'altro (allungato se serve, non azzerato: fuori dai tratti i valori non si leggono mai).
    fn fill_spans<T: Copy + Default + Send + Sync>(
        &self,
        spans: &[Span],
        mut out: Vec<T>,
        f: impl Fn(usize, usize, usize) -> T + Sync,
    ) -> Vec<T> {
        if out.len() < self.len() {
            out.resize(self.len(), T::default());
        }
        let ptr = SharedT(out.as_mut_ptr());
        spans.par_iter().for_each(|&(y, z, x0, x1)| {
            for x in x0..=x1 {
                // SAFETY: un tratto per riga, ogni cella è scritta da un solo thread.
                unsafe { *ptr.at(self.idx(x, y, z)) = f(x, y, z) };
            }
        });
        out
    }
}

/// Copia in `dst` la parte di `src` (campo di dimensioni `d`) nel riquadro allargato di `grow`.
fn copy_region(src: &[f32], dst: &mut [f32], d: [usize; 3], b: Bx, grow: usize) {
    let lo = [0, 1, 2].map(|k| b.lo[k].saturating_sub(grow));
    let hi = [0, 1, 2].map(|k| (b.hi[k] + grow).min(d[k] - 1));
    dst.par_chunks_mut(d[0]).enumerate().for_each(|(row, chunk)| {
        let (y, z) = (row / d[2], row % d[2]);
        if y < lo[1] || y > hi[1] || z < lo[2] || z > hi[2] {
            return;
        }
        let start = row * d[0];
        chunk[lo[0]..=hi[0]].copy_from_slice(&src[start + lo[0]..=start + hi[0]]);
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

    // Vicini di ogni cella per la pressione, in 2 byte: quali dei 6 vicini sono celle libere
    // dentro la griglia (bit, nell'ordine di `NB_DIRS`) e quante facce non sono solide (le facce
    // verso l'aria fuori dalla griglia contano, con p = 0). La pressione legge questi dati 32
    // volte per passo: una tabella di indici (28 byte per cella) costava soprattutto memoria.
    const NB_DIRS: [(i64, i64, i64); 6] = [(1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)];
    let nb_offset: [isize; 6] = NB_DIRS.map(|(dx, dy, dz)| dx as isize + (dz as isize) * nx as isize + (dy as isize) * (nx * nz) as isize);
    let neighbors: Vec<[u8; 2]> = (0..n)
        .into_par_iter()
        .map(|i| {
            if inp.solid[i] {
                return [0, 0];
            }
            let (x, y, z) = g.coords(i);
            let (xi, yi, zi) = (x as i64, y as i64, z as i64);
            let (mut mask, mut k) = (0u8, 0u8);
            for (bit, (dx, dy, dz)) in NB_DIRS.into_iter().enumerate() {
                let (qx, qy, qz) = (xi + dx, yi + dy, zi + dz);
                if solid(qx, qy, qz) {
                    continue;
                }
                k += 1;
                if qx >= 0 && qz >= 0 && qx < nx as i64 && qy < ny as i64 && qz < nz as i64 {
                    mask |= 1 << bit;
                }
            }
            [mask, k]
        })
        .collect();

    // Passo temporale: 1/30 s, ridotto se le velocità superano MAX_CFL celle per passo.
    // Fuori dal riquadro della nebbia tutti i campi restano a zero (aria ferma): ogni passo
    // lavora e alloca solo nel riquadro, e i buffer grandi come la griglia si riusano.
    let mut clock = Clock::new();
    let mut t = 0.0f32;
    let mut steps = 0u32;
    let mut last_dt = 1.0 / 30.0;
    let mut prev_box: Option<Bx> = None;
    let mut old_c: [Vec<f32>; 3] = [0, 1, 2].map(|a| vec![0.0f32; m.dims(Some(a)).iter().product()]);
    let mut fa = vec![0.0f32; n];
    let mut fb = vec![0.0f32; n];
    let mut active_cells = 0u64;

    let mut next_vmax = 0.0f32;
    // Campi di lavoro dei passi, riusati (grandi quanto il riquadro più grande visto).
    let (mut b_cv, mut b_om, mut b_force) = (Vec::<Vec3>::new(), Vec::<Vec3>::new(), Vec::<Vec3>::new());
    let (mut b_div, mut b_fwd, mut b_out) = (Vec::<f32>::new(), Vec::<S2>::new(), Vec::<[f32; 2]>::new());
    let profile = std::env::var_os("FORGE_FOG_PROFILE").is_some();
    let (mut near_cells, mut domain_cells) = (0u64, 0u64);
    while t < inp.sim_time - 1e-5 && !crate::progress::cancelled() {
        // Velocità massima sulle facce del dominio del passo prima (entro il margine dalla
        // nebbia: lì si muove l'aria che conta), misurata alla fine di quel passo.
        let vmax = next_vmax;
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
        // nebbia non può arrivarci in un passo (spostamento < 1 cella). La nebbia si cerca solo
        // nel riquadro di prima (in un passo si sposta meno del margine).
        let found = fog_box(&g, &rho, prev_box, WORK_MARGIN);
        if let Some(pb) = prev_box {
            // La nebbia si è ritirata: le celle lasciate tornano aria ferma.
            for a in 0..3 {
                clear_outside(&mut vel.c[a], m.dims(Some(a)), pb, found, 1);
            }
            clear_outside(&mut pressure, g.res, pb, found, 0);
            clear_outside(&mut fa, g.res, pb, found, 0);
            clear_outside(&mut fb, g.res, pb, found, 0);
        }
        prev_box = found;
        let Some(bx) = found else {
            t += dt;
            steps += 1;
            continue;
        };
        let loc = Local::new(bx);
        active_cells += loc.len() as u64;

        clock.lap(1);
        // Celle vicine alla nebbia (entro 3 celle): solo lì servono trasporto e forze.
        let near = near_fog(&g, &loc, &rho, 3);
        // Dominio: celle entro il margine dalla nebbia, per righe. Forze e trasporti agiscono
        // solo vicino alla nebbia e la pressione si risolve qui (oltre, p = 0: aria aperta),
        // invece che in tutto il riquadro, che con colate e tappeti è quasi tutto aria vuota.
        let domain = dilate_mask(&loc, near.clone(), WORK_MARGIN - 3);
        let spans = spans_of(&loc, &domain);
        let faces: [Vec<Span>; 3] = [0, 1, 2].map(|a| face_rows(&spans, a));
        if profile {
            near_cells += near.iter().filter(|v| **v != 0).count() as u64;
            domain_cells += domain.iter().filter(|v| **v != 0).count() as u64;
        }
        let near_at = |x: usize, y: usize, z: usize| loc.get(x, y, z).is_some_and(|j| near[j] != 0);
        let clamp_g = |v: i64, a: usize| v.clamp(0, g.res[a] as i64 - 1) as usize;

        // 2. Forze (al centro delle celle, poi riportate sulle facce).
        let phase = 0.5 + 0.5 * (t * std::f32::consts::TAU / 6.0).sin();
        let center_vel: Vec<Vec3> = loc.fill_spans(&spans, std::mem::take(&mut b_cv), |x, y, z| {
            if !near_at(x, y, z) {
                return Vec3::ZERO;
            }
            let a = |c: usize, dx: usize, dy: usize, dz: usize| vel.c[c][Mac::idx(m.dims(Some(c)), x + dx, y + dy, z + dz)];
            Vec3::new(
                0.5 * (a(0, 0, 0, 0) + a(0, 1, 0, 0)),
                0.5 * (a(1, 0, 0, 0) + a(1, 0, 1, 0)),
                0.5 * (a(2, 0, 0, 0) + a(2, 0, 0, 1)),
            )
        });
        // Valore di un campo del riquadro nella cella (x, y, z), portata dentro la griglia;
        // fuori dal riquadro zero (come prima, quando i campi erano grandi come la griglia).
        let at_loc = |field: &[Vec3], x: i64, y: i64, z: i64| -> Vec3 {
            loc.get(clamp_g(x, 0), clamp_g(y, 1), clamp_g(z, 2))
                .map_or(Vec3::ZERO, |j| field[j])
        };
        let omega: Vec<Vec3> = loc.fill_spans(&spans, std::mem::take(&mut b_om), |x, y, z| {
            if !near_at(x, y, z) {
                return Vec3::ZERO;
            }
            let (x, y, z) = (x as i64, y as i64, z as i64);
            let d = |dx: i64, dy: i64, dz: i64| {
                (at_loc(&center_vel, x + dx, y + dy, z + dz) - at_loc(&center_vel, x - dx, y - dy, z - dz)) / (2.0 * h)
            };
            let (dx, dy, dz) = (d(1, 0, 0), d(0, 1, 0), d(0, 0, 1));
            Vec3::new(dy.z - dz.y, dz.x - dx.z, dx.y - dy.x)
        });
        let force: Vec<Vec3> = loc.fill_spans(&spans, std::mem::take(&mut b_force), |x, y, z| {
            let i = g.index(x, y, z);
            if inp.solid[i] || rho[i] <= 0.0 {
                return Vec3::ZERO;
            }
            let mut f = Vec3::new(0.0, G_UP * rise * rho[i] - G_DOWN * heavy[i], 0.0);
            // Turbolenza solo dove c'è nebbia (l'aria ferma resta ferma).
            let presence = (rho[i] / 0.05).min(1.0);
            f += (turb[0][i] * phase + turb[1][i] * (1.0 - phase)) * (k_turb * presence);
            // Confinamento della vorticità: N × ω, con N = ∇|ω| normalizzato.
            if eps > 0.0 && presence > 0.0 {
                let (xi, yi, zi) = (x as i64, y as i64, z as i64);
                let mag = |dx: i64, dy: i64, dz: i64| at_loc(&omega, xi + dx, yi + dy, zi + dz).length();
                let grad = Vec3::new(
                    mag(1, 0, 0) - mag(-1, 0, 0),
                    mag(0, 1, 0) - mag(0, -1, 0),
                    mag(0, 0, 1) - mag(0, 0, -1),
                );
                let nrm = grad.normalize_or_zero();
                f += nrm.cross(omega[loc.idx(x, y, z)]) * (eps * h * presence);
            }
            f
        });
        let damp = (-DRAG * dt).exp();
        let wall_damp = (-(DRAG + WALL_DRAG * (BOUNDARY_LAYER / h).min(1.0)) * dt).exp();
        for a in 0..3 {
            update_rows(&m, a, &faces[a], &mut vel.c[a], |_, x, y, z, v| {
                let mut lo = [x as i64, y as i64, z as i64];
                lo[a] -= 1;
                let hi = [x as i64, y as i64, z as i64];
                if solid(lo[0], lo[1], lo[2]) || solid(hi[0], hi[1], hi[2]) {
                    *v = 0.0;
                    return;
                }
                // Lontano dalla nebbia nessuna forza (l'aria ferma resta ferma).
                if !near_at(x.min(nx - 1), y.min(ny - 1), z.min(nz - 1)) {
                    return;
                }
                let fa = |c: [i64; 3]| {
                    if c[0] < 0 || c[1] < 0 || c[2] < 0 || c[0] >= nx as i64 || c[1] >= ny as i64 || c[2] >= nz as i64 {
                        0.0
                    } else {
                        loc.get(c[0] as usize, c[1] as usize, c[2] as usize).map_or(0.0, |j| force[j][a])
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
        // 3. Trasporto della velocità: si legge la velocità di prima, copiata solo attorno al
        // riquadro (il trasporto non pesca oltre MAX_CFL celle).
        for (a, dst) in old_c.iter_mut().enumerate() {
            copy_region(&vel.c[a], dst, m.dims(Some(a)), bx, 3);
        }
        let old = Vel {
            c: std::mem::take(&mut old_c),
        };
        for a in 0..3 {
            let d = m.dims(Some(a));
            update_rows(&m, a, &faces[a], &mut vel.c[a], |_, x, y, z, v| {
                let mut lo = [x as i64, y as i64, z as i64];
                lo[a] -= 1;
                if solid(lo[0], lo[1], lo[2]) || solid(x as i64, y as i64, z as i64) {
                    *v = 0.0;
                    return;
                }
                // Lontano dalla nebbia l'aria si muove poco: il trasporto si salta.
                if !near_at(x.min(nx - 1), y.min(ny - 1), z.min(nz - 1)) {
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
        old_c = old.c;

        clock.lap(3);
        // 4. Proiezione (pressione), nel riquadro; fuori p = 0 (aria ferma, bordo aperto).
        let mut div: Vec<f32> = loc.fill_spans(&spans, std::mem::take(&mut b_div), |x, y, z| {
            if inp.solid[g.index(x, y, z)] {
                return 0.0;
            }
            let f = |a: usize, dx: usize, dy: usize, dz: usize| vel.c[a][Mac::idx(m.dims(Some(a)), x + dx, y + dy, z + dz)];
            (f(0, 1, 0, 0) - f(0, 0, 0, 0) + f(1, 0, 1, 0) - f(1, 0, 0, 0) + f(2, 0, 0, 1) - f(2, 0, 0, 0)) / h
        });
        // Il gas prodotto occupa volume: divergenza imposta nelle celle della sorgente.
        for &i in &source {
            let (x, y, z) = g.coords(i);
            if let Some(j) = loc.get(x, y, z) {
                div[j] -= src_div;
            }
        }
        let rhs_scale = h * h / dt;
        // Fuori dal dominio p = 0 (aria aperta): si azzerano le celle del riquadro fuori dai
        // tratti, così chi ne esce non tiene una pressione vecchia.
        {
            let mut span_of = vec![None; loc.d[1] * loc.d[2]];
            for &(y, z, x0, x1) in &spans {
                span_of[(y - bx.lo[1]) * loc.d[2] + (z - bx.lo[2])] = Some((x0, x1));
            }
            let ptr = Shared(pressure.as_mut_ptr());
            loc.rows().par_iter().for_each(|&(y, z)| {
                let keep = span_of[(y - bx.lo[1]) * loc.d[2] + (z - bx.lo[2])];
                for x in bx.lo[0]..=bx.hi[0] {
                    if keep.is_some_and(|(x0, x1)| x >= x0 && x <= x1) {
                        continue;
                    }
                    // SAFETY: ogni riga è scritta da un solo thread.
                    unsafe { *ptr.at(g.index(x, y, z)) = 0.0 };
                }
            });
        }
        for _ in 0..PRESSURE_ITERS {
            for color in 0..2usize {
                // Gauss–Seidel rosso/nero sul posto: le celle di un colore leggono solo celle
                // dell'altro colore, quindi le scritture parallele non si sovrappongono e il
                // risultato non dipende dall'ordine.
                let ptr = Shared(pressure.as_mut_ptr());
                let (neighbors, div, nb_offset) = (&neighbors, &div, &nb_offset);
                spans.par_iter().for_each(|&(y, z, x0, x1)| {
                    let start = x0 + (x0 + y + z + color) % 2;
                    for x in (start..=x1).step_by(2) {
                        let i = g.index(x, y, z);
                        let [mask, k] = neighbors[i];
                        if k == 0 {
                            continue;
                        }
                        // SAFETY: `i` ha il colore corrente; i vicini letti hanno l'altro colore e
                        // in questa mezza iterazione nessun thread li scrive. Ogni `i` è scritto
                        // da un solo thread (una riga a testa).
                        unsafe {
                            // Stesso ordine dei vicini di prima: somma identica.
                            let mut sum = 0.0f32;
                            for (bit, off) in nb_offset.iter().enumerate() {
                                if mask & (1 << bit) != 0 {
                                    sum += *ptr.at(i.wrapping_add_signed(*off));
                                }
                            }
                            let cur = *ptr.at(i);
                            let gs = (sum - rhs_scale * div[loc.idx(x, y, z)]) / k as f32;
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
        // Ultimo aggiornamento delle velocità del passo: insieme si misura la velocità massima
        // che fissa il passo temporale successivo (sulle stesse facce del dominio).
        next_vmax = 0.0;
        for a in 0..3 {
            let vm = update_rows_max(&m, a, &faces[a], &mut vel.c[a], |x, y, z, v| {
                let mut lo = [x as i64, y as i64, z as i64];
                lo[a] -= 1;
                if solid(lo[0], lo[1], lo[2]) || solid(x as i64, y as i64, z as i64) {
                    *v = 0.0;
                    return;
                }
                *v -= dt / h * (p_at(x as i64, y as i64, z as i64) - p_at(lo[0], lo[1], lo[2]));
            });
            next_vmax = next_vmax.max(vm);
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
        let fwd: Vec<S2> = loc.fill_spans(&spans, std::mem::take(&mut b_fwd), |x, y, z| {
            let i = g.index(x, y, z);
            if inp.solid[i] || !near_at(x, y, z) {
                S2::default()
            } else {
                sl2(&rho, &heavy, i, x, y, z, 1.0)
            }
        });
        // Passo in avanti nei buffer della griglia (solo nel riquadro): il passo indietro li
        // campiona con l'interpolazione trilineare.
        {
            let (pa, pb) = (Shared(fa.as_mut_ptr()), Shared(fb.as_mut_ptr()));
            spans.par_iter().for_each(|&(y, z, x0, x1)| {
                for x in x0..=x1 {
                    let s = fwd[loc.idx(x, y, z)];
                    let i = g.index(x, y, z);
                    // SAFETY: ogni riga è scritta da un solo thread.
                    unsafe {
                        *pa.at(i) = s.v[0];
                        *pb.at(i) = s.v[1];
                    }
                }
            });
        }
        let out: Vec<[f32; 2]> = loc.fill_spans(&spans, std::mem::take(&mut b_out), |x, y, z| {
            let i = g.index(x, y, z);
            if inp.solid[i] || !near_at(x, y, z) {
                return [0.0; 2];
            }
            let s = fwd[loc.idx(x, y, z)];
            let back = sl2(&fa, &fb, i, x, y, z, -1.0);
            [
                (s.v[0] + 0.5 * (rho[i] - back.v[0])).clamp(s.lo[0], s.hi[0]).max(0.0),
                (s.v[1] + 0.5 * (heavy[i] - back.v[1])).clamp(s.lo[1], s.hi[1]).max(0.0),
            ]
        });

        clock.lap(5);
        // 6. Dissolvenza (scritta insieme al risultato del trasporto, solo nel riquadro).
        let fade_vis = |v: f32| {
            // Il velo sottile svanisce prima del corpo denso.
            let thin = 1.0 - (v / 0.15).min(1.0);
            v * (-(k_vis * (1.0 + 4.0 * thin)) * dt).exp()
        };
        let kh = (-k_heavy * dt).exp();
        {
            let (pr, ph) = (Shared(rho.as_mut_ptr()), Shared(heavy.as_mut_ptr()));
            spans.par_iter().for_each(|&(y, z, x0, x1)| {
                for x in x0..=x1 {
                    let [r, c] = out[loc.idx(x, y, z)];
                    let r = fade_vis(r);
                    let i = g.index(x, y, z);
                    // SAFETY: ogni riga è scritta da un solo thread.
                    unsafe {
                        *pr.at(i) = r;
                        *ph.at(i) = (c * kh).min(r);
                    }
                }
            });
        }

        (b_cv, b_om, b_force) = (center_vel, omega, force);
        (b_div, b_fwd, b_out) = (div, fwd, out);
        clock.lap(6);
        crate::progress::set(crate::progress::Phase::Simulation, (t + dt) / inp.sim_time.max(1e-6));
        t += dt;
        steps += 1;
    }
    // Celle lavorate in media per passo, come frazione della griglia.
    let active = active_cells as f64 / (n as u64 * steps.max(1) as u64) as f64;
    if std::env::var_os("FORGE_FOG_PROFILE").is_some() {
        eprintln!(
            "profilo nebbia: {steps} passi, riquadro medio {:.1}% della griglia, vicino alla nebbia {:.1}%, entro il margine {:.1}%, ms {:?}",
            active * 100.0,
            near_cells as f64 / (n as u64 * steps.max(1) as u64) as f64 * 100.0,
            domain_cells as f64 / (n as u64 * steps.max(1) as u64) as f64 * 100.0,
            clock.ms.map(|v| v.round())
        );
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
