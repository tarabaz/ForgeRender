//! Texture bitmap per i materiali (colore base, roughness, normal map).
//!
//! Memorizzate a 8 bit per canale (la decodifica sRGB → lineare avviene al
//! campionamento tramite tabella), con filtro bilineare e ripetizione (tiling).
//!
//! - Le immagini in virgola mobile (EXR, HDR) sono lineari: per il colore base si
//!   codificano in sRGB, così non vengono decodificate due volte.
//! - Le height map del displacement ([`load_precise_cached`]) tengono anche una copia a
//!   16 bit di un canale quando la sorgente ha più di 8 bit (PNG/TIFF a 16 bit, EXR):
//!   evita i gradini sulla superficie spostata.

use anyhow::Context;
use forge_math::{Vec2, Vec3};
use forge_scene::material::{linear_to_srgb, srgb_to_linear};
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
    /// Canale rosso (lineare) a 16 bit, solo per le height map caricate con precisione.
    hi: Option<Vec<u16>>,
}

fn srgb_lut() -> &'static [f32; 256] {
    static LUT: OnceLock<[f32; 256]> = OnceLock::new();
    LUT.get_or_init(|| std::array::from_fn(|i| srgb_to_linear(i as f32 / 255.0)))
}

impl Texture {
    pub fn from_rgb8(width: usize, height: usize, data: Vec<[u8; 3]>, srgb: bool) -> Self {
        assert_eq!(data.len(), width * height);
        Self {
            width,
            height,
            data,
            srgb,
            hi: None,
        }
    }

    pub fn data(&self) -> &[[u8; 3]] {
        &self.data
    }

    pub fn is_srgb(&self) -> bool {
        self.srgb
    }

    pub fn load(path: &Path, srgb: bool) -> anyhow::Result<Self> {
        Self::load_with(path, srgb, false)
    }

    /// Ha una copia a 16 bit (height map caricata con precisione).
    pub fn is_precise(&self) -> bool {
        self.hi.is_some()
    }

    fn load_with(path: &Path, srgb: bool, precise: bool) -> anyhow::Result<Self> {
        use image::DynamicImage as D;
        let img = image::open(path).with_context(|| format!("lettura texture {}", path.display()))?;
        let (w, h) = (img.width(), img.height());
        anyhow::ensure!(w > 0 && h > 0, "texture vuota");
        let float = matches!(img, D::ImageRgb32F(_) | D::ImageRgba32F(_));
        let deep = float || matches!(img, D::ImageLuma16(_) | D::ImageLumaA16(_) | D::ImageRgb16(_) | D::ImageRgba16(_));
        let hi = (precise && deep && !srgb).then(|| {
            let f = img.to_rgb32f();
            f.pixels().map(|p| (p.0[0].clamp(0.0, 1.0) * 65535.0).round() as u16).collect()
        });
        let data = if float && srgb {
            // Colore lineare (EXR/HDR) → sRGB a 8 bit.
            let f = img.to_rgb32f();
            f.pixels()
                .map(|p| p.0.map(|c| (linear_to_srgb(c.clamp(0.0, 1.0)) * 255.0).round() as u8))
                .collect()
        } else {
            img.into_rgb8().pixels().map(|p| p.0).collect()
        };
        let mut t = Self::from_rgb8(w as usize, h as usize, data, srgb);
        t.hi = hi;
        Ok(t)
    }

    #[inline]
    fn texel(&self, x: i64, y: i64) -> Vec3 {
        let x = x.rem_euclid(self.width as i64) as usize;
        let y = y.rem_euclid(self.height as i64) as usize;
        if let Some(hi) = &self.hi {
            return Vec3::splat(hi[y * self.width + x] as f32 / 65535.0);
        }
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

type TextureCache = Mutex<HashMap<(PathBuf, bool, bool), Arc<Texture>>>;

/// Cache di processo delle texture caricate, per percorso e spazio colore.
pub fn load_cached(path: &Path, srgb: bool) -> anyhow::Result<Arc<Texture>> {
    cached(path, srgb, false)
}

/// Height map per il displacement: con una sorgente a più di 8 bit tiene i 16 bit.
pub fn load_precise_cached(path: &Path) -> anyhow::Result<Arc<Texture>> {
    cached(path, false, true)
}

fn cached(path: &Path, srgb: bool, precise: bool) -> anyhow::Result<Arc<Texture>> {
    static CACHE: OnceLock<TextureCache> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let key = (path.to_path_buf(), srgb, precise);
    if let Some(t) = cache.lock().unwrap().get(&key) {
        return Ok(t.clone());
    }
    let t = Arc::new(Texture::load_with(path, srgb, precise)?);
    tracing::info!(
        "texture caricata: {} ({}×{}{})",
        path.display(),
        t.width,
        t.height,
        if t.is_precise() { ", 16 bit" } else { "" }
    );
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

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("forge_tex_{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d.join(name)
    }

    /// Colore base in EXR (lineare): 0.5 resta 0.5, non viene decodificato due volte (0.214).
    #[test]
    fn exr_color_is_linear() {
        let p = tmp("colore.exr");
        image::Rgb32FImage::from_pixel(2, 2, image::Rgb([0.5, 0.25, 1.0])).save(&p).unwrap();
        let t = Texture::load(&p, true).unwrap();
        let c = t.sample(Vec2::splat(0.5));
        assert!(
            (c.x - 0.5).abs() < 0.01 && (c.y - 0.25).abs() < 0.01 && (c.z - 1.0).abs() < 0.01,
            "{c:?}"
        );
        // Mappa di dati in EXR (roughness): valori lineari a 8 bit.
        let r = Texture::load(&p, false).unwrap();
        assert!((r.sample(Vec2::splat(0.5)).x - 0.5).abs() < 0.003);
        assert!(!r.is_precise());
    }

    /// Height a 16 bit (PNG) ed EXR: con la precisione i valori fra due livelli a 8 bit restano.
    #[test]
    fn precise_height_keeps_16_bits() {
        let v = 1000u16; // 0.01526: tra i livelli 8 bit 3/255 (0.01176) e 4/255 (0.01569)
        let p16 = tmp("height16.png");
        image::ImageBuffer::<image::Luma<u16>, _>::from_pixel(2, 2, image::Luma([v]))
            .save(&p16)
            .unwrap();
        let exact = v as f32 / 65535.0;
        let hp = Texture::load_with(&p16, false, true).unwrap();
        assert!(hp.is_precise());
        assert!((hp.sample(Vec2::splat(0.5)).x - exact).abs() < 1e-5);
        let lp = Texture::load(&p16, false).unwrap();
        assert!((lp.sample(Vec2::splat(0.5)).x - exact).abs() > 1e-4, "a 8 bit si perde");
        let pexr = tmp("height.exr");
        image::Rgb32FImage::from_pixel(2, 2, image::Rgb([exact; 3])).save(&pexr).unwrap();
        let he = Texture::load_with(&pexr, false, true).unwrap();
        assert!((he.sample(Vec2::splat(0.5)).x - exact).abs() < 2e-5);
        // Una PNG a 8 bit non ha niente da guadagnare: nessuna copia a 16 bit.
        let p8 = tmp("height8.png");
        image::GrayImage::from_pixel(2, 2, image::Luma([4])).save(&p8).unwrap();
        assert!(!Texture::load_with(&p8, false, true).unwrap().is_precise());
    }

    #[test]
    fn srgb_decoding() {
        let t = Texture::from_rgb8(1, 1, vec![[128, 128, 128]], true);
        assert!((t.sample(Vec2::splat(0.5)).x - 0.2158).abs() < 1e-3);
    }
}
