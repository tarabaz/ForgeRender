//! Progetto con tutti i suoi file in un posto solo, da spostare o mandare a qualcuno:
//! - **cartella**: il `.frproject` con accanto `modelli/` (i file importati e quelli a cui
//!   rimandano: `.mtl` e texture degli OBJ, `.bin` e immagini dei glTF), `textures/` (le mappe dei
//!   materiali) e `hdri/` (l'ambiente); i percorsi nel progetto puntano alle copie;
//! - **ZIP** o **`.frpack`**: la stessa cartella compressa in un file solo. Il `.frpack` si apre
//!   direttamente con File → Apri (il programma lo estrae in una cartella sua e ci lavora).

use crate::{save, ProjectMeta, SourceRef, EXTENSION};
use anyhow::Context;
use forge_render_core::RenderSettings;
use forge_scene::Scene;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Estensione del progetto in un file solo.
pub const PACK_EXTENSION: &str = "frpack";

/// File copiati e file che mancavano (restano citati nel progetto ma non sono nel pacchetto).
#[derive(Debug, Default)]
pub struct PackReport {
    pub copied: usize,
    pub missing: Vec<PathBuf>,
}

/// Copia `src` in `dir` con il suo nome (o il nome più un numero, se c'è già un file diverso con
/// quel nome) e restituisce la copia. Lo stesso file copiato due volte è una sola copia.
fn copy_unique(src: &Path, dir: &Path, done: &mut HashMap<PathBuf, PathBuf>, report: &mut PackReport) -> Option<PathBuf> {
    let key = std::fs::canonicalize(src).unwrap_or_else(|_| src.to_path_buf());
    if let Some(p) = done.get(&key) {
        return Some(p.clone());
    }
    if !src.is_file() {
        report.missing.push(src.to_path_buf());
        return None;
    }
    let name = src.file_name()?.to_string_lossy().into_owned();
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) => (s.to_string(), format!(".{e}")),
        None => (name.clone(), String::new()),
    };
    let mut dst = dir.join(&name);
    let mut k = 2;
    while dst.exists() {
        dst = dir.join(format!("{stem}-{k}{ext}"));
        k += 1;
    }
    std::fs::create_dir_all(dir).ok()?;
    std::fs::copy(src, &dst).ok()?;
    report.copied += 1;
    done.insert(key, dst.clone());
    Some(dst)
}

/// File a cui rimanda un modello (percorsi relativi alla sua cartella): materiali e texture
/// degli OBJ, buffer e immagini dei glTF. I formati in un file solo (GLB, STL, STEP) non ne hanno.
fn companions(model: &Path) -> Vec<PathBuf> {
    let ext = model.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let Ok(text) = std::fs::read_to_string(model) else {
        return Vec::new();
    };
    match ext.as_str() {
        // STEP "a più file": i pezzi dell'assieme, accanto.
        "step" | "stp" => forge_import::step::external_refs(model),
        "obj" => {
            let mut out = Vec::new();
            for line in text.lines() {
                if let Some(rest) = line.trim().strip_prefix("mtllib") {
                    let mtl = PathBuf::from(rest.trim());
                    out.push(mtl.clone());
                    let base = model.parent().unwrap_or(Path::new("."));
                    if let Ok(m) = std::fs::read_to_string(base.join(&mtl)) {
                        let mtl_dir = mtl.parent().map(Path::to_path_buf).unwrap_or_default();
                        out.extend(mtl_maps(&m).into_iter().map(|t| mtl_dir.join(t)));
                    }
                }
            }
            out
        }
        "gltf" => {
            let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
                return Vec::new();
            };
            ["buffers", "images"]
                .iter()
                .filter_map(|k| v.get(k).and_then(|a| a.as_array()))
                .flatten()
                .filter_map(|b| b.get("uri").and_then(|u| u.as_str()))
                .filter(|u| !u.starts_with("data:"))
                .map(|u| PathBuf::from(u.replace("%20", " ")))
                .collect()
        }
        _ => Vec::new(),
    }
}

