//! Lucciole: uno stormo che vola nel suo volume e si accende a intervalli, ognuna per conto suo.
//!
//! Tutto è una funzione del tempo assoluto e del numero della lucciola: nessuno stato da integrare,
//! quindi Play, salto a un fotogramma, viewport e render vedono la stessa cosa, a qualunque velocità.
//! - **Volo:** somma di poche oscillazioni con frequenze e fasi proprie. La somma è limitata, quindi
//!   la lucciola resta nel volume senza rimbalzi né teletrasporti.
//! - **Ciclo:** spenta, accensione, accesa, spegnimento, con durate estratte a ogni giro.
//! - **Luce:** le lucciole accese diventano luci puntiformi vere della scena ([`Fireflies::lights`]),
//!   con ombre e raggio d'azione. Il renderer ne pesca una a caso per punto, quindi troppe luci fanno
//!   solo rumore: oltre [`Fireflies::max_lights`] quelle vicine si uniscono in una, che somma la loro
//!   potenza nel loro baricentro (l'energia totale non cambia, le ombre vicine sì).
//! - **Corpo e alone:** i punti visibili ([`Fireflies::sparks`]) li disegna l'alone in post-produzione
//!   ([`crate`] non c'entra con la luce): si regolano a parte dalla luce sulla scena.

use crate::light::Light;
use forge_math::Vec3;
use serde::{Deserialize, Serialize};

/// Una voce della palette: colore e parte di lucciole che lo prendono.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FireflyColor {
    pub color: [f32; 3],
    pub share: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Fireflies {
    pub enabled: bool,
    pub count: u32,
    pub seed: u32,
    /// Volume in cui volano: centro, dimensioni (m) e rotazione attorno alla verticale (gradi).
    pub center: Vec3,
    pub size: Vec3,
    pub yaw_deg: f32,
    /// Velocità media di volo (m/s) e quanto varia tra le lucciole (0–1).
    pub speed: f32,
    pub speed_spread: f32,
    /// Movimento verticale rispetto a quello orizzontale (0–1).
    pub vertical: f32,
    /// Fluidità: alta = curve larghe, bassa = più guizzi (0–1).
    pub smooth: f32,
    /// Quanto si disperdono nel volume (0 = raccolte al centro, 1 = dappertutto).
    pub spread: f32,
    /// Durate in secondi, minima e massima: spenta, accensione, accesa, spegnimento.
    pub off_s: [f32; 2],
    pub rise_s: [f32; 2],
    pub on_s: [f32; 2],
    pub fall_s: [f32; 2],
    /// Intensità di ogni accensione, minima e massima (W/sr).
    pub intensity: [f32; 2],
    pub palette: Vec<FireflyColor>,
    /// Lucciole con una tinta variata (0–1) e ampiezza della variazione (0–1 = fino a 30° di tinta).
    pub tint_share: f32,
    pub tint_amount: f32,
    /// Luce sulla scena: moltiplica l'intensità (0 = solo punti visibili).
    pub scene_light: f32,
    /// Fin dove arriva la luce di una lucciola (m).
    pub light_range: f32,
    /// Quante luci al massimo per il render: oltre, quelle vicine si uniscono.
    pub max_lights: u32,
    /// La luce è fermata dagli oggetti.
    pub shadows: bool,
    /// Diametro del corpo luminoso (mm).
    pub body_mm: f32,
    /// Alone: grandezza in diametri del corpo e forza.
    pub halo_size: f32,
    pub halo_strength: f32,
}

impl Default for Fireflies {
    fn default() -> Self {
        Self {
            enabled: false,
            count: 60,
            seed: 1,
            center: Vec3::new(0.0, 0.8, 0.0),
            size: Vec3::new(4.0, 1.6, 4.0),
            yaw_deg: 0.0,
            speed: 0.25,
            speed_spread: 0.5,
            vertical: 0.3,
            smooth: 0.7,
            spread: 1.0,
            off_s: [1.5, 6.0],
            rise_s: [0.3, 0.8],
            on_s: [0.4, 1.5],
            fall_s: [0.5, 1.2],
            intensity: [0.04, 0.12],
            palette: vec![
                FireflyColor {
                    color: [0.85, 1.0, 0.3],
                    share: 0.7,
                },
                FireflyColor {
                    color: [1.0, 0.75, 0.25],
                    share: 0.3,
                },
            ],
            tint_share: 0.4,
            tint_amount: 0.3,
            scene_light: 1.0,
            light_range: 2.5,
            max_lights: 12,
            shadows: true,
            body_mm: 6.0,
            halo_size: 8.0,
            halo_strength: 1.0,
        }
    }
}

