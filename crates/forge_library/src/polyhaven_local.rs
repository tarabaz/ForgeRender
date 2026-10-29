//! Import nella libreria dei materiali Poly Haven scaricati in locale con lo script
//! `tools/material-library-polyhaven/download_polyhaven_materials.py`.
//!
//! Formato dello script:
//! ```text
//! <radice>/index.csv, README.txt, errors.txt     (scritti solo a fine download)
//! <radice>/<id>/material.json                    manifest dello script (scritto dopo l'ultima mappa)
//! <radice>/<id>/maps/<file originali>            es. metal_plate_02_diff_2k.jpg, *_nor_gl_2k.exr, *.blend
//! ```
//! Si importano solo le cartelle con `material.json` (materiale completo): quella che lo
//! script sta scrivendo non si tocca. Nella cartella del materiale arrivano solo le mappe
//! collegate agli slot (vedi [`store::import_folder_with`]), il file MaterialX e il manifest
//! rinominato [`KEPT_MANIFEST`]; le mappe non usate (nor_dx, arm, rough_ao, bump, spec…) e
//! i file .blend/.gltf restano nella cartella dei download. Non si cancella niente.

use crate::online::OnlineAsset;
use crate::store::{self, LibraryMaterial, Transfer};
use anyhow::Context;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Manifest scritto dallo script in ogni cartella `<id>`.
pub const MANIFEST: &str = "material.json";
/// Nome del manifest conservato nella cartella del materiale della libreria.
pub const KEPT_MANIFEST: &str = "polyhaven_manifest.json";

#[derive(Clone, Debug, Deserialize)]
pub struct Manifest {
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub requested_resolution: String,
    #[serde(default)]
    pub maps: Vec<ManifestMap>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ManifestMap {
    #[serde(default)]
    pub map: String,
    #[serde(default)]
    pub resolution: String,
    #[serde(default)]
    pub file: String,
    #[serde(default)]
    pub status: String,
}

/// Voci dell'API che non sono mappe (formati per altri programmi).
const NON_MAP_ENTRIES: [&str; 6] = ["blend", "gltf", "mtlx", "usd", "fbx", "zip"];

impl Manifest {
    fn image_maps(&self) -> impl Iterator<Item = &ManifestMap> {
        self.maps
            .iter()
            .filter(|m| !NON_MAP_ENTRIES.contains(&m.map.to_ascii_lowercase().as_str()))
    }

    /// Risoluzione del materiale ("2K"): la più frequente tra le mappe, altrimenti quella chiesta.
    pub fn resolution(&self) -> String {
        let mut count: BTreeMap<String, usize> = BTreeMap::new();
        for m in self.image_maps().filter(|m| !m.resolution.is_empty()) {
            *count.entry(m.resolution.to_ascii_uppercase()).or_default() += 1;
        }
        count
            .into_iter()
            .max_by_key(|(_, n)| *n)
            .map(|(r, _)| r)
            .unwrap_or_else(|| self.requested_resolution.to_ascii_uppercase())
    }

    /// Provenienza nel formato dei materiali scaricati dall'app: "Poly Haven <id> (<RIS>)".
    pub fn source(&self) -> String {
        format!(
            "{} ({})",
            crate::online::source_key(crate::online::Provider::PolyHaven, &self.id),
            self.resolution()
        )
    }

    /// Nome del materiale nella libreria: nome Poly Haven + risoluzione.
    pub fn library_name(&self) -> String {
        let name = if self.name.trim().is_empty() { &self.id } else { self.name.trim() };
        format!("{name} {}", self.resolution())
    }
}

/// Cartella `<id>` completa (con il manifest).
#[derive(Clone, Debug)]
pub struct Candidate {
    pub dir: PathBuf,
    pub manifest: Manifest,
}

impl Candidate {
    fn maps_dir(&self) -> PathBuf {
        let m = self.dir.join("maps");
        if m.is_dir() {
            m
        } else {
            self.dir.clone()
        }
    }
}

/// Contenuto della cartella dei download.
#[derive(Debug, Default)]
pub struct Scan {
    pub complete: Vec<Candidate>,
    /// Cartelle senza manifest: download in corso o interrotto (non si toccano).
    pub incomplete: Vec<PathBuf>,
    /// Manifest illeggibili.
    pub invalid: Vec<(PathBuf, String)>,
}

/// Elenca le cartelle `<id>` della radice dei download, in ordine alfabetico.
pub fn scan(root: &Path) -> anyhow::Result<Scan> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(root)
        .with_context(|| format!("lettura di {}", root.display()))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    let mut scan = Scan::default();
    for dir in dirs {
        let mf = dir.join(MANIFEST);
        if !mf.is_file() {
            scan.incomplete.push(dir);
            continue;
        }
        match std::fs::read_to_string(&mf)
            .map_err(anyhow::Error::from)
            .and_then(|t| serde_json::from_str::<Manifest>(&t).map_err(anyhow::Error::from))
        {
            Ok(manifest) => scan.complete.push(Candidate { dir, manifest }),
            Err(e) => scan.invalid.push((dir, format!("{e:#}"))),
        }
    }
    Ok(scan)
}

