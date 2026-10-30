//! Prepara le mappe dei materiali predefiniti in `assets/materiali/<id>/` (1024 px, JPEG):
//! `colore.jpg` (sRGB), `rugosita.jpg` (lineare, grigio) e `normal.jpg` (OpenGL, lineare).
//!
//! Alcune vengono dalla libreria Poly Haven scaricata (CC0), altre si generano qui.
//!
//! ```text
//! cargo run --release -p forge_render_core --example materiali_predefiniti -- <Libreria\Materiali> <assets\materiali>
//! ```

use forge_render_core::texture::Texture;
use image::{imageops::FilterType, GrayImage, RgbImage};
use std::path::{Path, PathBuf};

const SIZE: u32 = 1024;

/// Predefiniti la cui mappa colore diventa grigia (il colore lo dà il materiale).
const NEUTRAL: [&str; 1] = ["pelle-nera"];

/// Materiale Poly Haven della libreria: id del predefinito, cartella e prefisso dei file.
const FROM_POLYHAVEN: [(&str, &str, &str); 2] = [
    ("legno-rovere", "Oak Veneer 01 2K", "oak_veneer_01"),
    ("pelle-nera", "Leather White 2K", "leather_white"),
];

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    anyhow::ensure!(
        args.len() == 2,
        "uso: materiali_predefiniti <Libreria\\Materiali> <assets\\materiali>"
    );
    let (lib, out) = (PathBuf::from(&args[0]), PathBuf::from(&args[1]));
    for (id, folder, prefix) in FROM_POLYHAVEN {
        let src = lib.join(folder);
        let dst = out.join(id);
        std::fs::create_dir_all(&dst)?;
        let mut color = image::open(src.join(format!("{prefix}_diff_2k.jpg")))?.to_rgb8();
        // Tinte date dal colore base del predefinito (pelle nera): la mappa resta solo grana, neutra.
        if NEUTRAL.contains(&id) {
            for p in color.pixels_mut() {
                let l = (0.2126 * p.0[0] as f32 + 0.7152 * p.0[1] as f32 + 0.0722 * p.0[2] as f32).round() as u8;
                p.0 = [l; 3];
            }
        }
        save_rgb(&resize(&color), &dst.join("colore.jpg"), 90)?;
        let rough = load_linear(&src.join(format!("{prefix}_rough_2k.exr")))?;
        save_gray(&resize(&rough), &dst.join("rugosita.jpg"), 92)?;
        let normal = load_linear(&src.join(format!("{prefix}_nor_gl_2k.exr")))?;
        save_rgb(&resize(&normal), &dst.join("normal.jpg"), 95)?;
        println!("{id}: da {folder}");
    }
    let dst = out.join("marmo-bianco");
    std::fs::create_dir_all(&dst)?;
    save_rgb(&marble(SIZE), &dst.join("colore.jpg"), 92)?;
    println!("marmo-bianco: generato");
    Ok(())
}

/// Mappa lineare (anche EXR a un canale) come 8 bit per canale, senza conversione sRGB.
fn load_linear(p: &Path) -> anyhow::Result<RgbImage> {
    let t = Texture::load(p, false)?;
    let mut img = RgbImage::new(t.width as u32, t.height as u32);
    for (px, c) in img.pixels_mut().zip(t.data()) {
        px.0 = *c;
    }
    Ok(img)
}

fn resize(img: &RgbImage) -> RgbImage {
    image::imageops::resize(img, SIZE, SIZE, FilterType::Lanczos3)
}

fn save_rgb(img: &RgbImage, p: &Path, quality: u8) -> anyhow::Result<()> {
    let mut f = std::io::BufWriter::new(std::fs::File::create(p)?);
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut f, quality).encode_image(img)?;
    Ok(())
}

fn save_gray(img: &RgbImage, p: &Path, quality: u8) -> anyhow::Result<()> {
    let g = GrayImage::from_fn(img.width(), img.height(), |x, y| image::Luma([img.get_pixel(x, y).0[0]]));
    let mut f = std::io::BufWriter::new(std::fs::File::create(p)?);
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut f, quality).encode_image(&g)?;
    Ok(())
}

// --- Marmo bianco (venato, ripetibile senza giunture) ---

fn hash(x: i64, y: i64, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343) ^ (y as u32).wrapping_mul(0xd816_3841) ^ seed.wrapping_mul(0xcb1a_b31f);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^= h >> 15;
    (h & 0xff_ffff) as f32 / 0xff_ffff as f32
}

/// Rumore a valori su una griglia di `period` celle che si ripete (coordinate 0..1).
fn noise(u: f32, v: f32, period: i64, seed: u32) -> f32 {
    let (x, y) = (u * period as f32, v * period as f32);
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let s = |t: f32| t * t * (3.0 - 2.0 * t);
    let (sx, sy) = (s(fx), s(fy));
    let at = |dx: i64, dy: i64| hash((x0 as i64 + dx).rem_euclid(period), (y0 as i64 + dy).rem_euclid(period), seed);
    let a = at(0, 0) + (at(1, 0) - at(0, 0)) * sx;
    let b = at(0, 1) + (at(1, 1) - at(0, 1)) * sx;
    a + (b - a) * sy
}

/// Somma di ottave (−0.5..0.5 circa), periodi che raddoppiano: resta ripetibile.
fn fbm(u: f32, v: f32, base: i64, octaves: u32, seed: u32) -> f32 {
    let (mut sum, mut amp, mut norm) = (0.0, 1.0, 0.0);
    for o in 0..octaves {
        sum += (noise(u, v, base << o, seed + o) - 0.5) * amp;
        norm += amp;
        amp *= 0.5;
    }
    sum / norm
}

fn marble(size: u32) -> RgbImage {
    use std::f32::consts::TAU;
    let srgb = |c: f32| (c.clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0 + 0.5) as u8;
    RgbImage::from_fn(size, size, |x, y| {
        let (u, v) = (x as f32 / size as f32, y as f32 / size as f32);
        // Venature principali in diagonale, deformate dal rumore; altre più sottili e chiare.
        let w1 = fbm(u, v, 3, 6, 1) * 2.4;
        let w2 = fbm(u, v, 5, 6, 7) * 2.0;
        let main = 1.0 - (TAU * (u + 2.0 * v + w1)).sin().abs();
        let thin = 1.0 - (TAU * (3.0 * u - v + w2)).sin().abs();
        // Linea scura al centro, alone sfumato attorno (il marmo vero non ha bordi netti).
        let w3 = fbm(u, v, 4, 6, 21) * 2.2;
        let faint = 1.0 - (TAU * (u - 2.0 * v + w3)).sin().abs();
        let vein = main.powf(16.0) * 0.6 + main.powf(4.0) * 0.18 + thin.powf(40.0) * 0.22 + faint.powf(6.0) * 0.06;
        // Nuvole morbide nel fondo e un leggero tono caldo.
        let cloud = fbm(u, v, 4, 5, 13) * 0.08;
        let base = [0.80, 0.79, 0.76].map(|c: f32| c + cloud);
        let grey = [0.32, 0.33, 0.35];
        let c = [0, 1, 2].map(|k| base[k] * (1.0 - vein) + grey[k] * vein);
        image::Rgb(c.map(srgb))
    })
}