/// Una lucciola in un istante, se è accesa.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spark {
    pub position: Vec3,
    pub color: [f32; 3],
    /// Luminosità da 0 a 1 nel ciclo.
    pub level: f32,
    /// Intensità in questo istante (W/sr), prima della luce sulla scena.
    pub power: f32,
}

/// Numero in [0, 1) fisso per seme, lucciola e canale.
fn hash(seed: u32, i: u32, k: u32) -> f32 {
    let mut x = seed.wrapping_mul(0x9E37_79B9) ^ i.wrapping_mul(0x85EB_CA6B) ^ k.wrapping_mul(0xC2B2_AE35);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2C1B_3C6D);
    x ^= x >> 12;
    x = x.wrapping_mul(0x297A_2D39);
    x ^= x >> 15;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn lerp_range(r: [f32; 2], u: f32) -> f32 {
    let (a, b) = (r[0].min(r[1]).max(0.0), r[0].max(r[1]).max(0.0));
    a + (b - a) * u
}

/// Canali per fase: durate, intensità del giro.
const CYCLE_CHANNELS: u32 = 8;

fn rgb_to_hsv(c: [f32; 3]) -> [f32; 3] {
    let (max, min) = (c[0].max(c[1]).max(c[2]), c[0].min(c[1]).min(c[2]));
    let d = max - min;
    let h = if d <= 0.0 {
        0.0
    } else if max == c[0] {
        ((c[1] - c[2]) / d).rem_euclid(6.0)
    } else if max == c[1] {
        (c[2] - c[0]) / d + 2.0
    } else {
        (c[0] - c[1]) / d + 4.0
    };
    [h / 6.0, if max > 0.0 { d / max } else { 0.0 }, max]
}

fn hsv_to_rgb(c: [f32; 3]) -> [f32; 3] {
    let h = c[0].rem_euclid(1.0) * 6.0;
    let k = c[1] * c[2];
    let x = k * (1.0 - (h % 2.0 - 1.0).abs());
    let m = c[2] - k;
    let (r, g, b) = match h as u32 {
        0 => (k, x, 0.0),
        1 => (x, k, 0.0),
        2 => (0.0, k, x),
        3 => (0.0, x, k),
        4 => (x, 0.0, k),
        _ => (k, 0.0, x),
    };
    [r + m, g + m, b + m]
}

impl Fireflies {
    /// Il preset "Bosco incantato": molte lucciole lente e sparse, luce calda e morbida.
    pub fn enchanted_forest(center: Vec3, size: Vec3) -> Self {
        Self {
            enabled: true,
            count: 120,
            center,
            size,
            speed: 0.18,
            smooth: 0.8,
            palette: vec![
                FireflyColor {
                    color: [0.75, 1.0, 0.25],
                    share: 0.55,
                },
                FireflyColor {
                    color: [1.0, 0.8, 0.3],
                    share: 0.35,
                },
                FireflyColor {
                    color: [1.0, 0.55, 0.2],
                    share: 0.1,
                },
            ],
            ..Default::default()
        }
    }

    /// Parti della palette riportate a somma 1 (quelle a zero restano a zero).
    pub fn shares(&self) -> Vec<f32> {
        let total: f32 = self.palette.iter().map(|p| p.share.max(0.0)).sum();
        self.palette
            .iter()
            .map(|p| {
                if total > 0.0 {
                    p.share.max(0.0) / total
                } else {
                    1.0 / self.palette.len().max(1) as f32
                }
            })
            .collect()
    }

