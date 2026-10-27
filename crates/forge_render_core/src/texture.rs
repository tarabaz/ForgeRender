//! Texture bitmap per i materiali (colore base, roughness, normal map).
//!
//! Memorizzate a 8 bit per canale (la decodifica sRGB → lineare avviene al
//! campionamento tramite tabella), con filtro bilineare e ripetizione (tiling).

use anyhow::Context;
use forge_math::{Vec2, Vec3};
use forge_scene::material::srgb_to_linear;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Debug)]
pub struct Texture {
    pub width: usize,
    pub height: usize,
    data: Vec<[u8; 3]>,
    /// Vero per il colore base (dati in sRGB), falso per roughness e normal map (dati lineari).
    srgb: bool,
}

fn srgb_lut() -> &'static [f32; 256] {
    static LUT: OnceLock<[f32; 256]> = OnceLock::new();
    LUT.get_or_init(|| std::array::from_fn(|i| srgb_to_linear(i as f32 / 255.0)))
}

impl Texture {
    pub fn from_rgb8(width: usize, height: usize, data: Vec<[u8; 3]>, srgb: bool) -> Self {
        assert_eq!(data.len(), width * height);
        Self { width, height, data, srgb }
    }

    pub fn data(&self) -> &[[u8; 3]] {
        &self.data
    }

    pub fn is_srgb(&self) -> bool {
        self.srgb
    }

    pub fn load(path: &Path, srgb: bool) -> anyhow::Result<Self> {
        let img = image::open(path)
            .with_context(|| format!("lettura texture {}", path.display()))?
            .into_rgb8();
        let (w, h) = img.dimensions();
        anyhow::ensure!(w > 0 && h > 0, "texture vuota");
        let data = img.pixels().map(|p| p.0).collect();
        Ok(Self::from_rgb8(w as usize, h as usize, data, srgb))
    }

    #[inline]
    fn texel(&self, x: i64, y: i64) -> Vec3 {
        let x = x.rem_euclid(self.width as i64) as usize;
        let y = y.rem_euclid(self.height as i64) as usize;
        let p = self.data[y * self.width + x];
        if self.srgb {
            let lut = srgb_lut();
            Vec3::new(lut[p[0] as usize], lut[p[1] as usize], lut[p[2] as usize])
        } else {
            Vec3::new(p[0] as f32, p[1] as f32, p[2] as f32) / 255.0
        }
    }

    /// Campionamento bilineare con ripetizione. `uv` con origine in basso a sinistra (convenzione OBJ).
    pub fn sample(&self, uv: Vec2) -> Vec3 {
        let fx = uv.x * self.width as f32 - 0.5;
        let fy = (1.0 - uv.y) * self.height as f32 - 0.5;
        if !fx.is_finite() || !fy.is_finite() {
            return Vec3::ZERO;
        }
        let (x0, y0) = (fx.floor(), fy.floor());
        let (tx, ty) = (fx - x0, fy - y0);
        let (x0, y0) = (x0 as i64, y0 as i64);
        let top = self.texel(x0, y0).lerp(self.texel(x0 + 1, y0), tx);
        let bot = self.texel(x0, y0 + 1).lerp(self.texel(x0 + 1, y0 + 1), tx);
        top.lerp(bot, ty)
    }
}

type TextureCache = Mutex<HashMap<(PathBuf, bool), Arc<Texture>>>;

/// Cache di processo delle texture caricate, per percorso e spazio colore.
pub fn load_cached(path: &Path, srgb: bool) -> anyhow::Result<Arc<Texture>> {
    static CACHE: OnceLock<TextureCache> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let key = (path.to_path_buf(), srgb);
    if let Some(t) = cache.lock().unwrap().get(&key) {
        return Ok(t.clone());
    }
    let t = Arc::new(Texture::load(path, srgb)?);
    tracing::info!("texture caricata: {} ({}×{})", path.display(), t.width, t.height);
    let mut c = cache.lock().unwrap();
    // Limite grossolano per non trattenere texture non più usate all'infinito.
    if c.len() > 64 {
        c.clear();
    }
    c.insert(key, t.clone());
    Ok(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bilinear_and_wrap() {
        // 2×1: nero | bianco (lineare).
        let t = Texture::from_rgb8(2, 1, vec![[0, 0, 0], [255, 255, 255]], false);
        // Centri dei texel.
        assert!((t.sample(Vec2::new(0.25, 0.5)).x - 0.0).abs() < 1e-6);
        assert!((t.sample(Vec2::new(0.75, 0.5)).x - 1.0).abs() < 1e-6);
        // A metà fra i centri: interpolazione.
        assert!((t.sample(Vec2::new(0.5, 0.5)).x - 0.5).abs() < 1e-6);
        // Ripetizione: u e u+1 danno lo stesso valore.
        assert!((t.sample(Vec2::new(1.25, 0.5)) - t.sample(Vec2::new(0.25, 0.5))).length() < 1e-6);
        assert!((t.sample(Vec2::new(-0.75, 0.5)) - t.sample(Vec2::new(0.25, 0.5))).length() < 1e-6);
    }

    #[test]
    fn srgb_decoding() {
        let t = Texture::from_rgb8(1, 1, vec![[128, 128, 128]], true);
        assert!((t.sample(Vec2::splat(0.5)).x - 0.2158).abs() < 1e-3);
    }
}