/// Avvisi sul download da salvare nelle notes del materiale (mai valori inventati).
fn manifest_notes(c: &Candidate) -> Vec<String> {
    let m = &c.manifest;
    let requested = m.requested_resolution.to_ascii_lowercase();
    let mut notes = Vec::new();
    for map in &m.maps {
        if !map.file.is_empty() && !c.dir.join(map.file.replace('\\', "/")).is_file() {
            notes.push(format!("mappa \"{}\" elencata nel download ma assente ({})", map.map, map.file));
        }
        if !map.status.is_empty() && !["downloaded", "already_present"].contains(&map.status.as_str()) {
            notes.push(format!("mappa \"{}\": stato del download \"{}\"", map.map, map.status));
        }
    }
    for map in m.image_maps() {
        if !requested.is_empty() && !map.resolution.is_empty() && map.resolution.to_ascii_lowercase() != requested {
            notes.push(format!(
                "mappa \"{}\" a {} invece di {} (non disponibile su Poly Haven)",
                map.map,
                map.resolution.to_ascii_uppercase(),
                requested.to_ascii_uppercase()
            ));
        }
    }
    notes
}

/// Importa un materiale completo. `info`: la sua voce nel catalogo Poly Haven (lato reale,
/// categorie, tag), se c'è. Restituisce il materiale e quante copie difettose dello stesso
/// materiale ha sostituito.
pub fn import(c: &Candidate, info: Option<&OnlineAsset>, transfer: Transfer) -> anyhow::Result<(LibraryMaterial, usize)> {
    let mut notes = manifest_notes(c);
    let size_m = info.and_then(|a| a.size_m);
    if info.is_none() {
        notes.push("materiale non trovato nel catalogo Poly Haven: scala 1 m, nessuna categoria".into());
    } else if size_m.is_none() {
        notes.push("dimensioni reali non disponibili nel catalogo Poly Haven: scala 1 m".into());
    }
    let m = &c.manifest;
    let lib = store::import_folder_with(&c.maps_dir(), Some(&m.library_name()), size_m, &m.source(), transfer)?;
    let mat = &lib.material;
    for (slot, missing) in [
        ("colore", mat.base_color_texture.is_none()),
        ("normal", mat.normal_texture.is_none()),
        ("rugosità", mat.roughness_texture.is_none()),
    ] {
        if missing {
            notes.push(format!("nessuna mappa di {slot} tra i file scaricati"));
        }
    }
    // Manifest dello script: si conserva con il materiale.
    let from = c.dir.join(MANIFEST);
    let to = lib.dir.join(KEPT_MANIFEST);
    let kept = match transfer {
        Transfer::Copy => std::fs::copy(&from, &to).map(|_| ()),
        Transfer::Move => std::fs::rename(&from, &to),
    };
    if let Err(e) = kept {
        notes.push(format!("manifest del download non conservato ({e})"));
    }
    let mut lib = store::set_notes(&lib.dir, &notes)?;
    if let Some(a) = info {
        lib = store::set_catalog_info(&lib.dir, &a.categories, &a.tags)?;
    }
    let replaced = crate::online::remove_broken_copies(&lib);
    if transfer == Transfer::Move {
        // Solo cartelle rimaste vuote (remove_dir non cancella quelle con file).
        let _ = std::fs::remove_dir(c.dir.join("maps"));
        let _ = std::fs::remove_dir(&c.dir);
    }
    Ok((lib, replaced))
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    /// Al massimo questo numero di import (riusciti o no).
    pub max: Option<usize>,
    pub transfer: Transfer,
    /// Solo elenco: non si importa niente.
    pub dry_run: bool,
}

/// Esito di [`import_all`].
#[derive(Debug, Default)]
pub struct Summary {
    /// (id, cartella nella libreria, avvisi).
    pub imported: Vec<(String, PathBuf, Vec<String>)>,
    /// Già nella libreria (stessa provenienza "Poly Haven <id>").
    pub already_present: Vec<String>,
    /// Da importare (solo con `dry_run`), oppure lasciati per il limite `max`.
    pub pending: Vec<String>,
    pub failed: Vec<(String, String)>,
    pub incomplete: Vec<PathBuf>,
    pub invalid: Vec<(PathBuf, String)>,
    /// Copie difettose sostituite.
    pub replaced: usize,
    /// Cartelle `<id>` dei materiali importati (per l'elenco dei file rimasti).
    pub imported_dirs: Vec<PathBuf>,
}

