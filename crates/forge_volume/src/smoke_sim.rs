//! Simulazione del fumo: fluido euleriano su griglia regolare con temperatura e galleggiamento
//! (Fedkiw, Stam, Jensen 2001, "Visual Simulation of Smoke").
//!
//! Campi al centro delle celle: velocità, densità del fumo, temperatura (sopra l'ambiente). Ogni
//! passo:
//! 1. sorgente: una sfera che emette fumo e calore con una velocità di uscita;
//! 2. forze: galleggiamento f = (β·T − α·ρ)·ŷ (il caldo sale, il fumo pesa), confinamento della
//!    vorticità (riccioli che la griglia smorzerebbe), turbolenza (curl noise) e vento;
//! 3. trasporto semi-lagrangiano di velocità, densità e temperatura;
//! 4. proiezione: pressione con Gauss–Seidel rosso/nero, così il fluido non si comprime;
//! 5. dissolvenza del fumo e raffreddamento.
//!
//! Gli oggetti sono celle solide (velocità nulla, niente fumo dentro). Il riquadro è aperto: il
//! fumo che esce si perde. Deterministico: stessi parametri e stesso seme, stesso fumo.

use crate::grid::GridSpec;
use crate::noise::curl;
use forge_math::Vec3;
use rayon::prelude::*;

/// Parametri della simulazione (già nei limiti).
#[derive(Clone, Debug)]
pub struct SmokeSimParams {
    pub spec: GridSpec,
    /// Centro e raggio della sorgente (m, spazio mondo).
    pub source: Vec3,
    pub source_radius: f32,
    /// Fumo emesso (densità aggiunta al secondo dentro la sorgente, 1 = pieno in un secondo).
    pub emission: f32,
    /// Temperatura della sorgente sopra l'ambiente (scala libera, 1 = fumo caldo tipico).
    pub temperature: f32,
    /// Spinta del caldo (m/s² per unità di temperatura).
    pub buoyancy: f32,
    /// Peso del fumo (m/s² per unità di densità): alto = gas pesante che scende.
    pub weight: f32,
    /// Velocità di uscita dalla sorgente (m/s, verso l'alto; negativa = verso il basso).
    pub exit_speed: f32,
    /// Raffreddamento (1/s).
    pub cooling: f32,
    /// Dissolvenza del fumo (1/s).
    pub dissipation: f32,
    /// Confinamento della vorticità (0 = nessuno).
    pub vorticity: f32,
    /// Turbolenza (m/s² di forza di curl noise) e la sua grandezza (m).
    pub turbulence: f32,
    pub turbulence_scale: f32,
    pub wind: Vec3,
    /// Secondi simulati (l'istante che si congela per il render).
    pub time: f32,
    pub seed: u32,
    /// Bordi sfumati: fascia vicino alle pareti e al soffitto (frazione del lato, 0 = nessuna)
    /// in cui fumo e calore si dissolvono in fretta, come se uscissero dal riquadro.
    pub edge_fade: f32,
    /// Riquadro del fumo: centro, rotazione attorno alla verticale (radianti) e mezze dimensioni nei suoi
    /// assi. La griglia copre il riquadro ruotato; fuori dal riquadro il fumo si perde.
    pub box_center: Vec3,
    pub box_yaw: f32,
    pub box_half: Vec3,
}

/// Risultato: densità (0–1 circa) al centro delle celle di `spec`.
#[derive(Clone, Debug)]
pub struct SmokeField {
    pub spec: GridSpec,
    pub density: Vec<f32>,
    /// Densità di riferimento (90° percentile delle celle con fumo): la forma si legge rispetto a lei.
    pub max: f32,
    pub steps: u32,
    pub ms: f64,
}

impl SmokeField {
    pub fn sample(&self, p: Vec3) -> f32 {
        self.spec.sample(&self.density, p)
    }
}

const PRESSURE_ITERS: usize = 40;

