//! Rumore procedurale deterministico: gradient noise 3D (tipo Perlin) con
//! seed, fBm e curl noise (campo di velocità a divergenza nulla per vortici e
//! filamenti). Nessuna tabella globale: il risultato dipende solo da posizione
//! e seed, quindi è identico su qualsiasi thread.

use forge_math::Vec3;

#[inline]
fn hash(x: i32, y: i32, z: i32, seed: u32) -> u32 {
    // Mescolamento intero (variante di "lowbias32") di coordinate e seed.
    let mut h = (x as u32).wrapping_mul(0x8da6_b343)
        ^ (y as u32).wrapping_mul(0xd816_3841)
        ^ (z as u32).wrapping_mul(0xcb1a_b31f)
        ^ seed.wrapping_mul(0x9e37_79b9);
    h ^= h >> 16;
    h = h.wrapping_mul(0x7feb_352d);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846c_a68b);
    h ^= h >> 16;
    h
}

/// Uno dei 12 gradienti sugli spigoli del cubo (come nel Perlin migliorato).
#[inline]
fn grad(h: u32, x: f32, y: f32, z: f32) -> f32 {
    match h % 12 {
        0 => x + y,
        1 => -x + y,
        2 => x - y,
        3 => -x - y,
        4 => x + z,
        5 => -x + z,
        6 => x - z,
        7 => -x - z,
        8 => y + z,
        9 => -y + z,
        10 => y - z,
        _ => -y - z,
    }
}

#[inline]
fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Gradient noise 3D in circa [-1, 1], continuo con derivate continue.
pub fn gradient(p: Vec3, seed: u32) -> f32 {
    let (fx, fy, fz) = (p.x.floor(), p.y.floor(), p.z.floor());
    let (x0, y0, z0) = (fx as i32, fy as i32, fz as i32);
    let (x, y, z) = (p.x - fx, p.y - fy, p.z - fz);
    let (u, v, w) = (fade(x), fade(y), fade(z));
    let g = |i: i32, j: i32, k: i32| grad(hash(x0 + i, y0 + j, z0 + k, seed), x - i as f32, y - j as f32, z - k as f32);
    let x00 = lerp(g(0, 0, 0), g(1, 0, 0), u);
    let x10 = lerp(g(0, 1, 0), g(1, 1, 0), u);
    let x01 = lerp(g(0, 0, 1), g(1, 0, 1), u);
    let x11 = lerp(g(0, 1, 1), g(1, 1, 1), u);
    lerp(lerp(x00, x10, v), lerp(x01, x11, v), w)
}

/// Somma frattale di `octaves` ottave (lacunarità 2, guadagno 0.5), normalizzata in circa [-1, 1].
pub fn fbm(p: Vec3, seed: u32, octaves: u32) -> f32 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..octaves {
        // Ogni ottava ha il suo seed e un piccolo spostamento: niente allineamenti tra ottave.
        sum += amp * gradient(p * freq + Vec3::splat(o as f32 * 17.13), seed.wrapping_add(o * 1013));
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

/// Curl noise: rotore di un potenziale vettoriale di rumore (divergenza nulla).
/// Restituisce una velocità di ampiezza tipica ~1.
pub fn curl(p: Vec3, seed: u32) -> Vec3 {
    let e = 0.01;
    let psi = |q: Vec3| {
        Vec3::new(
            fbm(q, seed, 2),
            fbm(q + Vec3::new(31.4, 7.1, 2.9), seed ^ 0x5bd1_e995, 2),
            fbm(q + Vec3::new(-11.7, 23.3, 41.9), seed ^ 0x27d4_eb2f, 2),
        )
    };
    let dx = (psi(p + Vec3::X * e) - psi(p - Vec3::X * e)) / (2.0 * e);
    let dy = (psi(p + Vec3::Y * e) - psi(p - Vec3::Y * e)) / (2.0 * e);
    let dz = (psi(p + Vec3::Z * e) - psi(p - Vec3::Z * e)) / (2.0 * e);
    // rot ψ = (∂ψz/∂y − ∂ψy/∂z, ∂ψx/∂z − ∂ψz/∂x, ∂ψy/∂x − ∂ψx/∂y)
    Vec3::new(dy.z - dz.y, dz.x - dx.z, dx.y - dy.x) * 0.5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_seed_dependent() {
        let p = Vec3::new(1.37, -2.11, 0.53);
        assert_eq!(gradient(p, 7), gradient(p, 7));
        assert_ne!(gradient(p, 7), gradient(p, 8));
        assert_eq!(fbm(p, 3, 5), fbm(p, 3, 5));
    }

    #[test]
    fn range_and_zero_at_lattice() {
        assert_eq!(gradient(Vec3::new(3.0, -4.0, 5.0), 1), 0.0);
        let mut lo = f32::INFINITY;
        let mut hi = f32::NEG_INFINITY;
        let mut mean = 0.0f64;
        let n = 20000;
        for i in 0..n {
            let p = Vec3::new(i as f32 * 0.137, (i % 97) as f32 * 0.29, (i % 13) as f32 * 0.71);
            let v = fbm(p, 42, 4);
            lo = lo.min(v);
            hi = hi.max(v);
            mean += v as f64;
        }
        assert!(lo >= -1.0 && hi <= 1.0, "{lo} {hi}");
        assert!(hi - lo > 0.6, "poca variazione: {lo} {hi}");
        assert!((mean / n as f64).abs() < 0.05, "media {}", mean / n as f64);
    }

    #[test]
    fn curl_is_divergence_free() {
        let e = 0.02;
        for i in 0..20 {
            let p = Vec3::new(i as f32 * 0.31, 0.7 + i as f32 * 0.11, -0.4 * i as f32);
            let div = (curl(p + Vec3::X * e, 5).x - curl(p - Vec3::X * e, 5).x + curl(p + Vec3::Y * e, 5).y - curl(p - Vec3::Y * e, 5).y
                + curl(p + Vec3::Z * e, 5).z
                - curl(p - Vec3::Z * e, 5).z)
                / (2.0 * e);
            let mag = curl(p, 5).length();
            assert!(div.abs() < 0.05 * mag.max(0.5), "divergenza {div} (|v| {mag})");
        }
    }
}