/// Importa tutti i materiali completi della cartella dei download. `catalog`: voci del
/// catalogo Poly Haven per id (vuoto se non disponibile). `progress` riceve una riga per materiale.
pub fn import_all(
    root: &Path,
    catalog: &HashMap<String, OnlineAsset>,
    opts: Options,
    progress: &mut dyn FnMut(&str),
) -> anyhow::Result<Summary> {
    let scan = scan(root)?;
    let mut local: HashSet<String> = crate::online::local_source_keys();
    let mut s = Summary {
        incomplete: scan.incomplete,
        invalid: scan.invalid,
        ..Default::default()
    };
    let mut attempts = 0usize;
    for c in &scan.complete {
        let id = c.manifest.id.clone();
        if local.contains(&crate::online::source_key(crate::online::Provider::PolyHaven, &id)) {
            s.already_present.push(id);
            continue;
        }
        if opts.dry_run || opts.max.is_some_and(|m| attempts >= m) {
            s.pending.push(id);
            continue;
        }
        attempts += 1;
        match import(c, catalog.get(&id), opts.transfer) {
            Ok((lib, replaced)) => {
                progress(&format!("{id}: importato in {}", lib.dir.display()));
                for n in &lib.notes {
                    progress(&format!("    nota: {n}"));
                }
                local.insert(crate::online::source_key(crate::online::Provider::PolyHaven, &id));
                s.replaced += replaced;
                s.imported.push((id, lib.dir, lib.notes));
                s.imported_dirs.push(c.dir.clone());
            }
            Err(e) => {
                progress(&format!("{id}: ERRORE {e:#}"));
                s.failed.push((id, format!("{e:#}")));
            }
        }
    }
    Ok(s)
}

/// File rimasti nelle cartelle indicate, raggruppati per tipo ("nor_dx .exr", ".blend"):
/// (tipo, numero di file, byte), in ordine di spazio occupato.
pub fn leftovers(dirs: &[PathBuf]) -> Vec<(String, usize, u64)> {
    let mut by: BTreeMap<String, (usize, u64)> = BTreeMap::new();
    for dir in dirs {
        let id = dir.file_name().and_then(|n| n.to_str()).unwrap_or("").to_ascii_lowercase();
        for f in crate::paths::list_files_all(dir, 1) {
            let bytes = std::fs::metadata(&f).map(|m| m.len()).unwrap_or(0);
            let e = by.entry(leftover_kind(&id, &f)).or_default();
            e.0 += 1;
            e.1 += bytes;
        }
    }
    let mut v: Vec<(String, usize, u64)> = by.into_iter().map(|(k, (n, b))| (k, n, b)).collect();
    v.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
    v
}

/// Tipo di un file rimasto: nome senza l'id del materiale e senza la risoluzione
/// (`aerial_rocks_02_nor_dx_2k.exr` → "nor_dx .exr", `aerial_rocks_02_2k.blend` → ".blend").
fn leftover_kind(id: &str, f: &Path) -> String {
    let ext = f.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let stem = f.file_stem().and_then(|s| s.to_str()).unwrap_or("").to_ascii_lowercase();
    let rest = stem.strip_prefix(id).map(|r| r.trim_start_matches('_')).unwrap_or(&stem);
    let kind: Vec<&str> = rest
        .split('_')
        .filter(|t| !t.is_empty() && !(t.ends_with('k') && t.len() > 1 && t[..t.len() - 1].chars().all(|c| c.is_ascii_digit())))
        .collect();
    if kind.is_empty() {
        format!(".{ext}")
    } else {
        format!("{} .{ext}", kind.join("_"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leftover_kinds() {
        let k = |f: &str| leftover_kind("aerial_rocks_02", Path::new(f));
        assert_eq!(k("aerial_rocks_02_nor_dx_2k.exr"), "nor_dx .exr");
        assert_eq!(k("aerial_rocks_02_rough_ao_2k.exr"), "rough_ao .exr");
        assert_eq!(k("aerial_rocks_02_2k.blend"), ".blend");
        assert_eq!(k("altro.txt"), "altro .txt");
    }

    #[test]
    fn manifest_resolution_and_names() {
        let m: Manifest = serde_json::from_str(
            r#"{"id":"metal_plate_02","name":"Metal Plate 02","requested_resolution":"2k","maps":[
                {"map":"blend","resolution":"4k","file":"maps\\metal_plate_02_4k.blend"},
                {"map":"Diffuse","resolution":"2k","file":"maps\\metal_plate_02_diff_2k.jpg"},
                {"map":"Rough","resolution":"2k","file":"maps\\metal_plate_02_rough_2k.exr"}]}"#,
        )
        .unwrap();
        assert_eq!(m.resolution(), "2K");
        assert_eq!(m.source(), "Poly Haven metal_plate_02 (2K)");
        assert_eq!(m.library_name(), "Metal Plate 02 2K");
    }
}
