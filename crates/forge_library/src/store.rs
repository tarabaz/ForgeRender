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
/// Anteprima renderizzata (v4: con gli EXR a un canale di Poly Haven letti; nella v3 rugosità,
/// AO e height in EXR "Y" mancavano. Le vecchie si rifanno).
pub const THUMBNAIL_FILE: &str = "anteprima-v6.png";
/// Anteprime delle versioni precedenti: si eliminano quando si salva quella nuova.
const LEGACY_THUMBNAIL_FILES: [&str; 11] = [
    "anteprima.png",
    "anteprima-v2.png",
    "anteprima-v3.png",
    "anteprima-v3-256.png",
    "anteprima-v3-512.png",
    "anteprima-v4.png",
    "anteprima-v4-256.png",
    "anteprima-v4-512.png",
    "anteprima-v5.png",
    "anteprima-v5-256.png",
    "anteprima-v5-512.png",
];
/// Versione di `materiale.json`. Nella 1 il displacement veniva acceso in automatico
/// all'import (3 % del lato della texture): al caricamento si riporta a 0.
const FILE_VERSION: u32 = 2;

#[derive(Serialize, Deserialize)]
struct MaterialFile {
    version: u32,
    /// Provenienza (es. "ambientCG Wood049", "importato da cartella").
    #[serde(default)]
    source: String,
    /// Avvisi dell'import (es. mappe che la fonte non offre alla risoluzione scelta).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    notes: Vec<String>,
    /// Categorie della fonte (es. Poly Haven: "wood", "floor"), come le scrive il sito.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    categories: Vec<String>,
    /// Tag della fonte (parole chiave in inglese), usati dalla ricerca.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    tags: Vec<String>,
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
    /// Avvisi dell'import salvati con il materiale.
    pub notes: Vec<String>,
    /// Categorie e tag della fonte (vuoti per i materiali importati da cartella).
    pub categories: Vec<String>,
    pub tags: Vec<String>,
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

/// Displacement dei materiali importati (impostazione dell'app; la CLI usa il predefinito).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ImportDisplacement {
    /// Spento: la height resta collegata, si accende dal pannello (predefinito).
    #[default]
    Off,
    /// Valore proposto: 3 % del lato della texture.
    Proposed,
    /// Scala scritta nel file MaterialX della fonte, se c'è; altrimenti spento.
    FromMaterialX,
}

static IMPORT_DISPLACEMENT: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

pub fn set_import_displacement(p: ImportDisplacement) {
    IMPORT_DISPLACEMENT.store(p as u8, std::sync::atomic::Ordering::Relaxed);
}