    /// Colore fisso della lucciola `i`: quello della palette che le tocca, con la sua variazione,
    /// che gira la tinta di poco e non cambia famiglia.
    pub fn color_of(&self, i: u32) -> [f32; 3] {
        let shares = self.shares();
        let mut u = hash(self.seed, i, 100);
        let mut color = self.palette.last().map_or([1.0, 0.9, 0.4], |p| p.color);
        for (p, s) in self.palette.iter().zip(&shares) {
            if u < *s {
                color = p.color;
                break;
            }
            u -= s;
        }
        if hash(self.seed, i, 101) < self.tint_share.clamp(0.0, 1.0) {
            let mut hsv = rgb_to_hsv(color);
            let turn = (hash(self.seed, i, 102) * 2.0 - 1.0) * self.tint_amount.clamp(0.0, 1.0) * (30.0 / 360.0);
            hsv[0] += turn;
            color = hsv_to_rgb(hsv);
        }
        color
    }

    /// Posizione della lucciola `i` all'istante `t`: dentro il volume, sempre.
    pub fn position(&self, i: u32, t: f32) -> Vec3 {
        let half = self.size.max(Vec3::splat(0.01)) * 0.5;
        let keep = self.spread.clamp(0.05, 1.0);
        let tau = std::f32::consts::TAU;
        // Frequenza della curva più ampia: percorre il volume alla velocità chiesta.
        let reach = (half.x + half.z).max(0.02) * 0.5 * keep;
        let own = 1.0 + (hash(self.seed, i, 3) * 2.0 - 1.0) * self.speed_spread.clamp(0.0, 1.0) * 0.8;
        let base = self.speed.max(0.0) * own / (tau * reach * 0.55);
        // Pesi delle tre oscillazioni: più fluida, meno guizzi. Sommano a 1.
        let wiggle = 1.0 - self.smooth.clamp(0.0, 1.0);
        let (w1, w2) = (0.3 * (0.3 + wiggle), 0.15 * (0.2 + wiggle));
        let weights = [1.0 - w1 - w2, w1, w2];
        let axis = |a: u32| -> f32 {
            let mut v = 0.0;
            for (k, w) in weights.iter().enumerate() {
                let c = a * 8 + k as u32 * 2 + 10;
                let f = base * (1.0 + hash(self.seed, i, c) * 0.5) * [1.0, 2.7, 6.1][k];
                v += w * (tau * (f * t + hash(self.seed, i, c + 1))).sin();
            }
            v
        };
        let home = Vec3::new(hash(self.seed, i, 0), hash(self.seed, i, 1), hash(self.seed, i, 2)) * 2.0 - Vec3::ONE;
        // Metà del volume per il punto di partenza, metà per il volo: la somma non esce mai.
        let vertical = self.vertical.clamp(0.0, 1.0);
        let p = Vec3::new(
            home.x * 0.5 + axis(0) * 0.5,
            home.y * 0.5 + axis(1) * 0.5 * vertical,
            home.z * 0.5 + axis(2) * 0.5,
        ) * half
            * Vec3::new(keep, keep, keep);
        let (s, c) = self.yaw_deg.to_radians().sin_cos();
        self.center + Vec3::new(p.x * c + p.z * s, p.y, p.z * c - p.x * s)
    }

    /// Luminosità (0–1) e intensità della lucciola `i` all'istante `t`. Il giro in corso si trova
    /// ripercorrendo i giri precedenti dai loro numeri fissi: salto e riproduzione coincidono.
    pub fn level(&self, i: u32, t: f32) -> (f32, f32) {
        let d = |n: u32, k: u32, r: [f32; 2]| lerp_range(r, hash(self.seed ^ n.wrapping_mul(0x9E37), i, 200 + k));
        let length = |n: u32| {
            d(n, 0, self.off_s).max(0.01) + d(n, 1, self.rise_s).max(0.01) + d(n, 2, self.on_s).max(0.0) + d(n, 3, self.fall_s).max(0.01)
        };
        // Fase iniziale diversa per ognuna: non partono insieme.
        let mut start = -hash(self.seed, i, 50) * length(0);
        let mut n = 0;
        while start + length(n) <= t {
            start += length(n);
            n += 1;
            if n > 100_000 {
                return (0.0, 0.0);
            }
        }
        let local = t - start;
        let (off, rise, on, fall) = (
            d(n, 0, self.off_s).max(0.01),
            d(n, 1, self.rise_s).max(0.01),
            d(n, 2, self.on_s).max(0.0),
            d(n, 3, self.fall_s).max(0.01),
        );
        let level = if local < off {
            0.0
        } else if local < off + rise {
            smooth((local - off) / rise)
        } else if local < off + rise + on {
            1.0
        } else {
            1.0 - smooth((local - off - rise - on) / fall)
        };
        let power = lerp_range(self.intensity, hash(self.seed ^ n.wrapping_mul(0x9E37), i, 200 + CYCLE_CHANNELS));
        (level, power)
    }