/// Texture citate in un file `.mtl` (l'ultima parola delle righe `map_…`, `bump`, `disp`, `norm`,
/// dopo le eventuali opzioni).
fn mtl_maps(mtl: &str) -> Vec<PathBuf> {
    mtl.lines()
        .filter_map(|l| {
            let l = l.trim();
            let key = l.split_whitespace().next()?.to_ascii_lowercase();
            let is_map = key.starts_with("map_") || matches!(key.as_str(), "bump" | "disp" | "norm" | "decal" | "refl");
            if !is_map {
                return None;
            }
            l.split_whitespace()
                .last()
                .filter(|w| !w.eq_ignore_ascii_case(&key))
                .map(PathBuf::from)
        })
        .collect()
}

/// Salva il progetto `name` nella cartella `dir` insieme a tutti i file che usa (vedi il modulo).
/// Restituisce il percorso del `.frproject`.
pub fn save_bundle(
    dir: &Path,
    name: &str,
    scene: &Scene,
    meta: &ProjectMeta,
    settings: &RenderSettings,
) -> anyhow::Result<(PathBuf, PackReport)> {
    std::fs::create_dir_all(dir).with_context(|| format!("{}", dir.display()))?;
    let mut report = PackReport::default();
    let mut done = HashMap::new();
    let mut scene = scene.clone();
    let mut sources = Vec::new();
    // Modelli: ognuno nella sua cartella, con i file a cui rimanda allo stesso percorso relativo.
    for (i, s) in meta.sources.iter().enumerate() {
        let stem = s
            .absolute_path
            .file_stem()
            .map(|x| x.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mdir = dir
            .join("modelli")
            .join(if i == 0 { stem.clone() } else { format!("{stem}-{}", i + 1) });
        let Some(copy) = copy_unique(&s.absolute_path, &mdir, &mut done, &mut report) else {
            continue;
        };
        // Conversione STEP già fatta: nel pacchetto, così si apre senza riconvertire.
        let mut converted = None;
        if let Some(conv) = meta
            .path
            .as_ref()
            .and_then(|p| p.parent())
            .zip(s.converted.as_ref())
            .map(|(d, c)| d.join(c))
            .filter(|c| c.is_file())
            .or_else(|| {
                forge_import::step::fingerprint(&s.absolute_path, &s.import_options)
                    .map(|k| forge_import::step::cached_conversion(&k))
                    .filter(|c| c.is_file())
            })
        {
            let rel = Path::new(crate::CONVERTED_DIR).join(if i == 0 {
                format!("{stem}.frmesh")
            } else {
                format!("{stem}-{}.frmesh", i + 1)
            });
            std::fs::create_dir_all(dir.join(crate::CONVERTED_DIR))?;
            std::fs::copy(&conv, dir.join(&rel)).with_context(|| format!("{}", conv.display()))?;
            report.copied += 1;
            converted = Some(rel);
        }
        let src_dir = s.absolute_path.parent().unwrap_or(Path::new("."));
        for rel in companions(&s.absolute_path) {
            let from = src_dir.join(&rel);
            let to = mdir.join(&rel);
            if from.is_file() {
                if let Some(parent) = to.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(&from, &to).with_context(|| format!("{}", from.display()))?;
                report.copied += 1;
            } else {
                report.missing.push(from);
            }
        }
        let converted_key = converted.as_ref().and(
            s.converted_key
                .clone()
                .or_else(|| forge_import::step::fingerprint(&s.absolute_path, &s.import_options)),
        );
        sources.push(SourceRef {
            absolute_path: copy,
            converted,
            converted_key,
            ..s.clone()
        });
    }
    // Texture dei materiali usati (assegnati o predefinito) e HDRI: i percorsi del progetto
    // puntano alle copie. I materiali della libreria non usati restano come sono.
    let tex_dir = dir.join("textures");
    let used: std::collections::HashSet<_> = scene
        .assignments
        .values()
        .chain(std::iter::once(&scene.default_material))
        .cloned()
        .collect();
    for m in scene.materials.materials.iter_mut().filter(|m| used.contains(&m.id)) {
        for slot in m.texture_slots_mut().into_iter().flatten() {
            if let Some(copy) = copy_unique(slot, &tex_dir, &mut done, &mut report) {
                *slot = copy;
            }
        }
    }
    for d in &mut scene.decals {
        if let Some(img) = &mut d.image {
            if let Some(copy) = copy_unique(img, &tex_dir, &mut done, &mut report) {
                *img = copy;
            }
        }
    }
    if let forge_scene::Environment::Hdri { path, .. } = &mut scene.environment {
        if let Some(copy) = copy_unique(path, &dir.join("hdri"), &mut done, &mut report) {
            *path = copy;
        }
    }
    let project = dir.join(format!("{name}.{EXTENSION}"));
    let meta = ProjectMeta {
        path: Some(project.clone()),
        sources,
        test_scene: meta.test_scene,
    };
    save(&project, &scene, &meta, settings)?;
    Ok((project, report))
}

/// Comprime la cartella `dir` nel file `out` (ZIP; le immagini già compresse si salvano come
/// sono, il resto compresso). Scrittura su un file temporaneo poi rinominato.
pub fn zip_dir(dir: &Path, out: &Path) -> anyhow::Result<()> {
    let tmp = out.with_extension("tmp");
    {
        let f = std::fs::File::create(&tmp).with_context(|| format!("{}", tmp.display()))?;
        let mut z = zip::ZipWriter::new(f);
        let deflate = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .large_file(true);
        let stored = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .large_file(true);
        let mut stack = vec![dir.to_path_buf()];
        let mut buf = Vec::new();
        while let Some(d) = stack.pop() {
            let mut entries: Vec<_> = std::fs::read_dir(&d)?.flatten().map(|e| e.path()).collect();
            entries.sort();
            for p in entries {
                if p.is_dir() {
                    stack.push(p);
                    continue;
                }
                let rel = p.strip_prefix(dir)?.to_string_lossy().replace('\\', "/");
                let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
                let packed = matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp" | "glb" | "zip" | "exr");
                z.start_file(rel, if packed { stored } else { deflate })?;
                buf.clear();
                std::fs::File::open(&p)?.read_to_end(&mut buf)?;
                z.write_all(&buf)?;
            }
        }
        z.finish()?;
    }
    std::fs::rename(&tmp, out).with_context(|| format!("{}", out.display()))?;
    Ok(())
}

/// Progetto con tutti i file in un solo file `out` (`.frpack` o `.zip`).
pub fn save_pack(out: &Path, scene: &Scene, meta: &ProjectMeta, settings: &RenderSettings) -> anyhow::Result<PackReport> {
    let name = out
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "progetto".into());
    let work = std::env::temp_dir().join(format!("forgerender-pack-{}-{}", std::process::id(), unique()));
    let _ = std::fs::remove_dir_all(&work);
    let r = save_bundle(&work, &name, scene, meta, settings).and_then(|(_, report)| {
        zip_dir(&work, out)?;
        Ok(report)
    });
    let _ = std::fs::remove_dir_all(&work);
    r
}

