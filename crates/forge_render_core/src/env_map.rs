//! Mappe d'ambiente HDRI equirettangolari con importance sampling.
//!
//! La mappa è trattata come funzione costante a tratti sui pixel (lookup
//! "nearest"): la PDF di campionamento è proporzionale a luminanza · sin θ e
//! coincide esattamente con la funzione campionata, come richiesto dal MIS.
//! Solo lo sfondo visto direttamente dalla camera usa l'interpolazione bilineare.
//!
//! Convenzione: asse Y verso l'alto; `u = 0.5 + atan2(x, -z) / 2π` (il centro
//! dell'immagine guarda verso −Z), `v = acos(y) / π` (riga 0 = zenit).

use anyhow::Context;
use forge_math::{luminance, Vec3};
use std::collections::HashMap;
use std::f32::consts::{PI, TAU};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

/// Distribuzione 1D costante a tratti (PBRT, cap. 13).
#[derive(Clone, Debug)]
pub struct Distribution1D {
    func: Vec<f32>,
    cdf: Vec<f32>,
    integral: f32,
}

impl Distribution1D {
    pub fn new(func: Vec<f32>) -> Self {
        let n = func.len();
        let mut cdf = vec![0.0f32; n + 1];
        for i in 0..n {
            cdf[i + 1] = cdf[i] + func[i].max(0.0) / n as f32;
        }
        let integral = cdf[n];
        if integral > 0.0 {
            for c in cdf.iter_mut().skip(1) {
                *c /= integral;
            }
        } else {
            for (i, c) in cdf.iter_mut().enumerate() {
                *c = i as f32 / n as f32;
            }
        }
        Self { func, cdf, integral }
    }

    pub fn count(&self) -> usize {
        self.func.len()
    }

    pub fn func(&self) -> &[f32] {
        &self.func
    }

    pub fn cdf(&self) -> &[f32] {
        &self.cdf
    }

    pub fn integral(&self) -> f32 {
        self.integral
    }

    /// Restituisce (x continuo in [0,1), pdf, indice).
    pub fn sample(&self, u: f32) -> (f32, f32, usize) {
        let n = self.func.len();
        // Ultimo indice con cdf[i] <= u.
        let idx = match self.cdf.partition_point(|&c| c <= u) {
            0 => 0,
            p => (p - 1).min(n - 1),
        };
        let width = self.cdf[idx + 1] - self.cdf[idx];
        let du = if width > 0.0 { (u - self.cdf[idx]) / width } else { 0.0 };
        let pdf = if self.integral > 0.0 { self.func[idx] / self.integral } else { 1.0 };
        (((idx as f32 + du) / n as f32).min(1.0 - f32::EPSILON), pdf, idx)
    }

    pub fn pdf(&self, idx: usize) -> f32 {
        if self.integral > 0.0 {
            self.func[idx] / self.integral
        } else {
            1.0
        }
    }
}

/// Mappa HDRI pronta per il rendering.
#[derive(Debug)]
pub struct EnvironmentMap {
    pub width: usize,
    pub height: usize,
    pixels: Vec<Vec3>,
    marginal: Distribution1D,
    conditional: Vec<Distribution1D>,
}

impl EnvironmentMap {
    pub fn from_pixels(width: usize, height: usize, pixels: Vec<Vec3>) -> Self {
        assert_eq!(pixels.len(), width * height);
        let pixels: Vec<Vec3> = pixels
            .into_iter()
            .map(|p| if p.is_finite() { p.max(Vec3::ZERO) } else { Vec3::ZERO })
            .collect();
        let mut conditional = Vec::with_capacity(height);
        let mut row_integrals = Vec::with_capacity(height);
        for y in 0..height {
            let sin_t = ((y as f32 + 0.5) / height as f32 * PI).sin();
            let row: Vec<f32> = (0..width).map(|x| luminance(pixels[y * width + x]) * sin_t).collect();
            let d = Distribution1D::new(row);
            row_integrals.push(d.integral);
            conditional.push(d);
        }
        let marginal = Distribution1D::new(row_integrals);
        Self {
            width,
            height,
            pixels,
            marginal,
            conditional,
        }
    }

    pub fn pixels(&self) -> &[Vec3] {
        &self.pixels
    }