pub fn import_displacement() -> ImportDisplacement {
    match IMPORT_DISPLACEMENT.load(std::sync::atomic::Ordering::Relaxed) {
        1 => ImportDisplacement::Proposed,
        2 => ImportDisplacement::FromMaterialX,
        _ => ImportDisplacement::Off,
    }
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
        displacement_mm: if has(&set.height) && import_displacement() == ImportDisplacement::Proposed {
            default_displacement_mm(size_m)
        } else {
            0.0
        },
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
        notes: f.notes,
        categories: f.categories,
        tags: f.tags,
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
    // Avvisi dell'import, categorie e tag restano anche quando si salva il materiale modificato.
    let (notes, categories, tags) = std::fs::read_to_string(dir.join(MATERIAL_FILE))
        .ok()
        .and_then(|t| serde_json::from_str::<MaterialFile>(&t).ok())
        .map(|f| (f.notes, f.categories, f.tags))
        .unwrap_or_default();
    let f = MaterialFile {
        version: FILE_VERSION,
        source: source.to_string(),
        notes,
        categories,
        tags,
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
    remove_thumbnails(dir);
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

/// Registra gli avvisi dell'import nel `materiale.json` e restituisce il materiale ricaricato.
pub fn set_notes(dir: &Path, notes: &[String]) -> anyhow::Result<LibraryMaterial> {
    let path = dir.join(MATERIAL_FILE);
    let mut f: MaterialFile = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    f.notes = notes.to_vec();
    std::fs::write(&path, serde_json::to_string_pretty(&f)?)?;
    load(dir)
}

/// Registra categorie e tag della fonte nel `materiale.json` e restituisce il materiale ricaricato.
pub fn set_catalog_info(dir: &Path, categories: &[String], tags: &[String]) -> anyhow::Result<LibraryMaterial> {
    let path = dir.join(MATERIAL_FILE);
    let mut f: MaterialFile = serde_json::from_str(&std::fs::read_to_string(&path)?)?;
    f.categories = categories.to_vec();
    f.tags = tags.to_vec();
    std::fs::write(&path, serde_json::to_string_pretty(&f)?)?;
    load(dir)
}

/// Lati (pixel) delle miniature: 128 è quella standard, le altre si renderizzano solo
/// quando la libreria mostra anteprime più grandi.
pub const THUMBNAIL_SIZES: [u32; 3] = [128, 256, 512];

/// Nome del file della miniatura di lato `size` (128 → [`THUMBNAIL_FILE`]).
pub fn thumbnail_file(size: u32) -> String {
    if size <= THUMBNAIL_SIZES[0] {
        THUMBNAIL_FILE.to_string()
    } else {
        format!("anteprima-v6-{size}.png")
    }
}

/// Elimina tutte le miniature del materiale (si rifanno alla prossima visualizzazione).
pub fn remove_thumbnails(dir: &Path) {
    for s in THUMBNAIL_SIZES {
        let _ = std::fs::remove_file(dir.join(thumbnail_file(s)));
    }
}

/// Cartella della cache delle miniature dei materiali predefiniti (nella libreria).
pub fn preset_thumbnail_dir() -> PathBuf {
    crate::paths::root().join(".cache").join("anteprime-predefiniti")
}

/// Svuota la cache delle anteprime: miniature di tutti i materiali della libreria e dei
/// predefiniti. Si rifanno alla prossima visualizzazione (con le impostazioni attuali).
/// Restituisce quanti file sono stati eliminati.
pub fn clear_thumbnail_cache() -> usize {
    let mut n = 0;
    let mut remove = |p: PathBuf| {
        if std::fs::remove_file(&p).is_ok() {
            n += 1;
        }
    };
    for m in list() {
        for s in THUMBNAIL_SIZES {
            remove(m.dir.join(thumbnail_file(s)));
        }
        for old in LEGACY_THUMBNAIL_FILES {
            remove(m.dir.join(old));
        }
    }
    if let Ok(entries) = std::fs::read_dir(preset_thumbnail_dir()) {
        for e in entries.flatten() {
            remove(e.path());
        }
    }
    n
}

/// Salva la miniatura (RGBA8) del materiale; il file dipende dal lato (`width`).
pub fn save_thumbnail(dir: &Path, width: u32, height: u32, rgba: &[u8]) -> anyhow::Result<PathBuf> {
    for old in LEGACY_THUMBNAIL_FILES {
        let _ = std::fs::remove_file(dir.join(old));
    }
    let p = dir.join(thumbnail_file(width));
    image::save_buffer(&p, rgba, width, height, image::ColorType::Rgba8)?;
    Ok(p)
}

/// Come arrivano i file nella cartella del materiale.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Transfer {
    /// Copiati: l'origine resta com'era.
    #[default]
    Copy,
    /// Spostati (stesso disco: niente doppio spazio). Se l'import fallisce tornano al loro posto.
    Move,
}

/// Importa le mappe di una cartella come nuovo materiale della libreria (i file vengono copiati).
pub fn import_folder(src: &Path, name: Option<&str>, size_m: Option<f32>, source: &str) -> anyhow::Result<LibraryMaterial> {
    import_folder_with(src, name, size_m, source, Transfer::Copy)
}

/// Come [`import_folder`], scegliendo se copiare o spostare i file. Arrivano nella cartella
/// del materiale solo le mappe collegate agli slot (più i canali estratti dalla ARM che
/// servono) e il file MaterialX; tutto il resto resta nell'origine.
pub fn import_folder_with(
    src: &Path,
    name: Option<&str>,
    size_m: Option<f32>,
    source: &str,
    transfer: Transfer,
) -> anyhow::Result<LibraryMaterial> {
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
    // File spostati (origine, destinazione): in caso di errore tornano indietro.
    let moved: std::cell::RefCell<Vec<(PathBuf, PathBuf)>> = Default::default();
    let put = |from: &Path, to: &Path| -> anyhow::Result<()> {
        match transfer {
            Transfer::Copy => {
                std::fs::copy(from, to).with_context(|| format!("copia di {}", from.display()))?;
            }
            Transfer::Move => {
                std::fs::rename(from, to).with_context(|| format!("spostamento di {}", from.display()))?;
                moved.borrow_mut().push((from.to_path_buf(), to.to_path_buf()));
            }
        }
        Ok(())
    };
    let result = (|| -> anyhow::Result<LibraryMaterial> {
        let copy = |p: &Option<PathBuf>| -> anyhow::Result<Option<PathBuf>> {
            let Some(p) = p else { return Ok(None) };
            let to = dest.join(p.file_name().context("nome di file non valido")?);
            put(p, &to)?;
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
        // Mappa ARM/ORM: si estraggono solo i canali che mancano come file singoli
        // (la metallicità tutta nera non si usa).
        if let Some(packed) = &set.packed {
            let need = [local.ao.is_none(), local.roughness.is_none(), local.metallic.is_none()];
            if need.contains(&true) {
                let stem = folder_name(&name).replace(' ', "_");
                let [ao, rough, metal] = detect::split_packed(packed, &dest, &stem, need)?;
                local.ao = local.ao.take().or(ao);
                if let Some(r) = rough {
                    local.roughness = Some(r);
                    local.roughness_is_gloss = false;
                }
                local.metallic = local.metallic.take().or(metal);
            }
        }
        let id = format!("lib:{}", dest.file_name().and_then(|n| n.to_str()).unwrap_or("materiale"));
        let m = material_from_maps(&name, &id, &local, size_m);
        save(&dest, &m, source)?;
        // File MaterialX della fonte: si conserva per leggerne i valori (vedi `mtlx`).
        if let Ok(rd) = std::fs::read_dir(src) {
            for p in rd.flatten().map(|e| e.path()) {
                if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("mtlx")) {
                    if let Some(n) = p.file_name() {
                        let _ = put(&p, &dest.join(n));
                    }
                }
            }
        }
        if import_displacement() == ImportDisplacement::FromMaterialX && m.height_texture.is_some() {
            let scale = crate::mtlx::find_in(&dest)
                .and_then(|p| crate::mtlx::read(&p).ok())
                .and_then(|v| crate::mtlx::displacement_scale(&v));
            if let Some(s) = scale {
                let mut m = m.clone();
                m.displacement_mm = (s * 1000.0).clamp(0.0, 1000.0);
                save(&dest, &m, source)?;
            }
        }
        load(&dest)
    })();
    if result.is_err() {
        for (from, to) in moved.borrow().iter().rev() {
            if let Err(e) = std::fs::rename(to, from) {
                // Il file resta nella cartella del materiale, che quindi non si cancella.
                tracing::error!("libreria: {} non riportato in {} ({e})", to.display(), from.display());
                return result;
            }
        }
        let _ = std::fs::remove_dir_all(&dest);
    }
    result
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
            let is_mtlx = fname.extension().is_some_and(|e| e.eq_ignore_ascii_case("mtlx"));
            if detect::classify(&fname).is_none() && !is_mtlx {
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
        // Si scrivono solo i canali usati: niente PNG della metallicità nera.
        assert!(!m.dir.join("Mattoni_Metalness.png").exists());
        assert!(!m.dir.join("brick_arm_1k.png").exists(), "la ARM separata non va in libreria");
        assert_eq!(mat.bump_mm, 0.0, "con la normal map il rilievo resta spento");
        assert!(mat.height_texture.is_some());
        assert_eq!(mat.displacement_mm, 0.0, "displacement spento di default");
        assert_eq!(mat.mapping.scale, 2.0);
        // Il JSON salva percorsi relativi: la cartella è spostabile.
        let json = std::fs::read_to_string(m.dir.join(MATERIAL_FILE)).unwrap();
        assert!(!json.contains(root.to_str().unwrap()), "{json}");

        // Spostamento: arrivano solo le mappe usate e il MaterialX, il resto resta nell'origine.
        let msrc = lib_root("move");
        gray(&msrc.join("tile_diff_1k.png"), 120);
        gray(&msrc.join("tile_rough_1k.png"), 60);
        gray(&msrc.join("tile_nor_dx_1k.png"), 128);
        gray(&msrc.join("tile_nor_gl_1k.png"), 128);
        std::fs::write(msrc.join("tile_1k.mtlx"), "<materialx/>").unwrap();
        std::fs::write(msrc.join("tile_1k.blend"), "x").unwrap();
        let mv = import_folder_with(&msrc, Some("Piastrelle"), None, "t", Transfer::Move).unwrap();
        for n in ["tile_diff_1k.png", "tile_rough_1k.png", "tile_nor_gl_1k.png", "tile_1k.mtlx"] {
            assert!(mv.dir.join(n).is_file(), "{n} spostato");
            assert!(!msrc.join(n).exists(), "{n} non più nell'origine");
        }
        for n in ["tile_nor_dx_1k.png", "tile_1k.blend"] {
            assert!(msrc.join(n).is_file(), "{n} resta nell'origine");
            assert!(!mv.dir.join(n).exists());
        }
        delete(&mv).unwrap();
        // Errore a metà (ARM illeggibile da separare): i file spostati tornano indietro.
        let bsrc = lib_root("move_bad");
        gray(&bsrc.join("bad_diff_1k.png"), 120);
        std::fs::write(bsrc.join("bad_arm_1k.png"), "non è un'immagine").unwrap();
        assert!(import_folder_with(&bsrc, Some("Rotto"), None, "t", Transfer::Move).is_err());
        assert!(bsrc.join("bad_diff_1k.png").is_file(), "il colore è tornato nell'origine");
        assert!(!crate::paths::materials_dir().join("Rotto").exists());
        std::fs::remove_dir_all(msrc).ok();
        std::fs::remove_dir_all(bsrc).ok();

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
            z.start_file("Metal012.mtlx", opt).unwrap();
            z.write_all(br#"<materialx><displacement name="d"><input name="scale" type="float" value="0.05"/></displacement></materialx>"#)
                .unwrap();
            z.finish().unwrap();
        }
        let z = import_zip(&zpath, None, None, "ambientCG Metal012").unwrap();
        assert_eq!(z.material.name, "Metal012");
        assert_eq!(z.material.metallic, 1.0);
        assert!(z.material.metallic_texture.is_some());
        assert_eq!(z.source, "ambientCG Metal012");
        // Il file MaterialX dello zip resta nella cartella del materiale.
        let mx = crate::mtlx::find_in(&z.dir).expect("mtlx conservato");
        assert_eq!(crate::mtlx::displacement_scale(&crate::mtlx::read(&mx).unwrap()), Some(0.05));
        assert!(!z.dir.join("Metal012.usdc").exists());

        // Politica del displacement all'import (impostazione dell'app).
        assert_eq!(import_displacement(), ImportDisplacement::Off);
        let hsrc = lib_root("hsrc");
        gray(&hsrc.join("Rock_Color.png"), 120);
        gray(&hsrc.join("Rock_Displacement.png"), 90);
        std::fs::write(
            hsrc.join("Rock.mtlx"),
            r#"<materialx><displacement name="d"><input name="scale" type="float" value="0.02"/></displacement></materialx>"#,
        )
        .unwrap();
        let off = import_folder(&hsrc, Some("Roccia off"), Some(2.0), "t").unwrap();
        assert_eq!(off.material.displacement_mm, 0.0);
        set_import_displacement(ImportDisplacement::Proposed);
        let prop = import_folder(&hsrc, Some("Roccia proposto"), Some(2.0), "t").unwrap();
        assert_eq!(prop.material.displacement_mm, default_displacement_mm(Some(2.0)));
        set_import_displacement(ImportDisplacement::FromMaterialX);
        let mtlx = import_folder(&hsrc, Some("Roccia mtlx"), Some(2.0), "t").unwrap();
        assert!((mtlx.material.displacement_mm - 20.0).abs() < 1e-3);
        set_import_displacement(ImportDisplacement::Off);
        for m in [off, prop, mtlx] {
            delete(&m).unwrap();
        }
        std::fs::remove_dir_all(hsrc).ok();

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
        // Le note dell'import restano anche dopo "Salva".
        let noted = set_notes(&all[0].dir, &["mappa \"Displacement\" non disponibile".to_string()]).unwrap();
        assert_eq!(noted.notes.len(), 1);
        assert_eq!(update(&all[0].dir, &edited).unwrap().notes, noted.notes);
        // Anche categorie e tag della fonte restano dopo "Salva".
        let cats = vec!["brick".to_string(), "wall".to_string()];
        let tagged = set_catalog_info(&all[0].dir, &cats, &["red".to_string()]).unwrap();
        assert_eq!(tagged.categories, cats);
        let kept = update(&all[0].dir, &edited).unwrap();
        assert_eq!((kept.categories, kept.tags), (cats, vec!["red".to_string()]));
        assert_eq!(update(&all[0].dir, &edited).unwrap().notes, noted.notes);
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
