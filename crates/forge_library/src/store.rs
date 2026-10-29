//! Materiali salvati nella libreria.
//!
//! Ogni materiale è una cartella dentro `Libreria/Materiali`:
//! ```text
//! Legno noce 049/
//!   materiale.json        parametri ForgeRender (texture con percorsi relativi)
//!   anteprima-v3.png      miniatura renderizzata (sfera grande quanto una tessera)
//!   Wood049_Color.jpg …   mappe PBR
//! ```
//! La cartella si può copiare su un altro PC o condividere: è autosufficiente.

use crate::detect::{self, MapSet};
use anyhow::{bail, Context};
use forge_scene::{Material, MaterialId, Projection, TextureMapping};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const MATERIAL_FILE: &str = "materiale.json";
/// Anteprima renderizzata (v3: displacement spento di default; le vecchie si rifanno).
pub const THUMBNAIL_FILE: &str = "anteprima-v3.png";
/// Anteprime delle versioni precedenti: si eliminano quando si salva quella nuova.
const LEGACY_THUMBNAIL_FILES: [&str; 2] = ["anteprima.png", "anteprima-v2.png"];
/// Versione di `materiale.json`. Nella 1 il displacement veniva acceso in automatico
/// all'import (3 % del lato della texture): al caricamento si riporta a 0.
const FILE_VERSION: u32 = 2;

#[derive(Serialize, Deserialize)]
struct MaterialFile {
    version: u32,
    /// Provenienza (es. "ambientCG Wood049", "importato da cartella").
    #[serde(default)]
    source: String,
    material: Material,
}

/// Materiale della libreria, con i percorsi delle texture già assoluti.
#[derive(Clone, Debug)]
pub struct LibraryMaterial {
    pub dir: PathBuf,
    pub material: Material,
    pub source: String,
    pub thumbnail: Option<PathBuf>,
    /// Mappe nello slot sbagliato secondo il riconoscimento attuale (vedi [`misassigned_maps`]):
    /// il materiale va riscaricato.
    pub misassigned: Vec<String>,
}

/// Nome di cartella valido su tutti i sistemi.
pub fn folder_name(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || " -_().".contains(c) { c } else { '_' })
        .collect();
    let s = s.trim().trim_matches('.').to_string();
    if s.is_empty() {
        "Materiale".into()
    } else {
        s.chars().take(80).collect()
    }
}

fn unique_dir(base: &Path, name: &str) -> PathBuf {
    let mut p = base.join(folder_name(name));
    let mut k = 2;
    while p.exists() {
        p = base.join(format!("{} {k}", folder_name(name)));
        k += 1;
    }
    p
}

/// Materiale ForgeRender costruito dalle mappe trovate. `size_m`: lato reale della
/// texture in metri (proiezione box), se noto.
pub fn material_from_maps(name: &str, id: &str, set: &MapSet, size_m: Option<f32>) -> Material {
    let has = |p: &Option<PathBuf>| p.is_some();
    Material {
        id: MaterialId::new(id),
        name: name.to_string(),
        base_color: if has(&set.base_color) { [1.0; 3] } else { [0.8; 3] },
        metallic: if has(&set.metallic) { 1.0 } else { 0.0 },
        roughness: if has(&set.roughness) { 1.0 } else { 0.5 },
        base_color_texture: set.base_color.clone(),
        roughness_texture: set.roughness.clone(),
        roughness_is_gloss: set.roughness_is_gloss,
        normal_texture: set.normal.clone(),
        normal_directx: set.normal_directx,
        metallic_texture: set.metallic.clone(),
        height_texture: set.height.clone(),
        // Con la normal map il rilievo fine è già incluso. Il displacement resta spento: molte
        // height map (metalli, vernici, plastiche) contengono solo micro-variazioni normalizzate
        // su tutta la scala e con un valore automatico diventano bozzi. Si accende dal pannello.
        bump_mm: if has(&set.normal) { 0.0 } else { 1.0 },
        displacement_mm: 0.0,
        ao_texture: set.ao.clone(),
        opacity_texture: set.opacity.clone(),
        mapping: TextureMapping {
            projection: Projection::Box,
            scale: size_m.filter(|s| s.is_finite() && *s > 0.0).unwrap_or(1.0),
            ..Default::default()
        },
        category: "Libreria".into(),
        ..Default::default()
    }
}