    pub fn marginal(&self) -> &Distribution1D {
        &self.marginal
    }

    pub fn conditional(&self) -> &[Distribution1D] {
        &self.conditional
    }

    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let img = image::open(path)
            .with_context(|| format!("lettura HDRI {}", path.display()))?
            .into_rgb32f();
        let (w, h) = img.dimensions();
        anyhow::ensure!(w >= 2 && h >= 1, "immagine HDRI troppo piccola");
        let pixels = img.pixels().map(|p| Vec3::new(p[0], p[1], p[2])).collect();
        Ok(Self::from_pixels(w as usize, h as usize, pixels))
    }

    #[inline]
    fn dir_to_uv(dir: Vec3, rotation: f32) -> (f32, f32) {
        let u = (0.5 + dir.x.atan2(-dir.z) / TAU - rotation / TAU).rem_euclid(1.0);
        let v = dir.y.clamp(-1.0, 1.0).acos() / PI;
        (u, v)
    }

    #[inline]
    fn uv_to_dir(u: f32, v: f32, rotation: f32) -> Vec3 {
        let phi = (u - 0.5) * TAU + rotation;
        let theta = v * PI;
        let (st, ct) = theta.sin_cos();
        Vec3::new(st * phi.sin(), ct, -st * phi.cos())
    }

    #[inline]
    fn texel(&self, u: f32, v: f32) -> (usize, usize) {
        let x = ((u * self.width as f32) as usize).min(self.width - 1);
        let y = ((v * self.height as f32) as usize).min(self.height - 1);
        (x, y)
    }

    /// Radianza (costante a tratti, coerente con la PDF).
    #[inline]
    pub fn radiance(&self, dir: Vec3, rotation: f32) -> Vec3 {
        let (u, v) = Self::dir_to_uv(dir, rotation);
        let (x, y) = self.texel(u, v);
        self.pixels[y * self.width + x]
    }

    /// Radianza interpolata bilinearmente (solo per lo sfondo visto dalla camera).
    pub fn radiance_smooth(&self, dir: Vec3, rotation: f32) -> Vec3 {
        let (u, v) = Self::dir_to_uv(dir, rotation);
        let fx = u * self.width as f32 - 0.5;
        let fy = (v * self.height as f32 - 0.5).clamp(0.0, (self.height - 1) as f32);
        let (x0, y0) = (fx.floor(), fy.floor());
        let (tx, ty) = (fx - x0, fy - y0);
        let wrap = |x: f32| (x as i64).rem_euclid(self.width as i64) as usize;
        let (xa, xb) = (wrap(x0), wrap(x0 + 1.0));
        let ya = y0 as usize;
        let yb = (ya + 1).min(self.height - 1);
        let p = |x: usize, y: usize| self.pixels[y * self.width + x];
        let top = p(xa, ya).lerp(p(xb, ya), tx);
        let bot = p(xa, yb).lerp(p(xb, yb), tx);
        top.lerp(bot, ty)
    }

    /// Campiona una direzione: (direzione, radianza, pdf in angolo solido).
    pub fn sample(&self, u1: f32, u2: f32, rotation: f32) -> Option<(Vec3, Vec3, f32)> {
        let (v, pdf_v, y) = self.marginal.sample(u1);
        let (u, pdf_u, x) = self.conditional[y].sample(u2);
        let theta = v * PI;
        let sin_t = theta.sin();
        if sin_t <= 0.0 {
            return None;
        }
        let pdf = pdf_u * pdf_v / (2.0 * PI * PI * sin_t);
        if pdf <= 0.0 {
            return None;
        }
        let dir = Self::uv_to_dir(u, v, rotation);
        Some((dir, self.pixels[y * self.width + x], pdf))
    }

    /// PDF in angolo solido del campionamento di `dir`.
    pub fn pdf(&self, dir: Vec3, rotation: f32) -> f32 {
        let (u, v) = Self::dir_to_uv(dir, rotation);
        let (x, y) = self.texel(u, v);
        let sin_dir = (v * PI).sin();
        if sin_dir <= 0.0 {
            return 0.0;
        }
        self.marginal.pdf(y) * self.conditional[y].pdf(x) / (2.0 * PI * PI * sin_dir)
    }
}