struct Grid {
    s: GridSpec,
}

impl Grid {
    #[inline]
    fn idx(&self, x: usize, y: usize, z: usize) -> usize {
        self.s.index(x, y, z)
    }
    /// Campione trilineare in coordinate di griglia (centri a i + 0,5); 0 fuori.
    fn sample(&self, f: &[f32], q: Vec3) -> f32 {
        let [rx, ry, rz] = self.s.res;
        if q.x < 0.0 || q.y < 0.0 || q.z < 0.0 || q.x > rx as f32 || q.y > ry as f32 || q.z > rz as f32 {
            return 0.0;
        }
        let q = q - Vec3::splat(0.5);
        let f0 = q.floor();
        let t = q - f0;
        let c = |v: f32, n: usize| (v as i64).clamp(0, n as i64 - 1) as usize;
        let (x0, y0, z0) = (c(f0.x, rx), c(f0.y, ry), c(f0.z, rz));
        let (x1, y1, z1) = (c(f0.x + 1.0, rx), c(f0.y + 1.0, ry), c(f0.z + 1.0, rz));
        let l = |a: f32, b: f32, t: f32| a + (b - a) * t;
        let a = l(f[self.idx(x0, y0, z0)], f[self.idx(x1, y0, z0)], t.x);
        let b = l(f[self.idx(x0, y1, z0)], f[self.idx(x1, y1, z0)], t.x);
        let cc = l(f[self.idx(x0, y0, z1)], f[self.idx(x1, y0, z1)], t.x);
        let d = l(f[self.idx(x0, y1, z1)], f[self.idx(x1, y1, z1)], t.x);
        l(l(a, b, t.y), l(cc, d, t.y), t.z)
    }
}

/// Simula fino a `p.time`. `solid` = celle occupate dagli oggetti (stessa griglia). `progress`
/// riceve la frazione fatta; `cancelled` ferma il calcolo (None).
pub fn simulate_smoke(
    p: &SmokeSimParams,
    solid: Option<&[bool]>,
    progress: &dyn Fn(f32),
    cancelled: &dyn Fn() -> bool,
) -> Option<SmokeField> {
    run(p, solid, progress, cancelled, None)
}

/// Come [`simulate_smoke`], e in più consegna il fumo ogni `every` secondi simulati (e
/// all'istante finale) a `on_frame(tempo, densità)`: serve all'anteprima nella viewport, che
/// mostra come si muove il fumo nel tempo.
pub fn simulate_smoke_frames(
    p: &SmokeSimParams,
    solid: Option<&[bool]>,
    every: f32,
    on_frame: &mut dyn FnMut(f32, &[f32]),
    cancelled: &dyn Fn() -> bool,
) -> Option<SmokeField> {
    run(p, solid, &|_| {}, cancelled, Some((every.max(1e-3), on_frame)))
}

/// Destinatario dei fotogrammi di [`simulate_smoke_frames`]: (tempo, densità).
type FrameSink<'a> = &'a mut dyn FnMut(f32, &[f32]);

