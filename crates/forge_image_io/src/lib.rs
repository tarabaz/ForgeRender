//! Esportazione delle immagini renderizzate.

use anyhow::Context;
use forge_render_core::{tonemap_to_rgb16, tonemap_to_rgba8, Frame, ToneMapping};
use std::path::{Path, PathBuf};

/// Scrive un file passando da un temporaneo nella stessa cartella e poi
/// rinominandolo: un crash a metà scrittura non lascia mai un file corrotto.
pub fn write_atomic(path: &Path, write: impl FnOnce(&Path) -> anyhow::Result<()>) -> anyhow::Result<()> {
    let tmp = tmp_path(path);
    write(&tmp).with_context(|| format!("scrittura di {}", tmp.display()))?;
    std::fs::rename(&tmp, path).with_context(|| format!("rinomina in {}", path.display()))?;
    Ok(())
}

fn tmp_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().map(|n| n.to_os_string()).unwrap_or_default();
    name.push(".tmp");
    path.with_file_name(name)
}

/// Salva un'immagine RGBA8 come PNG.
pub fn save_png(path: &Path, width: u32, height: u32, rgba: &[u8]) -> anyhow::Result<()> {
    anyhow::ensure!(rgba.len() == (width * height * 4) as usize, "dimensioni dell'immagine non coerenti");
    write_atomic(path, |tmp| {
        image::save_buffer_with_format(tmp, rgba, width, height, image::ColorType::Rgba8, image::ImageFormat::Png)?;
        Ok(())
    })
}

/// Applica tone mapping al frame e lo salva come PNG. `denoise_strength` miscela
/// l'immagine grezza con quella ripulita, se presente (0 = grezza, 1 = ripulita).
pub fn save_frame_png(path: &Path, frame: &Frame, tone: ToneMapping, exposure_ev: f32, denoise_strength: f32) -> anyhow::Result<()> {
    let rgba = tonemap_to_rgba8(&frame.display(denoise_strength), tone, exposure_ev);
    save_png(path, frame.width, frame.height, &rgba)
}

/// Formati in cui si può esportare un render (un solo comando "Esporta…", il formato si
/// sceglie nella finestra di salvataggio).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    /// 8 bit senza perdita: il formato predefinito.
    Png,
    /// 8 bit compresso (qualità 92): file piccoli per web e social.
    Jpeg,
    /// 16 bit per canale, dopo il tone mapping: per il fotoritocco.
    Tiff16,
    /// Valori lineari a 32 bit (radianza × esposizione, senza tone mapping): per compositing.
    Exr,
}

impl ExportFormat {
    pub const ALL: [ExportFormat; 4] = [ExportFormat::Png, ExportFormat::Jpeg, ExportFormat::Tiff16, ExportFormat::Exr];

    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            ExportFormat::Png => &["png"],
            ExportFormat::Jpeg => &["jpg", "jpeg"],
            ExportFormat::Tiff16 => &["tif", "tiff"],
            ExportFormat::Exr => &["exr"],
        }
    }

    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        Self::ALL.into_iter().find(|f| f.extensions().contains(&ext.as_str()))
    }
}

/// Salva il frame nel formato indicato (vedi [`ExportFormat`]).
pub fn save_frame(
    path: &Path,
    format: ExportFormat,
    frame: &Frame,
    tone: ToneMapping,
    exposure_ev: f32,
    denoise_strength: f32,
) -> anyhow::Result<()> {
    let (w, h) = (frame.width, frame.height);
    let linear = frame.display(denoise_strength);
    match format {
        ExportFormat::Png => save_png(path, w, h, &tonemap_to_rgba8(&linear, tone, exposure_ev)),
        ExportFormat::Jpeg => {
            let rgba = tonemap_to_rgba8(&linear, tone, exposure_ev);
            let rgb: Vec<u8> = rgba.as_chunks::<4>().0.iter().flat_map(|p| [p[0], p[1], p[2]]).collect();
            write_atomic(path, |tmp| {
                let file = std::io::BufWriter::new(std::fs::File::create(tmp)?);
                image::codecs::jpeg::JpegEncoder::new_with_quality(file, 92).encode(&rgb, w, h, image::ExtendedColorType::Rgb8)?;
                Ok(())
            })
        }
        ExportFormat::Tiff16 => {
            let rgb = tonemap_to_rgb16(&linear, tone, exposure_ev);
            let img = image::ImageBuffer::<image::Rgb<u16>, _>::from_raw(w, h, rgb).context("dimensioni dell'immagine non coerenti")?;
            write_atomic(path, |tmp| {
                img.save_with_format(tmp, image::ImageFormat::Tiff)?;
                Ok(())
            })
        }
        ExportFormat::Exr => {
            let mul = 2f32.powf(exposure_ev);
            let data: Vec<f32> = linear
                .iter()
                .flat_map(|p| p.map(|v| if v.is_finite() { v * mul } else { 0.0 }))
                .collect();
            let img = image::ImageBuffer::<image::Rgb<f32>, _>::from_raw(w, h, data).context("dimensioni dell'immagine non coerenti")?;
            write_atomic(path, |tmp| {
                img.save_with_format(tmp, image::ImageFormat::OpenExr)?;
                Ok(())
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_roundtrip() {
        let dir = std::env::temp_dir().join(format!("forge_png_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("t.png");
        let rgba: Vec<u8> = (0..4 * 3 * 2).map(|i| (i * 10) as u8).collect();
        save_png(&p, 3, 2, &rgba).unwrap();
        let img = image::open(&p).unwrap().to_rgba8();
        assert_eq!(img.dimensions(), (3, 2));
        assert_eq!(img.as_raw(), &rgba);
        assert!(!tmp_path(&p).exists());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn every_export_format_writes_a_readable_file() {
        let dir = std::env::temp_dir().join(format!("forge_export_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let frame = Frame {
            width: 4,
            height: 2,
            samples: 1,
            linear: std::sync::Arc::new(vec![[0.5, 0.25, 2.0]; 8]),
            job_id: 0,
            denoised: None,
            denoised_samples: 0,
            guides: None,
            adaptive: None,
        };
        for f in ExportFormat::ALL {
            let p = dir.join(format!("t.{}", f.extensions()[0]));
            save_frame(&p, f, &frame, ToneMapping::AcesFilmic, 0.0, 1.0).unwrap();
            assert_eq!(ExportFormat::from_path(&p), Some(f));
            let img = image::open(&p).unwrap();
            assert_eq!((img.width(), img.height()), (4, 2), "{f:?}");
            if f == ExportFormat::Exr {
                // Valori lineari conservati, anche sopra 1.
                let px = img.to_rgb32f().get_pixel(0, 0).0;
                assert!((px[2] - 2.0).abs() < 1e-3, "{px:?}");
            }
        }
        std::fs::remove_dir_all(dir).ok();
    }
}