/// Cache di processo delle mappe caricate (le HDRI possono pesare centinaia di MB).
pub fn load_cached(path: &Path) -> anyhow::Result<Arc<EnvironmentMap>> {
    static CACHE: OnceLock<Mutex<HashMap<PathBuf, Arc<EnvironmentMap>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(m) = cache.lock().unwrap().get(path) {
        return Ok(m.clone());
    }
    let t0 = std::time::Instant::now();
    let map = Arc::new(EnvironmentMap::load(path)?);
    tracing::info!(
        "HDRI caricata: {} ({}×{}) in {:.0} ms",
        path.display(),
        map.width,
        map.height,
        t0.elapsed().as_secs_f64() * 1e3
    );
    let mut c = cache.lock().unwrap();
    // Si tiene in memoria solo l'ultima mappa usata oltre a quella nuova.
    if c.len() >= 2 {
        c.clear();
    }
    c.insert(path.to_path_buf(), map.clone());
    Ok(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_math::{sampling, Pcg32};

    fn test_map() -> EnvironmentMap {
        // Cielo debole con un "sole" molto luminoso: il caso tipico delle HDRI reali.
        let (w, h) = (64, 32);
        let mut px = vec![Vec3::splat(0.2); w * h];
        for y in 6..9 {
            for x in 40..43 {
                px[y * w + x] = Vec3::new(500.0, 450.0, 400.0);
            }
        }
        EnvironmentMap::from_pixels(w, h, px)
    }

    #[test]
    fn uv_dir_roundtrip() {
        for (u, v) in [(0.1, 0.2), (0.5, 0.5), (0.9, 0.8), (0.75, 0.3)] {
            for rot in [0.0, 1.0] {
                let d = EnvironmentMap::uv_to_dir(u, v, rot);
                let (u2, v2) = EnvironmentMap::dir_to_uv(d, rot);
                assert!((u - u2).abs() < 1e-4 && (v - v2).abs() < 1e-4, "{u},{v} -> {u2},{v2}");
            }
        }
        // Il centro dell'immagine guarda verso -Z.
        let d = EnvironmentMap::uv_to_dir(0.5, 0.5, 0.0);
        assert!((d - (-Vec3::Z)).length() < 1e-5);
    }

    /// La PDF restituita da `sample` coincide con `pdf(dir)` e integra a 1.
    #[test]
    fn pdf_consistent_and_normalized() {
        let m = test_map();
        let mut rng = Pcg32::new(3, 4);
        for _ in 0..2000 {
            let Some((d, _, p)) = m.sample(rng.next_f32(), rng.next_f32(), 0.7) else {
                continue;
            };
            let p2 = m.pdf(d, 0.7);
            assert!((p - p2).abs() / p < 1e-2, "{p} vs {p2}");
        }
        let n = 400_000;
        let mut integral = 0.0f64;
        for _ in 0..n {
            let d = sampling::uniform_sphere(rng.next_f32(), rng.next_f32());
            integral += (m.pdf(d, 0.7) / sampling::UNIFORM_SPHERE_PDF) as f64;
        }
        let integral = integral / n as f64;
        assert!((integral - 1.0).abs() < 0.02, "integrale {integral}");
    }

    /// Stima dell'irradianza su un piano: campionamento per importanza e
    /// campionamento uniforme devono concordare (il primo con molta meno varianza).
    #[test]
    fn importance_sampling_is_unbiased() {
        let m = test_map();
        let n = Vec3::Y;
        let mut rng = Pcg32::new(8, 1);
        let k = 2_000_000;
        let (mut e_uni, mut e_is) = (0.0f64, 0.0f64);
        for _ in 0..k {
            let d = sampling::uniform_sphere(rng.next_f32(), rng.next_f32());
            e_uni += (luminance(m.radiance(d, 0.0)) * d.dot(n).max(0.0) / sampling::UNIFORM_SPHERE_PDF) as f64;
            if let Some((d, l, p)) = m.sample(rng.next_f32(), rng.next_f32(), 0.0) {
                e_is += (luminance(l) * d.dot(n).max(0.0) / p) as f64;
            }
        }
        let (e_uni, e_is) = (e_uni / k as f64, e_is / k as f64);
        assert!((e_uni - e_is).abs() / e_is < 0.03, "uniforme {e_uni} vs importanza {e_is}");
    }
}
