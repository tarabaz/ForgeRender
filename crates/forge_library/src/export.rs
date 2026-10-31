//! Esportazione di un materiale in un file zip: `materiale.json` (tutte le impostazioni) e le
//! mappe, con i percorsi resi relativi ai file dello zip. È lo stesso formato dei materiali
//! predefiniti a dati (`assets/materiali-…/<id>/materiale.json` + mappe): scompattato in una
//! cartella si legge con [`forge_scene::material::data_presets`].

use anyhow::{bail, Context};
use forge_scene::Material;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Nome del file delle impostazioni dentro lo zip.
pub const SETTINGS_FILE: &str = "materiale.json";

/// Scrive lo zip del materiale in `out`. Le mappe mancanti fermano l'esportazione (uno zip
/// incompleto non serve). Restituisce quante mappe contiene.
pub fn material_zip(m: &Material, out: &Path) -> anyhow::Result<usize> {
    let mut m = m.clone();
    let mut files: Vec<(String, PathBuf)> = Vec::new();
    for slot in m.texture_slots_mut().into_iter() {
        let Some(path) = slot.clone() else { continue };
        if !path.is_file() {
            bail!("mappa non trovata: {}", path.display());
        }
        // Stesso file in più slot (es. una ARM): una copia sola.
        if let Some((name, _)) = files.iter().find(|(_, p)| *p == path) {
            *slot = Some(PathBuf::from(name));
            continue;
        }
        let base = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "mappa".into());
        // Nomi uguali da cartelle diverse: si numerano.
        let mut name = base.clone();
        let mut k = 2;
        while files.iter().any(|(n, _)| n.eq_ignore_ascii_case(&name)) || name.eq_ignore_ascii_case(SETTINGS_FILE) {
            name = format!("{k}_{base}");
            k += 1;
        }
        *slot = Some(PathBuf::from(&name));
        files.push((name, path));
    }
    let json = serde_json::to_string_pretty(&m)? + "\n";
    if let Some(d) = out.parent() {
        std::fs::create_dir_all(d).with_context(|| format!("cartella {}", d.display()))?;
    }
    // File temporaneo e rinomina: uno zip a metà non resta mai con il nome giusto.
    let tmp = out.with_extension("zip.tmp");
    let write = || -> anyhow::Result<()> {
        let mut z = zip::ZipWriter::new(std::fs::File::create(&tmp).with_context(|| format!("{}", tmp.display()))?);
        let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        z.start_file(SETTINGS_FILE, opts)?;
        z.write_all(json.as_bytes())?;
        for (name, path) in &files {
            // Le immagini sono già compresse: si salvano così come sono.
            let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
            z.start_file(name.as_str(), stored)?;
            z.write_all(&std::fs::read(path).with_context(|| format!("{}", path.display()))?)?;
        }
        z.finish()?;
        Ok(())
    };
    if let Err(e) = write() {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    std::fs::rename(&tmp, out).with_context(|| format!("{}", out.display()))?;
    Ok(files.len())
}

/// Nome di file proposto per lo zip del materiale (dall'id, senza caratteri strani).
pub fn suggested_name(m: &Material) -> String {
    let s: String =
        m.id.0
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .collect();
    format!("{}.zip", if s.is_empty() { "materiale".into() } else { s })
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_scene::MaterialId;

    /// Zip con le impostazioni e le mappe (nomi relativi, doppioni numerati, stessa mappa in due
    /// slot una volta sola); scompattato in una cartella si rilegge come predefinito a dati
    /// con tutti i valori e le mappe al loro posto.
    #[test]
    fn zip_round_trips_as_a_data_preset() {
        let root = std::env::temp_dir().join(format!("forge_export_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (a, b) = (root.join("a"), root.join("b"));
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        std::fs::write(a.join("colore.png"), b"colore").unwrap();
        std::fs::write(b.join("colore.png"), b"rugosita").unwrap();
        std::fs::write(a.join("arm.png"), b"arm").unwrap();
        let m = Material {
            id: MaterialId::new("mio-acciaio"),
            name: "Mio acciaio".into(),
            roughness: 0.37,
            bump_mm: 2.5,
            base_color_texture: Some(a.join("colore.png")),
            roughness_texture: Some(b.join("colore.png")),
            metallic_texture: Some(a.join("arm.png")),
            ao_texture: Some(a.join("arm.png")),
            mapping: forge_scene::TextureMapping {
                fit_object: true,
                scale: 0.5,
                ..Default::default()
            },
            ..Default::default()
        };
        let out = root.join("uscita").join(suggested_name(&m));
        assert_eq!(material_zip(&m, &out).unwrap(), 3);
        // Scompatta come farà l'inserimento dei predefiniti.
        let dir = root.join("predefiniti").join("mio-acciaio");
        std::fs::create_dir_all(&dir).unwrap();
        let mut z = zip::ZipArchive::new(std::fs::File::open(&out).unwrap()).unwrap();
        z.extract(&dir).unwrap();
        let loaded = forge_scene::material::data_presets(&root.join("predefiniti"));
        assert_eq!(loaded.len(), 1);
        let l = &loaded[0];
        assert_eq!(l.name, "Mio acciaio");
        assert!((l.roughness - 0.37).abs() < 1e-6 && (l.bump_mm - 2.5).abs() < 1e-6);
        assert!(l.mapping.fit_object && (l.mapping.scale - 0.5).abs() < 1e-6);
        let read = |p: &Option<PathBuf>| std::fs::read(p.as_ref().unwrap()).unwrap();
        assert_eq!(read(&l.base_color_texture), b"colore");
        assert_eq!(read(&l.roughness_texture), b"rugosita");
        assert_eq!(l.metallic_texture, l.ao_texture, "stessa mappa, un file solo");
        // Una mappa mancante ferma l'esportazione senza lasciare file.
        let mut broken = m.clone();
        broken.normal_texture = Some(root.join("non-c-e.png"));
        let out2 = root.join("rotto.zip");
        assert!(material_zip(&broken, &out2).is_err());
        assert!(!out2.exists() && !out2.with_extension("zip.tmp").exists());
    }
}