/// Escursione di displacement proposta quando lo si accende dal pannello: 3% del lato della
/// texture (3–60 mm), adatta a materiali con rilievo vero (pietre, mattoni, cortecce).
pub fn default_displacement_mm(size_m: Option<f32>) -> f32 {
    (size_m.filter(|s| s.is_finite() && *s > 0.0).unwrap_or(1.0) * 30.0).clamp(3.0, 60.0)
}

/// Elenco dei materiali della libreria, in ordine alfabetico.
pub fn list() -> Vec<LibraryMaterial> {
    let Ok(rd) = std::fs::read_dir(crate::paths::materials_dir()) else {
        return Vec::new();
    };
    let mut v: Vec<LibraryMaterial> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join(MATERIAL_FILE).is_file())
        .filter_map(|d| match load(&d) {
            Ok(m) => Some(m),
            Err(e) => {
                tracing::warn!("libreria: {} ignorato ({e:#})", d.display());
                None
            }
        })
        .collect();
    v.sort_by_key(|m| m.material.name.to_lowercase());
    v
}

/// Carica un materiale dalla sua cartella.
pub fn load(dir: &Path) -> anyhow::Result<LibraryMaterial> {
    let text = std::fs::read_to_string(dir.join(MATERIAL_FILE))?;
    let f: MaterialFile = serde_json::from_str(&text).context("materiale.json non valido")?;
    let mut material = f.material;
    // Versione 1: il displacement era quello automatico dell'import, non una scelta.
    if f.version < 2 {
        material.displacement_mm = 0.0;
    }
    for slot in material.texture_slots_mut().into_iter().flatten() {
        if slot.is_relative() {
            *slot = dir.join(&*slot);
        }
    }
    material.id = MaterialId::new(format!("lib:{}", dir.file_name().and_then(|n| n.to_str()).unwrap_or("materiale")));
    let thumb = dir.join(THUMBNAIL_FILE);
    Ok(LibraryMaterial {
        dir: dir.to_path_buf(),
        misassigned: misassigned_maps(&material),
        material,
        source: f.source,
        thumbnail: thumb.is_file().then_some(thumb),
    })
}

/// Salva `material` nella cartella (le texture già dentro la cartella diventano relative).
pub fn save(dir: &Path, material: &Material, source: &str) -> anyhow::Result<()> {
    let mut m = material.clone();
    for slot in m.texture_slots_mut().into_iter().flatten() {
        if let Ok(rel) = slot.strip_prefix(dir) {
            *slot = rel.to_path_buf();
        }
    }
    let f = MaterialFile {
        version: FILE_VERSION,
        source: source.to_string(),
        material: m,
    };
    std::fs::write(dir.join(MATERIAL_FILE), serde_json::to_string_pretty(&f)?)?;
    Ok(())
}

/// Mappe il cui nome indica un tipo diverso dallo slot in cui sono. Succedeva ai materiali
/// importati prima del riconoscimento per suffisso (29/10/2026): con nomi come
/// `metal_plate_02_ao_4k.png` l'AO finiva come metallicità e il colore veniva scartato.
/// Le mappe con nomi non riconoscibili (scelte a mano) non contano.
pub fn misassigned_maps(m: &Material) -> Vec<String> {
    use detect::MapKind as K;
    type Slot<'a> = (&'a Option<PathBuf>, &'static str, fn(K) -> bool);
    let slots: [Slot; 7] = [
        (&m.base_color_texture, "colore", |k| k == K::BaseColor),
        (&m.roughness_texture, "rugosità", |k| matches!(k, K::Roughness | K::Gloss)),
        (&m.normal_texture, "normal", |k| matches!(k, K::Normal(_))),
        (&m.metallic_texture, "metallicità", |k| k == K::Metallic),
        (&m.height_texture, "rilievo", |k| k == K::Height),
        (&m.ao_texture, "occlusione", |k| k == K::Ao),
        (&m.opacity_texture, "opacità", |k| k == K::Opacity),
    ];
    slots
        .into_iter()
        .filter_map(|(p, slot, ok)| {
            let p = p.as_ref()?;
            let kind = detect::classify(p)?;
            (!ok(kind)).then(|| format!("{slot}: {}", p.file_name().map(|n| n.to_string_lossy()).unwrap_or_default()))
        })
        .collect()
}