fn run(
    p: &SmokeSimParams,
    solid: Option<&[bool]>,
    progress: &dyn Fn(f32),
    cancelled: &dyn Fn() -> bool,
    mut frames: Option<(f32, FrameSink<'_>)>,
) -> Option<SmokeField> {
    let t0 = std::time::Instant::now();
    let g = Grid { s: p.spec };
    let [rx, ry, rz] = p.spec.res;
    let n = p.spec.len();
    let h = p.spec.cell;
    let solid: Vec<bool> = solid.filter(|s| s.len() == n).map(|s| s.to_vec()).unwrap_or_else(|| vec![false; n]);
    let (mut u, mut v, mut w) = (vec![0.0f32; n], vec![0.0f32; n], vec![0.0f32; n]);
    let (mut rho, mut temp) = (vec![0.0f32; n], vec![0.0f32; n]);
    let mut pres = vec![0.0f32; n];
    // Passo: la velocità tipica (spinta, uscita, vento) attraversa al più ~2 celle per passo.
    let v_typ = (p.buoyancy * p.temperature)
        .sqrt()
        .max(p.exit_speed.abs())
        .max(p.wind.length())
        .max(0.2)
        * 1.5;
    let dt = (2.0 * h / v_typ).clamp(1.0 / 240.0, 1.0 / 24.0);
    let steps = ((p.time / dt).ceil() as u32).max(1);
    let dt = p.time / steps as f32;
    let src_r2 = p.source_radius.max(h * 0.75).powi(2);
    let coords = |i: usize| p.spec.coords(i);
    // Bordi sfumati: in ogni cella quanto fumo resta a ogni passo (1 lontano dalle pareti; sulla
    // parete si dissolve in circa un decimo di secondo, fuori dal riquadro subito).
    // La profondità della fascia cambia da punto a punto con un frattale: il fumo si sfilaccia
    // in lingue e buchi invece di svanire tutto alla stessa distanza dalla parete.
    let fade = if p.edge_fade.is_finite() {
        p.edge_fade.clamp(0.0, 0.5)
    } else {
        0.0
    };
    let longest = (rx.max(ry).max(rz) as f32 * h).max(1e-4);
    // Distanza dalle pareti e dal soffitto del riquadro in frazione del lato (negativa fuori).
    // Senza rotazione il riquadro è la griglia: si usano gli indici (bordo esatto delle celle).
    let rotated = p.box_yaw.is_finite() && p.box_yaw.abs() > 1e-5;
    let (ys, yc) = if rotated { p.box_yaw.sin_cos() } else { (0.0, 1.0) };
    let mid = p.box_center;
    let half = p.box_half.max(Vec3::splat(1e-4));
    let wall = |x: usize, y: usize, z: usize| -> f32 {
        let top = ((ry - 1 - y) as f32 + 0.5) / ry as f32;
        if !rotated {
            let side = |a: usize, m: usize| (a.min(m - 1 - a) as f32 + 0.5) / m as f32;
            return side(x, rx).min(side(z, rz)).min(top);
        }
        let r = p.spec.center(x, y, z) - mid;
        // Nel riquadro: rotazione inversa attorno alla verticale.
        let lx = yc * r.x - ys * r.z;
        let lz = ys * r.x + yc * r.z;
        ((half.x - lx.abs()) / (2.0 * half.x))
            .min((half.z - lz.abs()) / (2.0 * half.z))
            .min(top)
    };
    let edge: Vec<f32> = (0..n)
        .into_par_iter()
        .map(|i| {
            let (x, y, z) = coords(i);
            let d = wall(x, y, z);
            if d < 0.0 {
                return 0.0;
            }
            if fade <= 0.0 || d >= fade * 1.6 {
                return 1.0;
            }
            let c = p.spec.center(x, y, z) / (longest * fade * 0.6);
            let depth = fade * (1.0 + 0.6 * crate::noise::fbm(c, p.seed ^ 0x0ed9_e5a1, 4)).max(0.15);
            let t = (1.0 - d / depth).clamp(0.0, 1.0);
            (-25.0 * t * t * dt).exp()
        })
        .collect();
    for step in 0..steps {
        if cancelled() {
            return None;
        }
        progress(step as f32 / steps as f32);
        let time = step as f32 * dt;
        // 1. Sorgente (con un leggero sfarfallio per rompere la simmetria).
        let flicker = 0.85 + 0.15 * (time * 7.3 + p.seed as f32).sin();
        rho.par_iter_mut()
            .zip(temp.par_iter_mut())
            .zip(v.par_iter_mut())
            .enumerate()
            .for_each(|(i, ((r, t), vy))| {
                let (x, y, z) = coords(i);
                let c = p.spec.center(x, y, z);
                let d2 = (c - p.source).length_squared();
                if d2 < src_r2 && !solid[i] {
                    let k = 1.0 - d2 / src_r2;
                    *r = (*r + p.emission * flicker * k * dt).min(1.0);
                    *t = t.max(p.temperature * k);
                    *vy = vy.max(p.exit_speed * k).min(p.exit_speed.max(*vy));
                    if p.exit_speed < 0.0 {
                        *vy = vy.min(p.exit_speed * k);
                    }
                }
            });
        // 2. Forze: galleggiamento, turbolenza, vento (verso la sua velocità), vorticità.
        let omega: Vec<Vec3> = if p.vorticity > 0.0 {
            (0..n)
                .into_par_iter()
                .map(|i| {
                    let (x, y, z) = coords(i);
                    let at = |f: &[f32], dx: i64, dy: i64, dz: i64| {
                        let c = |a: usize, d: i64, m: usize| (a as i64 + d).clamp(0, m as i64 - 1) as usize;
                        f[g.idx(c(x, dx, rx), c(y, dy, ry), c(z, dz, rz))]
                    };
                    let dwdy = (at(&w, 0, 1, 0) - at(&w, 0, -1, 0)) / (2.0 * h);
                    let dvdz = (at(&v, 0, 0, 1) - at(&v, 0, 0, -1)) / (2.0 * h);
                    let dudz = (at(&u, 0, 0, 1) - at(&u, 0, 0, -1)) / (2.0 * h);
                    let dwdx = (at(&w, 1, 0, 0) - at(&w, -1, 0, 0)) / (2.0 * h);
                    let dvdx = (at(&v, 1, 0, 0) - at(&v, -1, 0, 0)) / (2.0 * h);
                    let dudy = (at(&u, 0, 1, 0) - at(&u, 0, -1, 0)) / (2.0 * h);
                    Vec3::new(dwdy - dvdz, dudz - dwdx, dvdx - dudy)
                })
                .collect()
        } else {
            Vec::new()
        };
        let ts = p.turbulence_scale.max(h * 2.0);
        let tt = time * 0.35;
        let forces: Vec<Vec3> = (0..n)
            .into_par_iter()
            .map(|i| {
                if solid[i] {
                    return Vec3::ZERO;
                }
                let (x, y, z) = coords(i);
                let mut f = Vec3::new(0.0, p.buoyancy * temp[i] - p.weight * rho[i], 0.0);
                // Turbolenza solo dove c'è fumo (non agita l'aria vuota, più veloce).
                if p.turbulence > 0.0 && rho[i] > 1e-3 {
                    let c = p.spec.center(x, y, z);
                    f += curl(c / ts + Vec3::new(0.0, -tt, tt * 0.5), p.seed) * p.turbulence;
                }
                if p.wind != Vec3::ZERO {
                    f += (p.wind - Vec3::new(u[i], v[i], w[i])) * 0.8;
                }
                if p.vorticity > 0.0 {
                    let mag = |dx: i64, dy: i64, dz: i64| {
                        let c = |a: usize, d: i64, m: usize| (a as i64 + d).clamp(0, m as i64 - 1) as usize;
                        omega[g.idx(c(x, dx, rx), c(y, dy, ry), c(z, dz, rz))].length()
                    };
                    let grad = Vec3::new(
                        mag(1, 0, 0) - mag(-1, 0, 0),
                        mag(0, 1, 0) - mag(0, -1, 0),
                        mag(0, 0, 1) - mag(0, 0, -1),
                    );
                    let len = grad.length();
                    if len > 1e-9 {
                        f += (grad / len).cross(omega[i]) * (p.vorticity * h);
                    }
                }
                f
            })
            .collect();
        u.par_iter_mut().zip(&forces).for_each(|(a, f)| *a += f.x * dt);
        v.par_iter_mut().zip(&forces).for_each(|(a, f)| *a += f.y * dt);
        w.par_iter_mut().zip(&forces).for_each(|(a, f)| *a += f.z * dt);
        // 3. Trasporto della velocità (semi-lagrangiano, all'indietro lungo la velocità).
        let back = |i: usize, u: &[f32], v: &[f32], w: &[f32]| {
            let (x, y, z) = coords(i);
            Vec3::new(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5) - Vec3::new(u[i], v[i], w[i]) * (dt / h)
        };
        let (nu, (nv, nw)): (Vec<f32>, (Vec<f32>, Vec<f32>)) = (0..n)
            .into_par_iter()
            .map(|i| {
                if solid[i] {
                    return (0.0, (0.0, 0.0));
                }
                let q = back(i, &u, &v, &w);
                (g.sample(&u, q), (g.sample(&v, q), g.sample(&w, q)))
            })
            .unzip();
        u = nu;
        v = nv;
        w = nw;
        // 4. Proiezione: divergenza, pressione (Gauss–Seidel rosso/nero), gradiente.
        let div: Vec<f32> = (0..n)
            .into_par_iter()
            .map(|i| {
                if solid[i] {
                    return 0.0;
                }
                let (x, y, z) = coords(i);
                // Bordo del riquadro aperto (velocità estesa); oggetti: velocità nulla.
                let get = |f: &[f32], xx: i64, yy: i64, zz: i64| {
                    let c = |a: i64, m: usize| a.clamp(0, m as i64 - 1) as usize;
                    let j = g.idx(c(xx, rx), c(yy, ry), c(zz, rz));
                    if solid[j] {
                        0.0
                    } else {
                        f[j]
                    }
                };
                let (xi, yi, zi) = (x as i64, y as i64, z as i64);
                ((get(&u, xi + 1, yi, zi) - get(&u, xi - 1, yi, zi))
                    + (get(&v, xi, yi + 1, zi) - get(&v, xi, yi - 1, zi))
                    + (get(&w, xi, yi, zi + 1) - get(&w, xi, yi, zi - 1)))
                    / (2.0 * h)
            })
            .collect();
        for _ in 0..PRESSURE_ITERS {
            for color in 0..2usize {
                let shared = SharedPtr(pres.as_mut_ptr());
                (0..ry).into_par_iter().for_each(|y| {
                    let pr = shared.get();
                    for z in 0..rz {
                        for x in 0..rx {
                            if (x + y + z) % 2 != color {
                                continue;
                            }
                            let i = g.idx(x, y, z);
                            if solid[i] {
                                continue;
                            }
                            // Fuori dal riquadro pressione 0 (aperto); negli oggetti come la cella (Neumann).
                            let nb = |xx: i64, yy: i64, zz: i64| -> f32 {
                                if xx < 0 || yy < 0 || zz < 0 || xx >= rx as i64 || yy >= ry as i64 || zz >= rz as i64 {
                                    return 0.0;
                                }
                                let j = g.idx(xx as usize, yy as usize, zz as usize);
                                // SAFETY: lettura di celle dell'altro colore, non scritte in questo mezzo passo.
                                unsafe {
                                    if solid[j] {
                                        *pr.add(i)
                                    } else {
                                        *pr.add(j)
                                    }
                                }
                            };
                            let (xi, yi, zi) = (x as i64, y as i64, z as i64);
                            let s = nb(xi - 1, yi, zi)
                                + nb(xi + 1, yi, zi)
                                + nb(xi, yi - 1, zi)
                                + nb(xi, yi + 1, zi)
                                + nb(xi, yi, zi - 1)
                                + nb(xi, yi, zi + 1);
                            // SAFETY: ogni cella di questo colore la scrive un solo thread (la sua riga y).
                            unsafe {
                                *pr.add(i) = (s - div[i] * h * h) / 6.0;
                            }
                        }
                    }
                });
            }
        }
        let grad: Vec<Vec3> = (0..n)
            .into_par_iter()
            .map(|i| {
                if solid[i] {
                    return Vec3::ZERO;
                }
                let (x, y, z) = coords(i);
                let get = |xx: i64, yy: i64, zz: i64| {
                    if xx < 0 || yy < 0 || zz < 0 || xx >= rx as i64 || yy >= ry as i64 || zz >= rz as i64 {
                        return 0.0;
                    }
                    let j = g.idx(xx as usize, yy as usize, zz as usize);
                    if solid[j] {
                        pres[i]
                    } else {
                        pres[j]
                    }
                };
                let (xi, yi, zi) = (x as i64, y as i64, z as i64);
                Vec3::new(
                    get(xi + 1, yi, zi) - get(xi - 1, yi, zi),
                    get(xi, yi + 1, zi) - get(xi, yi - 1, zi),
                    get(xi, yi, zi + 1) - get(xi, yi, zi - 1),
                ) / (2.0 * h)
            })
            .collect();
        u.par_iter_mut().zip(&grad).for_each(|(a, gr)| *a -= gr.x);
        v.par_iter_mut().zip(&grad).for_each(|(a, gr)| *a -= gr.y);
        w.par_iter_mut().zip(&grad).for_each(|(a, gr)| *a -= gr.z);
        // 5. Trasporto di fumo e temperatura, dissolvenza e raffreddamento.
        let keep_r = (-p.dissipation.max(0.0) * dt).exp();
        let keep_t = (-p.cooling.max(0.0) * dt).exp();
        let (nr, nt): (Vec<f32>, Vec<f32>) = (0..n)
            .into_par_iter()
            .map(|i| {
                if solid[i] {
                    return (0.0, 0.0);
                }
                let q = back(i, &u, &v, &w);
                let e = edge[i];
                (
                    (g.sample(&rho, q) * keep_r * e).max(0.0),
                    (g.sample(&temp, q) * keep_t * e).max(0.0),
                )
            })
            .unzip();
        rho = nr;
        temp = nt;
        if let Some((every, f)) = frames.as_mut() {
            let (before, after) = (time, time + dt);
            if (after / *every).floor() > (before / *every).floor() || step + 1 == steps {
                f(after, &rho);
            }
        }
    }
    progress(1.0);
    // Riferimento: il 90° percentile delle celle con fumo (non la sorgente, sempre piena).
    let mut vals: Vec<f32> = rho.iter().copied().filter(|&v| v > 1e-4).collect();
    let max = if vals.is_empty() {
        0.0
    } else {
        let k = (vals.len() as f32 * 0.9) as usize;
        let k = k.min(vals.len() - 1);
        *vals.select_nth_unstable_by(k, |a, b| a.total_cmp(b)).1
    };
    Some(SmokeField {
        spec: p.spec,
        max,
        density: rho,
        steps,
        ms: t0.elapsed().as_secs_f64() * 1e3,
    })
}

#[derive(Clone, Copy)]
struct SharedPtr(*mut f32);
// SAFETY: usato solo nel Gauss–Seidel rosso/nero, dove le scritture di ogni mezzo passo vanno a
// celle diverse per thread e le letture a celle dell'altro colore.
unsafe impl Send for SharedPtr {}
unsafe impl Sync for SharedPtr {}
impl SharedPtr {
    fn get(self) -> *mut f32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(spec: GridSpec) -> SmokeSimParams {
        SmokeSimParams {
            spec,
            source: Vec3::new(0.5, 0.1, 0.5),
            source_radius: 0.06,
            emission: 3.0,
            temperature: 1.0,
            buoyancy: 4.0,
            weight: 0.2,
            exit_speed: 0.3,
            cooling: 0.5,
            dissipation: 0.1,
            vorticity: 2.0,
            turbulence: 1.0,
            turbulence_scale: 0.15,
            wind: Vec3::ZERO,
            time: 1.5,
            seed: 3,
            edge_fade: 0.0,
            box_center: Vec3::ZERO,
            box_yaw: 0.0,
            box_half: Vec3::ONE,
        }
    }

    /// Fumo che arriva al soffitto: senza bordi sfumati si ammassa contro il riquadro, con i
    /// bordi sfumati sotto il soffitto ne resta molto meno, ma il fumo c'è ancora più in basso.
    #[test]
    fn soft_edges_keep_smoke_off_the_box() {
        let spec = GridSpec::covering(Vec3::ZERO, Vec3::new(1.0, 1.0, 1.0), 24);
        let none = || false;
        let p = SmokeSimParams { time: 4.0, ..params(spec) };
        let hard = simulate_smoke(&p, None, &|_| {}, &none).unwrap();
        let soft = simulate_smoke(&SmokeSimParams { edge_fade: 0.2, ..p }, None, &|_| {}, &none).unwrap();
        let top = |f: &SmokeField| -> f32 {
            let ry = f.spec.res[1];
            (0..f.spec.len())
                .filter(|&i| f.spec.coords(i).1 + 1 == ry)
                .map(|i| f.density[i])
                .sum()
        };
        let mid = |f: &SmokeField| -> f32 {
            (0..f.spec.len())
                .filter(|&i| {
                    let (_, y, _) = f.spec.coords(i);
                    (6..14).contains(&y)
                })
                .map(|i| f.density[i])
                .sum()
        };
        eprintln!("soffitto {} → {}, a metà {} → {}", top(&hard), top(&soft), mid(&hard), mid(&soft));
        assert!(top(&hard) > 0.05, "senza bordi sfumati il fumo arriva al soffitto");
        assert!(top(&soft) < top(&hard) * 0.1, "con i bordi sfumati sotto il soffitto quasi niente");
        assert!(mid(&soft) > mid(&hard) * 0.3, "il fumo più in basso resta");
    }

    /// Fumo caldo: sale dalla sorgente (più fumo in alto che in basso dopo un po'), è
    /// deterministico; un gas freddo e pesante resta in basso.
    #[test]
    fn hot_smoke_rises_and_heavy_gas_stays_low() {
        let spec = GridSpec::covering(Vec3::ZERO, Vec3::new(1.0, 1.5, 1.0), 32);
        let none = || false;
        let a = simulate_smoke(&params(spec), None, &|_| {}, &none).unwrap();
        let b = simulate_smoke(&params(spec), None, &|_| {}, &none).unwrap();
        assert_eq!(a.density, b.density, "deterministico");
        let layer = |f: &SmokeField, y0: f32, y1: f32| -> f32 {
            (0..f.spec.len())
                .filter(|&i| {
                    let (x, y, z) = f.spec.coords(i);
                    let c = f.spec.center(x, y, z);
                    c.y >= y0 && c.y < y1
                })
                .map(|i| f.density[i])
                .sum()
        };
        // Baricentro del fumo: ben sopra la sorgente (y = 0,1).
        let centroid = |f: &SmokeField| {
            let (mut m, mut my) = (0.0f32, 0.0f32);
            for i in 0..f.spec.len() {
                let (x, y, z) = f.spec.coords(i);
                m += f.density[i];
                my += f.density[i] * f.spec.center(x, y, z).y;
            }
            my / m.max(1e-9)
        };
        assert!(centroid(&a) > 0.22, "il caldo sale: baricentro {}", centroid(&a));
        assert!(layer(&a, 0.4, 1.5) > 0.0, "arriva in alto");
        let heavy = SmokeSimParams {
            temperature: 0.0,
            weight: 3.0,
            exit_speed: 0.0,
            source: Vec3::new(0.5, 0.6, 0.5),
            ..params(spec)
        };
        let c = simulate_smoke(&heavy, None, &|_| {}, &none).unwrap();
        assert!(layer(&c, 0.0, 0.6) > layer(&c, 0.6, 1.5), "il gas pesante scende");
    }
}