    /// Le lucciole accese all'istante `t`.
    pub fn sparks(&self, t: f32) -> Vec<Spark> {
        if !self.enabled {
            return Vec::new();
        }
        (0..self.count)
            .filter_map(|i| {
                let (level, power) = self.level(i, t);
                (level > 1e-3).then(|| Spark {
                    position: self.position(i, t),
                    color: self.color_of(i),
                    level,
                    power: power * level,
                })
            })
            .collect()
    }

    /// Luci vere della scena all'istante `t`: una per lucciola accesa o, se sono troppe, una per
    /// zona del volume, con la potenza sommata.
    pub fn lights(&self, t: f32) -> Vec<Light> {
        if self.scene_light <= 0.0 {
            return Vec::new();
        }
        let sparks = self.sparks(t);
        let cells = cluster(&sparks, self.max_lights.max(1) as usize, self.center, self.size);
        cells
            .iter()
            .enumerate()
            .map(|(k, c)| {
                let mut l = Light::point(&format!("Lucciole {k}"), c.position, c.power * self.scene_light);
                l.color = c.color;
                l.cast_shadows = self.shadows;
                l.range = self.light_range.max(0.05);
                l
            })
            .collect()
    }
}

/// Unisce le lucciole in al massimo `max` gruppi su una griglia del volume, senza preferire colori o
/// zone: ogni gruppo sta nel baricentro delle sue, ha la loro potenza sommata e il colore medio.
fn cluster(sparks: &[Spark], max: usize, center: Vec3, size: Vec3) -> Vec<Spark> {
    if sparks.len() <= max {
        return sparks.to_vec();
    }
    let side = (max as f32).sqrt().floor().max(1.0) as usize;
    let half = size.max(Vec3::splat(0.01)) * 0.5;
    #[derive(Clone, Copy, Default)]
    struct Sum {
        pos: Vec3,
        color: [f32; 3],
        power: f32,
        level: f32,
    }
    let mut cells = vec![Sum::default(); side * side];
    for s in sparks {
        let q = (s.position - center + half) / (half * 2.0);
        let cell = |v: f32| ((v.clamp(0.0, 0.999) * side as f32) as usize).min(side - 1);
        let c = &mut cells[cell(q.x) + cell(q.z) * side];
        let w = s.power.max(1e-9);
        c.pos += s.position * w;
        for a in 0..3 {
            c.color[a] += s.color[a] * w;
        }
        c.power += w;
        c.level = c.level.max(s.level);
    }
    cells
        .iter()
        .filter(|c| c.power > 0.0)
        .map(|c| Spark {
            position: c.pos / c.power,
            color: c.color.map(|v| v / c.power),
            level: c.level,
            power: c.power,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f() -> Fireflies {
        Fireflies {
            enabled: true,
            count: 200,
            ..Default::default()
        }
    }

    /// Volano anche spente, restano nel volume e non fanno mai un salto.
    #[test]
    fn they_fly_smoothly_inside_their_volume() {
        let f = f();
        let half = f.size * 0.5;
        for i in 0..f.count {
            let mut last = f.position(i, 0.0);
            let mut travelled = 0.0;
            for k in 1..240 {
                let p = f.position(i, k as f32 / 24.0);
                let d = p - f.center;
                assert!(d.x.abs() <= half.x && d.y.abs() <= half.y && d.z.abs() <= half.z, "fuori: {d:?}");
                assert!((p - last).length() < 0.15, "salto di {}", (p - last).length());
                travelled += (p - last).length();
                last = p;
            }
            assert!(travelled > 0.2, "ferma: {travelled}");
        }
    }

    /// Più orizzontale che verticale, come chiesto.
    #[test]
    fn horizontal_motion_dominates() {
        let f = f();
        let (mut h, mut v) = (0.0, 0.0);
        for i in 0..f.count {
            for k in 0..120 {
                let d = f.position(i, (k + 1) as f32 / 24.0) - f.position(i, k as f32 / 24.0);
                h += (d.x * d.x + d.z * d.z).sqrt();
                v += d.y.abs();
            }
        }
        assert!(v < h * 0.5, "verticale {v}, orizzontale {h}");
    }

    /// Ciclo morbido e asincrono: mai tutte insieme, nessun salto di luminosità, stesso risultato
    /// comunque ci si arrivi.
    #[test]
    fn the_cycle_is_soft_and_not_in_sync() {
        let f = f();
        let mut lit = Vec::new();
        for k in 0..480 {
            let t = k as f32 / 24.0;
            lit.push(f.sparks(t).len());
            for i in 0..f.count {
                assert_eq!(f.level(i, t), f.level(i, t));
                let (a, b) = (f.level(i, t).0, f.level(i, t + 1.0 / 240.0).0);
                assert!((a - b).abs() < 0.1, "salto da {a} a {b}");
            }
        }
        let (lo, hi) = (*lit.iter().min().unwrap(), *lit.iter().max().unwrap());
        assert!(hi > 0 && hi < f.count as usize, "sempre tutte: {hi}");
        assert!(hi < lo + f.count as usize / 2, "lampeggiano insieme: da {lo} a {hi}");
    }

    /// La palette dà le parti chieste, il colore di ognuna non cambia, la tinta resta in famiglia.
    #[test]
    fn colours_follow_the_palette() {
        let f = Fireflies {
            count: 4000,
            tint_share: 0.0,
            palette: vec![
                FireflyColor {
                    color: [1.0, 0.0, 0.0],
                    share: 3.0,
                },
                FireflyColor {
                    color: [0.0, 1.0, 0.0],
                    share: 1.0,
                },
            ],
            ..f()
        };
        assert_eq!(f.shares(), vec![0.75, 0.25]);
        let red = (0..f.count).filter(|&i| f.color_of(i)[0] > 0.5).count() as f32 / f.count as f32;
        assert!((red - 0.75).abs() < 0.03, "{red}");
        let tinted = Fireflies {
            tint_share: 1.0,
            tint_amount: 1.0,
            palette: vec![FireflyColor {
                color: [1.0, 0.8, 0.2],
                share: 1.0,
            }],
            ..f.clone()
        };
        let base = rgb_to_hsv([1.0, 0.8, 0.2])[0];
        let mut varied = false;
        for i in 0..200 {
            let c = tinted.color_of(i);
            assert_eq!(c, tinted.color_of(i));
            let turn = (rgb_to_hsv(c)[0] - base).abs();
            assert!(turn <= 30.0 / 360.0 + 1e-3, "{turn}");
            varied |= turn > 0.01;
        }
        assert!(varied);
    }

    /// Le accese diventano luci vere; oltre il limite si uniscono, con la stessa energia.
    #[test]
    fn lights_keep_the_energy_when_merged() {
        let few = f();
        let t = 7.0;
        let sparks = few.sparks(t);
        assert!(sparks.len() > 16, "{}", sparks.len());
        let power: f32 = sparks.iter().map(|s| s.power).sum();
        let lights = few.lights(t);
        assert!(lights.len() <= few.max_lights as usize);
        let total: f32 = lights.iter().map(|l| l.intensity).sum();
        assert!((total - power).abs() < power * 1e-3, "{total} contro {power}");
        assert!(lights.iter().all(|l| l.cast_shadows && l.range > 0.0));
        let off = Fireflies {
            enabled: false,
            ..few.clone()
        };
        assert!(off.lights(t).is_empty());
        let unlit = Fireflies { scene_light: 0.0, ..few };
        assert!(unlit.lights(t).is_empty(), "solo punti visibili");
        assert!(!unlit.sparks(t).is_empty());
    }

    #[test]
    fn hsv_round_trip() {
        for c in [[1.0, 0.5, 0.25], [0.2, 0.9, 0.4], [0.1, 0.2, 0.8]] {
            let b = hsv_to_rgb(rgb_to_hsv(c));
            assert!((0..3).all(|k| (b[k] - c[k]).abs() < 1e-4), "{b:?}");
        }
    }
}