/// Cartella della libreria del materiale con id `lib:<cartella>`, se esiste.
pub fn dir_of(id: &MaterialId) -> Option<PathBuf> {
    let name = id.0.strip_prefix("lib:")?;
    let dir = crate::paths::materials_dir().join(name);
    (!name.is_empty() && dir.join(MATERIAL_FILE).is_file()).then_some(dir)
}

/// Copia nella cartella le texture che stanno fuori (la cartella resta autosufficiente).
/// Le texture già dentro restano dove sono; file con lo stesso nome non si sovrascrivono.
fn copy_textures_into(dir: &Path, m: &mut Material) -> anyhow::Result<()> {
    let mut copied: Vec<(PathBuf, PathBuf)> = Vec::new();
    for slot in m.texture_slots_mut().into_iter() {
        let Some(src) = slot.clone() else { continue };
        if src.starts_with(dir) {
            continue;
        }
        if let Some((_, to)) = copied.iter().find(|(from, _)| *from == src) {
            *slot = Some(to.clone());
            continue;
        }
        let file = src.file_name().context("nome di file della texture non valido")?;
        let mut to = dir.join(file);
        let mut k = 2;
        while to.exists() {
            let stem = Path::new(file).file_stem().and_then(|s| s.to_str()).unwrap_or("texture");
            let ext = Path::new(file).extension().and_then(|s| s.to_str()).unwrap_or("png");
            to = dir.join(format!("{stem} {k}.{ext}"));
            k += 1;
        }
        std::fs::copy(&src, &to).with_context(|| format!("copia della texture {}", src.display()))?;
        copied.push((src, to.clone()));
        *slot = Some(to);
    }
    Ok(())
}

/// Sovrascrive il materiale della libreria nella sua cartella con le impostazioni di
/// `material` (es. modificate nel progetto). La miniatura si rifà.
pub fn update(dir: &Path, material: &Material) -> anyhow::Result<LibraryMaterial> {
    let root = crate::paths::materials_dir();
    anyhow::ensure!(dir.starts_with(&root), "cartella fuori dalla libreria");
    let source = load(dir).map(|m| m.source).unwrap_or_default();
    let mut m = material.clone();
    copy_textures_into(dir, &mut m)?;
    save(dir, &m, &source)?;
    let _ = std::fs::remove_file(dir.join(THUMBNAIL_FILE));
    load(dir)
}

/// Salva `material` nella libreria come nuovo materiale `name` (cartella nuova con la
/// copia di tutte le texture). L'originale non cambia.
pub fn save_as(material: &Material, name: &str, source: &str) -> anyhow::Result<LibraryMaterial> {
    let name = name.trim();
    anyhow::ensure!(!name.is_empty(), "nome vuoto");
    std::fs::create_dir_all(crate::paths::materials_dir())?;
    let dest = unique_dir(&crate::paths::materials_dir(), name);
    std::fs::create_dir_all(&dest)?;
    let result = (|| {
        let mut m = material.clone();
        m.name = name.to_string();
        m.category = "Libreria".into();
        copy_textures_into(&dest, &mut m)?;
        save(&dest, &m, source)?;
        load(&dest)
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&dest);
    }
    result
}

/// Salva la miniatura (RGBA8) del materiale.
pub fn save_thumbnail(dir: &Path, width: u32, height: u32, rgba: &[u8]) -> anyhow::Result<PathBuf> {
    for old in LEGACY_THUMBNAIL_FILES {
        let _ = std::fs::remove_file(dir.join(old));
    }
    let p = dir.join(THUMBNAIL_FILE);
    image::save_buffer(&p, rgba, width, height, image::ColorType::Rgba8)?;
    Ok(p)
}

