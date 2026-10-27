//! Esportazione delle immagini renderizzate.

use anyhow::Context;
use forge_render_core::{tonemap_to_rgba8, Frame, ToneMapping};
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

/// Applica tone mapping al frame e lo salva come PNG.
pub fn save_frame_png(path: &Path, frame: &Frame, tone: ToneMapping, exposure_ev: f32) -> anyhow::Result<()> {
    let rgba = tonemap_to_rgba8(&frame.linear, tone, exposure_ev);
    save_png(path, frame.width, frame.height, &rgba)
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
}