/// Estrae il pacchetto `pack` nella cartella `into` e restituisce il `.frproject` che contiene.
pub fn unpack(pack: &Path, into: &Path) -> anyhow::Result<PathBuf> {
    let f = std::fs::File::open(pack).with_context(|| format!("{}", pack.display()))?;
    let mut z = zip::ZipArchive::new(f).context("il pacchetto non è un archivio valido")?;
    std::fs::create_dir_all(into)?;
    let mut project = None;
    for i in 0..z.len() {
        let mut e = z.by_index(i)?;
        // Solo percorsi dentro la cartella di destinazione.
        let Some(rel) = e.enclosed_name() else { continue };
        let to = into.join(&rel);
        if e.is_dir() {
            std::fs::create_dir_all(&to)?;
            continue;
        }
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = std::fs::File::create(&to)?;
        std::io::copy(&mut e, &mut out)?;
        if rel.components().count() == 1 && rel.extension().is_some_and(|x| x == EXTENSION) {
            project = Some(to);
        }
    }
    project.ok_or_else(|| anyhow::anyhow!("nel pacchetto non c'è un progetto .{EXTENSION}"))
}

fn unique() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{load, source_ref};
    use forge_scene::{Material, MaterialId};

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("forge_pack_{}_{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// Un OBJ con il suo .mtl e una texture, una texture di materiale e una HDRI sparse in cartelle
    /// diverse: il pacchetto le contiene tutte e, aperto altrove, il progetto le ritrova.
    #[test]
    fn pack_contains_everything_and_opens_elsewhere() {
        let src = tmp("sorgenti");
        std::fs::create_dir_all(src.join("tex")).unwrap();
        std::fs::write(
            src.join("cubo.obj"),
            "mtllib cubo.mtl\nv 0 0 0\nv 1 0 0\nv 0 1 0\nusemtl rosso\nf 1 2 3\n",
        )
        .unwrap();
        std::fs::write(src.join("cubo.mtl"), "newmtl rosso\nKd 1 0 0\nmap_Kd -bm 1 tex/legno.png\n").unwrap();
        std::fs::write(src.join("tex/legno.png"), b"png finto").unwrap();
        let other = tmp("altrove");
        std::fs::write(other.join("marmo.jpg"), b"jpg finto").unwrap();
        std::fs::write(other.join("cielo.hdr"), b"hdr finto").unwrap();

        let opts = forge_import::ImportOptions::default();
        let mut scene = Scene::default();
        forge_import::import_file(&mut scene, &src.join("cubo.obj"), &opts).unwrap();
        scene.materials.upsert(Material {
            id: MaterialId::new("marmo"),
            base_color_texture: Some(other.join("marmo.jpg")),
            ..Default::default()
        });
        // Il marmo è assegnato al modello: la sua texture va nel pacchetto.
        let id = scene.nodes.iter().find(|n| n.mesh.is_some()).unwrap().id.clone();
        scene.assignments.insert(id, MaterialId::new("marmo"));
        scene.environment = forge_scene::Environment::Hdri {
            path: other.join("cielo.hdr"),
            strength: 1.0,
            rotation_deg: 0.0,
        };
        let meta = ProjectMeta {
            path: None,
            sources: vec![source_ref(None, &src.join("cubo.obj"), opts)],
            test_scene: false,
        };
        let out = tmp("uscita").join("mio.frpack");
        let report = save_pack(&out, &scene, &meta, &RenderSettings::default()).unwrap();
        assert!(report.missing.is_empty(), "{:?}", report.missing);
        assert_eq!(report.copied, 5);

        // I file originali spariscono: il pacchetto deve bastare da solo.
        std::fs::remove_dir_all(&src).unwrap();
        std::fs::remove_dir_all(&other).unwrap();
        let open = tmp("aperto");
        let project = unpack(&out, &open).unwrap();
        assert!(open.join("modelli/cubo/cubo.mtl").is_file());
        assert!(open.join("modelli/cubo/tex/legno.png").is_file());
        let l = load(&project).unwrap();
        assert!(l.report.missing_sources.is_empty(), "{:?}", l.report.missing_sources);
        assert!(l.scene.nodes.iter().any(|n| n.mesh.is_some()), "il modello è stato reimportato");
        let tex = l
            .scene
            .materials
            .get(&MaterialId::new("marmo"))
            .unwrap()
            .base_color_texture
            .clone()
            .unwrap();
        assert!(tex.starts_with(&open) && tex.is_file(), "{}", tex.display());
        match &l.scene.environment {
            forge_scene::Environment::Hdri { path, .. } => assert!(path.starts_with(&open) && path.is_file()),
            e => panic!("{e:?}"),
        }
        assert!(
            l.report.warnings.iter().all(|w| !w.contains("non trovato")),
            "{:?}",
            l.report.warnings
        );
    }

    #[test]
    fn mtl_maps_skip_options() {
        let m = "newmtl a\nmap_Kd -s 1 1 1 legno.png\nbump -bm 0.5 rilievo.png\nKd 1 1 1\n";
        assert_eq!(mtl_maps(m), vec![PathBuf::from("legno.png"), PathBuf::from("rilievo.png")]);
    }
}