/// Importa le mappe di una cartella come nuovo materiale della libreria (i file vengono copiati).
pub fn import_folder(src: &Path, name: Option<&str>, size_m: Option<f32>, source: &str) -> anyhow::Result<LibraryMaterial> {
    let files = detect::image_files(src);
    let set = detect::detect(&files);
    if set.is_empty() {
        bail!(
            "nessuna mappa riconosciuta in {} (servono file come *_Color, *_Roughness, *_Normal…)",
            src.display()
        );
    }
    // Nome: quello indicato, altrimenti dai file delle mappe, altrimenti dalla cartella.
    let name = name.map(str::to_string).unwrap_or_else(|| {
        [&set.base_color, &set.normal, &set.roughness, &set.packed]
            .iter()
            .find_map(|p| p.as_deref().and_then(detect::material_name_from_file))
            .unwrap_or_else(|| clean_name(src.file_name().and_then(|n| n.to_str()).unwrap_or("Materiale")))
    });
    std::fs::create_dir_all(crate::paths::materials_dir())?;
    let dest = unique_dir(&crate::paths::materials_dir(), &name);
    std::fs::create_dir_all(&dest)?;
    let result = (|| -> anyhow::Result<LibraryMaterial> {
        let copy = |p: &Option<PathBuf>| -> anyhow::Result<Option<PathBuf>> {
            let Some(p) = p else { return Ok(None) };
            let to = dest.join(p.file_name().context("nome di file non valido")?);
            std::fs::copy(p, &to).with_context(|| format!("copia di {}", p.display()))?;
            Ok(Some(to))
        };
        let mut local = MapSet {
            base_color: copy(&set.base_color)?,
            roughness: copy(&set.roughness)?,
            roughness_is_gloss: set.roughness_is_gloss,
            normal: copy(&set.normal)?,
            normal_directx: set.normal_directx,
            metallic: copy(&set.metallic)?,
            height: copy(&set.height)?,
            ao: copy(&set.ao)?,
            opacity: copy(&set.opacity)?,
            packed: None,
        };
        // Mappa ARM/ORM: si separa nei canali che mancano come file singoli.
        if let Some(packed) = &set.packed {
            if local.roughness.is_none() || local.metallic.is_none() || local.ao.is_none() {
                let stem = folder_name(&name).replace(' ', "_");
                let (ao, rough, metal) = detect::split_packed(packed, &dest, &stem)?;
                local.ao = local.ao.or(Some(ao));
                if local.roughness.is_none() {
                    local.roughness = Some(rough);
                    local.roughness_is_gloss = false;
                }
                // La metallicità impacchettata si usa solo se non è tutta nera.
                if local.metallic.is_none() && !is_black(&metal) {
                    local.metallic = Some(metal);
                }
            }
        }
        let id = format!("lib:{}", dest.file_name().and_then(|n| n.to_str()).unwrap_or("materiale"));
        let m = material_from_maps(&name, &id, &local, size_m);
        save(&dest, &m, source)?;
        load(&dest)
    })();
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&dest);
    }
    result
}

fn is_black(p: &Path) -> bool {
    image::open(p).map(|i| i.to_luma8().pixels().all(|px| px.0[0] < 3)).unwrap_or(true)
}

/// Nome leggibile da un nome di cartella/zip: "Wood049_2K-JPG" → "Wood049".
pub fn clean_name(raw: &str) -> String {
    let w: Vec<&str> = raw
        .split(['_', '-'])
        .filter(|t| {
            let l = t.to_ascii_lowercase();
            !(["jpg", "png", "exr", "jpeg"].contains(&l.as_str())
                || (l.ends_with('k') && l[..l.len() - 1].chars().all(|c| c.is_ascii_digit())))
        })
        .collect();
    let s = w.join(" ");
    if s.trim().is_empty() {
        raw.to_string()
    } else {
        s.trim().to_string()
    }
}

