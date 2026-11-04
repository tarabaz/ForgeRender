//! Scarica materiali CC0 (Poly Haven, ambientCG) a 2K e li salva come zip di "Esporta zip…",
//! una cartella per categoria: servono a sostituire nella libreria di serie i materiali che non
//! si possono ridistribuire. Il download passa per una libreria temporanea, come nell'app.
//!
//! File dell'elenco, una riga per materiale (`#` = commento):
//! ```text
//! <ph|acg> <id del sito> <cartella di categoria> <nome dello zip>
//! ```
//!
//! ```text
//! cargo run --release -p forge_library --example sostituti_cc0 -- <elenco.txt> <uscita>
//! ```

use forge_library::online::{self, AssetKind, Provider};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    anyhow::ensure!(args.len() == 2, "uso: sostituti_cc0 <elenco.txt> <uscita>");
    let out = PathBuf::from(&args[1]);
    let tmp = out.join("_libreria_temporanea");
    forge_library::paths::set_custom_root(Some(tmp.clone()));
    forge_library::paths::ensure()?;
    let ph = online::catalog(Provider::PolyHaven, AssetKind::Material, false)?;
    let acg = online::catalog(Provider::AmbientCg, AssetKind::Material, false)?;
    let mut errors = 0;
    for line in std::fs::read_to_string(&args[0])?.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.splitn(4, ' ').collect();
        anyhow::ensure!(parts.len() == 4, "riga non valida: {line}");
        let (cat, file) = (parts[2], parts[3]);
        let zip = out.join(cat.replace('_', " ")).join(format!("{file}.zip"));
        if zip.is_file() {
            println!("già fatto: {}", zip.display());
            continue;
        }
        let list = if parts[0] == "ph" { &ph } else { &acg };
        let Some(asset) = list.iter().find(|a| a.id == parts[1]) else {
            eprintln!("ERRORE: {} non trovato nel catalogo", parts[1]);
            errors += 1;
            continue;
        };
        print!("{} {} … ", asset.provider.label(), asset.id);
        match online::download_material(asset, "2K", &|_| {}) {
            Ok(m) => {
                std::fs::create_dir_all(zip.parent().unwrap())?;
                let n = forge_library::export::material_zip(&m.material, &zip)?;
                println!("{n} mappe → {}", zip.display());
            }
            Err(e) => {
                println!("ERRORE {e:#}");
                errors += 1;
            }
        }
    }
    std::fs::remove_dir_all(&tmp).ok();
    anyhow::ensure!(errors == 0, "{errors} materiali non scaricati");
    Ok(())
}