/// Importa un archivio zip (es. scaricato da ambientCG): estrae le immagini e le importa.
pub fn import_zip(zip_path: &Path, name: Option<&str>, size_m: Option<f32>, source: &str) -> anyhow::Result<LibraryMaterial> {
    let tmp = std::env::temp_dir().join(format!(
        "forgerender_zip_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&tmp)?;
    let result = (|| {
        let f = std::fs::File::open(zip_path).with_context(|| format!("apertura di {}", zip_path.display()))?;
        let mut z = zip::ZipArchive::new(f).context("archivio zip non valido")?;
        for i in 0..z.len() {
            let mut entry = z.by_index(i)?;
            if entry.is_dir() {
                continue;
            }
            // Solo il nome del file (niente percorsi: evita di scrivere fuori dalla cartella).
            let Some(fname) = entry
                .enclosed_name()
                .and_then(|p| p.file_name().map(|n| n.to_owned()))
                .map(PathBuf::from)
            else {
                continue;
            };
            if detect::classify(&fname).is_none() {
                continue;
            }
            let mut out = std::fs::File::create(tmp.join(&fname))?;
            std::io::copy(&mut entry, &mut out)?;
        }
        let default_name = clean_name(zip_path.file_stem().and_then(|n| n.to_str()).unwrap_or("Materiale"));
        import_folder(&tmp, Some(name.unwrap_or(&default_name)), size_m, source)
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    result
}

/// Elimina un materiale dalla libreria (la cartella intera).
pub fn delete(m: &LibraryMaterial) -> anyhow::Result<()> {
    let root = crate::paths::materials_dir();
    anyhow::ensure!(m.dir.starts_with(&root), "cartella fuori dalla libreria");
    std::fs::remove_dir_all(&m.dir)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn lib_root(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("forge_lib_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn gray(path: &Path, v: u8) {
        image::GrayImage::from_pixel(4, 4, image::Luma([v])).save(path).unwrap();
    }

    #[test]
    fn clean_names() {
        assert_eq!(clean_name("Wood049_2K-JPG"), "Wood049");
        assert_eq!(clean_name("rock_wall_02"), "rock wall 02");
        assert_eq!(folder_name("a/b:c"), "a_b_c");
    }

    // I test che usano FORGE_LIBRARY_DIR sono in un'unica funzione: la variabile è globale al processo.
    #[test]
    fn import_folder_zip_and_packed_maps() {
        let root = lib_root("root");
        // SAFETY: unico test del crate che modifica l'ambiente.
        unsafe { std::env::set_var("FORGE_LIBRARY_DIR", &root) };
        crate::paths::ensure().unwrap();
        assert!(root.join("HDRI/LEGGIMI.txt").is_file());

        // Cartella stile Poly Haven: colore, normal GL, mappa ARM impacchettata.
        let src = lib_root("src");
        image::RgbImage::from_pixel(4, 4, image::Rgb([200, 100, 50]))
            .save(src.join("brick_diff_1k.png"))
            .unwrap();
        image::RgbImage::from_pixel(4, 4, image::Rgb([128, 128, 255]))
            .save(src.join("brick_nor_gl_1k.png"))
            .unwrap();
        image::RgbImage::from_pixel(4, 4, image::Rgb([255, 180, 0]))
            .save(src.join("brick_arm_1k.png"))
            .unwrap();
        gray(&src.join("brick_disp_1k.png"), 90);
        let m = import_folder(&src, Some("Mattoni"), Some(2.0), "test").unwrap();
        assert_eq!(m.material.name, "Mattoni");
        assert!(m.dir.starts_with(root.join("Materiali")));
        let mat = &m.material;
        assert!(mat.base_color_texture.as_ref().unwrap().is_file());
        assert!(mat.normal_texture.is_some() && !mat.normal_directx);
        // ARM separata: rugosità dal verde (180), AO dal rosso, metallicità nera ignorata.
        let rough = image::open(mat.roughness_texture.as_ref().unwrap()).unwrap().to_luma8();
        assert_eq!(rough.get_pixel(0, 0).0[0], 180);
        assert!(mat.ao_texture.is_some());
        assert!(mat.metallic_texture.is_none());
        assert_eq!(mat.bump_mm, 0.0, "con la normal map il rilievo resta spento");
        assert!(mat.height_texture.is_some());
        assert_eq!(mat.displacement_mm, 0.0, "displacement spento di default");
        assert_eq!(mat.mapping.scale, 2.0);
        // Il JSON salva percorsi relativi: la cartella è spostabile.
        let json = std::fs::read_to_string(m.dir.join(MATERIAL_FILE)).unwrap();
        assert!(!json.contains(root.to_str().unwrap()), "{json}");

        // Zip stile ambientCG, con una cartella interna e file non immagine.
        let zpath = src.join("Metal012_1K-JPG.zip");
        {
            let mut z = zip::ZipWriter::new(std::fs::File::create(&zpath).unwrap());
            let opt = zip::write::SimpleFileOptions::default();
            for (n, v) in [
                ("Metal012_1K-JPG_Color.png", 150u8),
                ("Metal012_1K-JPG_Metalness.png", 255),
                ("Metal012_1K-JPG_Roughness.png", 60),
            ] {
                let p = src.join(n);
                gray(&p, v);
                z.start_file(format!("sub/{n}"), opt).unwrap();
                z.write_all(&std::fs::read(&p).unwrap()).unwrap();
            }
            z.start_file("Metal012.usdc", opt).unwrap();
            z.write_all(b"x").unwrap();
            z.finish().unwrap();
        }
        let z = import_zip(&zpath, None, None, "ambientCG Metal012").unwrap();
        assert_eq!(z.material.name, "Metal012");
        assert_eq!(z.material.metallic, 1.0);
        assert!(z.material.metallic_texture.is_some());
        assert_eq!(z.source, "ambientCG Metal012");

        // Elenco, ricaricamento e cancellazione.
        let all = list();
        assert_eq!(
            all.iter().map(|m| m.material.name.as_str()).collect::<Vec<_>>(),
            ["Mattoni", "Metal012"]
        );
        assert!(all[0].material.id.0.starts_with("lib:"));

        // File della versione 1 (displacement automatico dell'import): torna a 0.
        let json_path = all[0].dir.join(MATERIAL_FILE);
        let mut v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&json_path).unwrap()).unwrap();
        v["version"] = 1.into();
        v["material"]["displacement_mm"] = 30.0.into();
        std::fs::write(&json_path, v.to_string()).unwrap();
        assert_eq!(load(&all[0].dir).unwrap().material.displacement_mm, 0.0);
        // Una scelta salvata dalla versione 2 in poi resta.
        let mut chosen = load(&all[0].dir).unwrap().material;
        chosen.displacement_mm = 12.0;
        assert_eq!(update(&all[0].dir, &chosen).unwrap().material.displacement_mm, 12.0);
        let all = list();

        // Salva: le modifiche restano nella cartella, provenienza e texture conservate.
        let mut edited = all[0].material.clone();
        edited.roughness = 0.25;
        edited.mapping.scale = 3.5;
        assert_eq!(dir_of(&edited.id).as_deref(), Some(all[0].dir.as_path()));
        let upd = update(&all[0].dir, &edited).unwrap();
        assert_eq!(upd.material, edited, "ricaricato uguale a quello salvato");
        assert_eq!(upd.source, "test");
        assert_eq!(load(&all[0].dir).unwrap().material.roughness, 0.25);
        // Una texture esterna viene copiata dentro la cartella.
        let ext_tex = src.join("esterna.png");
        gray(&ext_tex, 77);
        edited.ao_texture = Some(ext_tex.clone());
        let upd = update(&all[0].dir, &edited).unwrap();
        let ao = upd.material.ao_texture.clone().unwrap();
        assert!(ao.starts_with(&all[0].dir) && ao.is_file(), "{}", ao.display());

        // Salva con nome: cartella nuova con le sue texture, originale invariato.
        let copy = save_as(&upd.material, "Mattoni chiari", "copia di Mattoni").unwrap();
        assert_eq!(copy.material.name, "Mattoni chiari");
        assert_ne!(copy.dir, all[0].dir);
        assert_eq!(copy.material.roughness, 0.25);
        for t in copy.material.texture_slots().into_iter().flatten() {
            assert!(t.starts_with(&copy.dir) && t.is_file(), "{}", t.display());
        }
        assert_eq!(load(&all[0].dir).unwrap().material.name, "Mattoni");
        assert!(save_as(&upd.material, "  ", "").is_err());
        delete(&copy).unwrap();

        // Materiale importato col vecchio riconoscimento: una mappa nello slot sbagliato.
        let mut broken = load(&all[0].dir).unwrap().material;
        assert!(misassigned_maps(&broken).is_empty());
        broken.metallic_texture = broken.normal_texture.clone();
        let bad = misassigned_maps(&broken);
        assert_eq!(bad.len(), 1, "{bad:?}");
        assert!(bad[0].starts_with("metallicità"), "{bad:?}");
        // Una texture con un nome qualsiasi scelta a mano non è un errore.
        broken.metallic_texture = Some(PathBuf::from("la_mia_texture.png"));
        assert!(misassigned_maps(&broken).is_empty());
        assert_eq!(dir_of(&MaterialId::new("lib:non-esiste")), None);
        assert_eq!(dir_of(&MaterialId::new("acciaio")), None);
        delete(&all[1]).unwrap();
        assert_eq!(list().len(), 1);

        // Cartella senza mappe: errore chiaro e nessuna cartella creata.
        let empty = lib_root("empty");
        assert!(import_folder(&empty, None, None, "").is_err());
        assert_eq!(list().len(), 1);
        std::fs::remove_dir_all(root).ok();
        std::fs::remove_dir_all(src).ok();
    }
}
