//! Ricerca e download da ambientCG e Poly Haven (contenuti CC0, liberi anche
//! per uso commerciale).
//!
//! - ambientCG: API `https://ambientcg.com/api/v2/full_json`; ogni materiale si
//!   scarica come zip (`https://ambientcg.com/get?file=<id>_<1K|2K|4K>-JPG.zip`)
//!   e viene importato con [`crate::store::import_zip`].
//! - Poly Haven: elenco `https://api.polyhaven.com/assets?t=textures|hdris`, file
//!   `https://api.polyhaven.com/files/<id>`; le mappe si scaricano una per una.
//!
//! Il parsing è tollerante: se un campo manca si usano gli indirizzi standard.
//!
//! Il catalogo completo di ogni sito si scarica una volta e resta in cache su disco
//! (aggiornato dopo una settimana o a richiesta): la ricerca è locale e istantanea su
//! tutti i materiali, non solo sui primi risultati.

use crate::store::{self, LibraryMaterial};
use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Provider {
    AmbientCg,
    PolyHaven,
}

impl Provider {
    pub fn label(self) -> &'static str {
        match self {
            Provider::AmbientCg => "ambientCG",
            Provider::PolyHaven => "Poly Haven",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssetKind {
    Material,
    Hdri,
}

/// Risultato di una ricerca online.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OnlineAsset {
    pub provider: Provider,
    pub kind: AssetKind,
    pub id: String,
    pub name: String,
    pub category: String,
    pub tags: Vec<String>,
    pub thumbnail_url: String,
    /// Lato reale della texture in metri, se noto.
    pub size_m: Option<f32>,
}

/// Risoluzioni offerte nel download.
pub const RESOLUTIONS: [&str; 4] = ["1K", "2K", "4K", "8K"];

const USER_AGENT: &str = concat!("ForgeRender/", env!("CARGO_PKG_VERSION"));
/// Limite di sicurezza per un singolo download (le mappe 8K possono superare i 200 MB).
const MAX_DOWNLOAD: u64 = 1 << 30;

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .user_agent(USER_AGENT)
        .timeout_global(Some(std::time::Duration::from_secs(600)))
        .build()
        .into()
}

pub fn get_bytes(url: &str) -> anyhow::Result<Vec<u8>> {
    let mut resp = agent().get(url).call().with_context(|| format!("richiesta a {url}"))?;
    let bytes = resp.body_mut().with_config().limit(MAX_DOWNLOAD).read_to_vec()?;
    Ok(bytes)
}

fn get_json(url: &str) -> anyhow::Result<Value> {
    let bytes = get_bytes(url)?;
    serde_json::from_slice(&bytes).with_context(|| format!("risposta non valida da {url}"))
}

fn download_to(url: &str, path: &Path) -> anyhow::Result<()> {
    let bytes = get_bytes(url)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    // Scrittura atomica: un download interrotto non lascia file a metà.
    let tmp = path.with_extension("download");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

fn encode(q: &str) -> String {
    q.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' => (b as char).to_string(),
            b' ' => "+".into(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

// ------------------------------------------------------------------ ambientCG

pub fn ambientcg_search_url(query: &str, kind: AssetKind, offset: usize, limit: usize) -> String {
    let t = match kind {
        AssetKind::Material => "Material",
        AssetKind::Hdri => "HDRI",
    };
    format!(
        "https://ambientcg.com/api/v2/full_json?type={t}&q={}&limit={limit}&offset={offset}&sort=Popular&include=displayData,previewData,tagData,dimensionsData",
        encode(query)
    )
}

/// Interpreta la risposta di `full_json`.
pub fn parse_ambientcg(v: &Value, kind: AssetKind) -> Vec<OnlineAsset> {
    let Some(list) = v.get("foundAssets").and_then(Value::as_array) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|a| {
            let id = a.get("assetId")?.as_str()?.to_string();
            let name = a.get("displayName").and_then(Value::as_str).unwrap_or(&id).to_string();
            let category = a
                .get("displayCategory")
                .or_else(|| a.get("category"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let tags = a
                .get("tags")
                .and_then(Value::as_array)
                .map(|t| t.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
                .unwrap_or_default();
            let preview = a.get("previewImage");
            let thumbnail_url = ["256-PNG", "256-JPG-FFFFFF", "128-PNG", "512-PNG"]
                .iter()
                .find_map(|k| preview.and_then(|p| p.get(*k)).and_then(Value::as_str))
                .map(str::to_string)
                .unwrap_or_else(|| {
                    format!("https://acg-media.struffelproductions.com/file/ambientCG-Web/media/thumbnail/256-PNG/{id}.png")
                });
            // Dimensioni reali in cm (quando presenti).
            let size_m = ["dimensionX", "dimensionY"]
                .iter()
                .find_map(|k| a.get(*k).and_then(Value::as_f64))
                .filter(|cm| *cm > 0.0)
                .map(|cm| (cm / 100.0) as f32);
            Some(OnlineAsset {
                provider: Provider::AmbientCg,
                kind,
                id,
                name,
                category,
                tags,
                thumbnail_url,
                size_m,
            })
        })
        .collect()
}

pub fn ambientcg_download_url(id: &str, resolution: &str) -> String {
    format!("https://ambientcg.com/get?file={id}_{resolution}-JPG.zip")
}

// ------------------------------------------------------------------ Poly Haven

pub fn polyhaven_list_url(kind: AssetKind) -> &'static str {
    match kind {
        AssetKind::Material => "https://api.polyhaven.com/assets?t=textures",
        AssetKind::Hdri => "https://api.polyhaven.com/assets?t=hdris",
    }
}

/// Interpreta l'elenco `assets` (oggetto id → dati) e filtra per testo.
pub fn parse_polyhaven(v: &Value, kind: AssetKind, query: &str) -> Vec<OnlineAsset> {
    let Some(map) = v.as_object() else { return Vec::new() };
    let q = query.trim().to_lowercase();
    let mut out: Vec<(i64, OnlineAsset)> = map
        .iter()
        .filter_map(|(id, a)| {
            let name = a.get("name").and_then(Value::as_str).unwrap_or(id).to_string();
            let strings = |k: &str| -> Vec<String> {
                a.get(k)
                    .and_then(Value::as_array)
                    .map(|t| t.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
                    .unwrap_or_default()
            };
            let categories = strings("categories");
            let tags = strings("tags");
            if !q.is_empty() {
                let hay = format!("{id} {name} {} {}", categories.join(" "), tags.join(" ")).to_lowercase();
                if !q.split_whitespace().all(|w| hay.contains(w)) {
                    return None;
                }
            }
            // "dimensions": [larghezza, altezza] in millimetri.
            let size_m = a
                .get("dimensions")
                .and_then(Value::as_array)
                .and_then(|d| d.first())
                .and_then(Value::as_f64)
                .filter(|mm| *mm > 0.0)
                .map(|mm| (mm / 1000.0) as f32);
            let downloads = a.get("download_count").and_then(Value::as_i64).unwrap_or(0);
            Some((
                downloads,
                OnlineAsset {
                    provider: Provider::PolyHaven,
                    kind,
                    id: id.clone(),
                    name,
                    category: categories.first().cloned().unwrap_or_default(),
                    tags,
                    thumbnail_url: format!("https://cdn.polyhaven.com/asset_img/thumbs/{id}.png?width=256&height=256"),
                    size_m,
                },
            ))
        })
        .collect();
    // I più scaricati per primi, come sul sito.
    out.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.name.cmp(&b.1.name)));
    out.into_iter().map(|x| x.1).collect()
}

/// URL delle mappe di una texture Poly Haven alla risoluzione indicata (`files/<id>`).
/// Si sceglie PNG se disponibile, poi JPG; si salta la normal DirectX se c'è quella OpenGL
/// e la mappa ARM se ci sono le mappe separate.
pub fn polyhaven_texture_files(v: &Value, resolution: &str) -> Vec<String> {
    let res = resolution.to_lowercase();
    let Some(map) = v.as_object() else { return Vec::new() };
    let keys: Vec<String> = map.keys().map(|k| k.to_lowercase()).collect();
    let has = |k: &str| keys.iter().any(|x| x == k);
    let mut urls = Vec::new();
    for (key, entry) in map {
        let k = key.to_lowercase();
        if ["blend", "gltf", "mtlx", "usd", "fbx"].contains(&k.as_str()) {
            continue;
        }
        if k == "nor_dx" && has("nor_gl") {
            continue;
        }
        if k == "arm" && has("rough") {
            continue;
        }
        let Some(by_res) = entry.get(&res) else { continue };
        if let Some(url) = ["png", "jpg"]
            .iter()
            .find_map(|f| by_res.get(*f).and_then(|x| x.get("url")).and_then(Value::as_str))
        {
            urls.push(url.to_string());
        }
    }
    urls.sort();
    urls
}

/// URL della HDRI Poly Haven (`files/<id>` → `hdri` → risoluzione → `hdr`).
pub fn polyhaven_hdri_file(v: &Value, resolution: &str) -> Option<String> {
    let res = resolution.to_lowercase();
    v.get("hdri")?.get(&res)?.get("hdr")?.get("url")?.as_str().map(str::to_string)
}

// ------------------------------------------------------------------ Catalogo

/// Dopo quanto tempo il catalogo in cache si riscarica da solo.
const CATALOG_MAX_AGE: std::time::Duration = std::time::Duration::from_secs(7 * 24 * 3600);
/// Risultati per pagina chiesti ad ambientCG mentre si scarica il catalogo.
const ACG_PAGE: usize = 250;

fn catalog_path(provider: Provider, kind: AssetKind) -> PathBuf {
    let k = match kind {
        AssetKind::Material => "materiali",
        AssetKind::Hdri => "hdri",
    };
    crate::paths::cache_dir()
        .join("catalogo")
        .join(format!("{}_{k}.json", provider.label().replace(' ', "").to_lowercase()))
}

fn fetch_catalog(provider: Provider, kind: AssetKind) -> anyhow::Result<Vec<OnlineAsset>> {
    match provider {
        Provider::AmbientCg => {
            let mut all: Vec<OnlineAsset> = Vec::new();
            let mut seen = std::collections::HashSet::new();
            let mut offset = 0;
            // Pagina dopo pagina finché il sito ne restituisce di nuove (il sito può dare pagine
            // più corte di quelle chieste); al massimo 400 pagine come protezione.
            for _ in 0..400 {
                let v = get_json(&ambientcg_search_url("", kind, offset, ACG_PAGE))?;
                let got = parse_ambientcg(&v, kind);
                let before = all.len();
                offset += got.len();
                all.extend(got.into_iter().filter(|a| seen.insert(a.id.clone())));
                if all.len() == before {
                    break;
                }
            }
            Ok(all)
        }
        Provider::PolyHaven => Ok(parse_polyhaven(&get_json(polyhaven_list_url(kind))?, kind, "")),
    }
}

/// Catalogo completo del sito (dalla cache su disco se recente). Con `refresh` si
/// riscarica comunque; se il sito non risponde si usa la cache, anche vecchia.
pub fn catalog(provider: Provider, kind: AssetKind, refresh: bool) -> anyhow::Result<Vec<OnlineAsset>> {
    let path = catalog_path(provider, kind);
    let cached = || -> Option<Vec<OnlineAsset>> { serde_json::from_slice(&std::fs::read(&path).ok()?).ok() };
    let fresh = std::fs::metadata(&path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.elapsed().ok())
        .is_some_and(|age| age < CATALOG_MAX_AGE);
    if fresh && !refresh {
        if let Some(c) = cached() {
            return Ok(c);
        }
    }
    match fetch_catalog(provider, kind) {
        Ok(list) if !list.is_empty() => {
            if let Some(d) = path.parent() {
                let _ = std::fs::create_dir_all(d);
            }
            let _ = std::fs::write(&path, serde_json::to_vec(&list)?);
            tracing::info!("catalogo {} scaricato: {} elementi", provider.label(), list.len());
            Ok(list)
        }
        other => match cached() {
            Some(c) => {
                tracing::warn!("catalogo {} non aggiornato, uso la copia in cache", provider.label());
                Ok(c)
            }
            None => other.and_then(|_| bail!("il catalogo di {} è vuoto", provider.label())),
        },
    }
}

/// Filtra il catalogo: tutte le parole devono comparire in id, nome, categoria o tag.
pub fn filter(assets: &[OnlineAsset], query: &str) -> Vec<OnlineAsset> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return assets.to_vec();
    }
    assets
        .iter()
        .filter(|a| {
            let hay = format!("{} {} {} {}", a.id, a.name, a.category, a.tags.join(" ")).to_lowercase();
            q.split_whitespace().all(|w| hay.contains(w))
        })
        .cloned()
        .collect()
}

// ------------------------------------------------------------------ Operazioni

/// Cerca materiali o HDRI sul sito indicato, nel catalogo completo.
pub fn search(provider: Provider, kind: AssetKind, query: &str, offset: usize, limit: usize) -> anyhow::Result<Vec<OnlineAsset>> {
    let all = catalog(provider, kind, false)?;
    Ok(filter(&all, query).into_iter().skip(offset).take(limit).collect())
}

/// Al massimo questo numero di miniature si scarica in contemporanea: con decine di
/// richieste insieme i siti rallentano o le rifiutano.
const MAX_THUMB_DOWNLOADS: usize = 6;

fn thumb_slot() -> impl Drop {
    use std::sync::{Condvar, Mutex};
    static STATE: (Mutex<usize>, Condvar) = (Mutex::new(0), Condvar::new());
    struct Slot;
    impl Drop for Slot {
        fn drop(&mut self) {
            *STATE.0.lock().unwrap() -= 1;
            STATE.1.notify_one();
        }
    }
    let mut n = STATE.0.lock().unwrap();
    while *n >= MAX_THUMB_DOWNLOADS {
        n = STATE.1.wait(n).unwrap();
    }
    *n += 1;
    Slot
}

/// Il materiale è già nella libreria locale (scaricato prima, a qualsiasi risoluzione)?
pub fn already_downloaded(asset: &OnlineAsset, local_sources: &std::collections::HashSet<String>) -> bool {
    local_sources.contains(&source_key(asset.provider, &asset.id))
}

/// Chiave "sito id" ricavata dall'origine salvata con i materiali scaricati.
pub fn source_key(provider: Provider, id: &str) -> String {
    format!("{} {}", provider.label(), id)
}

/// La HDRI è già nella cartella HDRI della libreria (a qualsiasi risoluzione)?
pub fn hdri_downloaded(asset: &OnlineAsset, local_hdris: &[PathBuf]) -> bool {
    let id = asset.id.to_lowercase();
    local_hdris.iter().any(|p| {
        p.file_stem()
            .map(|s| s.to_string_lossy().to_lowercase())
            .is_some_and(|s| s == id || s.starts_with(&format!("{id}_")))
    })
}

/// Chiavi "sito id" dei materiali della libreria (per riconoscere quelli già scaricati).
pub fn local_source_keys() -> std::collections::HashSet<String> {
    store::list()
        .into_iter()
        .filter_map(|m| m.source.rsplit_once(" (").map(|(k, _)| k.to_string()))
        .collect()
}

/// Miniatura (PNG/JPG), con cache su disco.
pub fn thumbnail(asset: &OnlineAsset) -> anyhow::Result<Vec<u8>> {
    let cache = crate::paths::cache_dir().join("miniature").join(format!(
        "{}_{}.img",
        asset.provider.label().replace(' ', ""),
        store::folder_name(&asset.id)
    ));
    if let Ok(b) = std::fs::read(&cache) {
        return Ok(b);
    }
    let _slot = thumb_slot();
    let b = get_bytes(&asset.thumbnail_url)?;
    if let Some(d) = cache.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::write(&cache, &b);
    Ok(b)
}

/// Scarica un materiale e lo aggiunge alla libreria.
pub fn download_material(asset: &OnlineAsset, resolution: &str, progress: &dyn Fn(&str)) -> anyhow::Result<LibraryMaterial> {
    let source = format!("{} {} ({resolution})", asset.provider.label(), asset.id);
    let name = format!("{} {resolution}", asset.name);
    match asset.provider {
        Provider::AmbientCg => {
            progress("download dello zip…");
            let tmp = std::env::temp_dir().join(format!("forgerender_{}_{resolution}.zip", asset.id));
            download_to(&ambientcg_download_url(&asset.id, resolution), &tmp)?;
            progress("importazione delle mappe…");
            let r = store::import_zip(&tmp, Some(&name), asset.size_m, &source);
            let _ = std::fs::remove_file(&tmp);
            r
        }
        Provider::PolyHaven => {
            let files = get_json(&format!("https://api.polyhaven.com/files/{}", asset.id))?;
            let urls = polyhaven_texture_files(&files, resolution);
            if urls.is_empty() {
                bail!("{}: nessuna mappa disponibile a {resolution}", asset.name);
            }
            let tmp = std::env::temp_dir().join(format!("forgerender_ph_{}_{resolution}", asset.id));
            std::fs::create_dir_all(&tmp)?;
            let r = (|| {
                for (i, url) in urls.iter().enumerate() {
                    progress(&format!("mappa {}/{}…", i + 1, urls.len()));
                    let fname = url.rsplit('/').next().unwrap_or("mappa.jpg");
                    let fname = fname.split('?').next().unwrap_or(fname);
                    download_to(url, &tmp.join(fname))?;
                }
                progress("importazione delle mappe…");
                store::import_folder(&tmp, Some(&name), asset.size_m, &source)
            })();
            let _ = std::fs::remove_dir_all(&tmp);
            r
        }
    }
}

/// Scarica una HDRI nella cartella `Libreria/HDRI` e ne restituisce il percorso.
pub fn download_hdri(asset: &OnlineAsset, resolution: &str, progress: &dyn Fn(&str)) -> anyhow::Result<PathBuf> {
    let dir = crate::paths::hdri_dir();
    match asset.provider {
        Provider::PolyHaven => {
            let files = get_json(&format!("https://api.polyhaven.com/files/{}", asset.id))?;
            let url = polyhaven_hdri_file(&files, resolution).with_context(|| format!("{}: nessuna HDRI a {resolution}", asset.name))?;
            let out = dir.join(format!("{}_{}.hdr", asset.id, resolution.to_lowercase()));
            progress("download della HDRI…");
            download_to(&url, &out)?;
            Ok(out)
        }
        Provider::AmbientCg => {
            // ambientCG distribuisce le HDRI come zip con file .exr/.hdr.
            let tmp = std::env::temp_dir().join(format!("forgerender_{}_{resolution}_hdri.zip", asset.id));
            progress("download della HDRI…");
            download_to(&format!("https://ambientcg.com/get?file={}_{resolution}-HDR.zip", asset.id), &tmp)?;
            let f = std::fs::File::open(&tmp)?;
            let mut z = zip::ZipArchive::new(f).context("archivio zip non valido")?;
            let mut found = None;
            for i in 0..z.len() {
                let mut e = z.by_index(i)?;
                let Some(name) = e.enclosed_name().and_then(|p| p.file_name().map(|n| n.to_owned())) else {
                    continue;
                };
                let n = name.to_string_lossy().to_lowercase();
                if n.ends_with(".hdr") || n.ends_with(".exr") {
                    std::fs::create_dir_all(&dir)?;
                    let out = dir.join(&name);
                    std::io::copy(&mut e, &mut std::fs::File::create(&out)?)?;
                    found = Some(out);
                    break;
                }
            }
            let _ = std::fs::remove_file(&tmp);
            found.context("lo zip non contiene una HDRI")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ambientcg_response() {
        let v: Value = serde_json::from_str(
            r#"{"foundAssets":[
                {"assetId":"Wood049","displayName":"Wood 049","displayCategory":"Wood","tags":["wood","oak"],
                 "previewImage":{"64-PNG":"https://x/64/Wood049.png","256-PNG":"https://x/256/Wood049.png"},
                 "dimensionX":200},
                {"assetId":"Metal012"}
            ],"numberOfResults":2}"#,
        )
        .unwrap();
        let r = parse_ambientcg(&v, AssetKind::Material);
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].name, "Wood 049");
        assert_eq!(r[0].thumbnail_url, "https://x/256/Wood049.png");
        assert_eq!(r[0].size_m, Some(2.0));
        assert_eq!(r[0].tags, ["wood", "oak"]);
        // Campi mancanti: indirizzo standard della miniatura.
        assert!(r[1].thumbnail_url.ends_with("/256-PNG/Metal012.png"));
        assert_eq!(
            ambientcg_download_url("Wood049", "2K"),
            "https://ambientcg.com/get?file=Wood049_2K-JPG.zip"
        );
        assert!(ambientcg_search_url("legno chiaro", AssetKind::Material, 0, 30).contains("q=legno+chiaro"));
    }

    #[test]
    fn catalog_filter_matches_all_words() {
        let a = |id: &str, name: &str, cat: &str, tags: &[&str]| OnlineAsset {
            provider: Provider::AmbientCg,
            kind: AssetKind::Material,
            id: id.into(),
            name: name.into(),
            category: cat.into(),
            tags: tags.iter().map(|t| t.to_string()).collect(),
            thumbnail_url: String::new(),
            size_m: None,
        };
        let all = vec![
            a("Wood049", "Wood 049", "Wood", &["oak", "planks"]),
            a("Metal012", "Metal 012", "Metal", &["brushed", "steel"]),
            a("Planks021", "Planks 021", "Wood", &["floor", "dark"]),
        ];
        assert_eq!(filter(&all, "").len(), 3);
        assert_eq!(filter(&all, "WOOD").len(), 2);
        let r = filter(&all, "wood dark");
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].id, "Planks021");
        assert!(filter(&all, "glass").is_empty());
        // Chiave dei già scaricati: "sito id" dall'origine salvata "sito id (risoluzione)".
        let mut local = std::collections::HashSet::new();
        local.insert(source_key(Provider::AmbientCg, "Metal012"));
        assert!(already_downloaded(&all[1], &local));
        assert!(!already_downloaded(&all[0], &local));
        let hdris = vec![PathBuf::from("/lib/HDRI/studio_small_2k.hdr")];
        let h = OnlineAsset {
            provider: Provider::PolyHaven,
            kind: AssetKind::Hdri,
            ..a("studio_small", "Studio Small", "", &[])
        };
        assert!(hdri_downloaded(&h, &hdris));
    }

    #[test]
    fn polyhaven_list_and_files() {
        let v: Value = serde_json::from_str(
            r#"{"rock_wall":{"name":"Rock Wall","categories":["rock","wall"],"tags":["stone"],"dimensions":[3000,3000],"download_count":10},
                "wood_floor":{"name":"Wood Floor","categories":["wood"],"tags":["floor"],"download_count":50}}"#,
        )
        .unwrap();
        let all = parse_polyhaven(&v, AssetKind::Material, "");
        assert_eq!(all.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(), ["wood_floor", "rock_wall"]);
        let rock = parse_polyhaven(&v, AssetKind::Material, "stone");
        assert_eq!(rock.len(), 1);
        assert_eq!(rock[0].size_m, Some(3.0));
        assert!(rock[0].thumbnail_url.contains("/thumbs/rock_wall.png"));

        let files: Value = serde_json::from_str(
            r#"{"Diffuse":{"1k":{"jpg":{"url":"https://dl/rock_wall_diff_1k.jpg"},"png":{"url":"https://dl/rock_wall_diff_1k.png"}}},
                "nor_gl":{"1k":{"jpg":{"url":"https://dl/rock_wall_nor_gl_1k.jpg"}}},
                "nor_dx":{"1k":{"jpg":{"url":"https://dl/rock_wall_nor_dx_1k.jpg"}}},
                "Rough":{"1k":{"jpg":{"url":"https://dl/rock_wall_rough_1k.jpg"}}},
                "arm":{"1k":{"jpg":{"url":"https://dl/rock_wall_arm_1k.jpg"}}},
                "blend":{"1k":{"blend":{"url":"https://dl/x.blend"}}}}"#,
        )
        .unwrap();
        let urls = polyhaven_texture_files(&files, "1K");
        assert_eq!(
            urls,
            [
                "https://dl/rock_wall_diff_1k.png",
                "https://dl/rock_wall_nor_gl_1k.jpg",
                "https://dl/rock_wall_rough_1k.jpg"
            ]
        );
        let hdri: Value = serde_json::from_str(r#"{"hdri":{"2k":{"hdr":{"url":"https://dl/studio_2k.hdr"}}}}"#).unwrap();
        assert_eq!(polyhaven_hdri_file(&hdri, "2K").as_deref(), Some("https://dl/studio_2k.hdr"));
    }

    /// Prova reale con la rete (da lanciare a mano: `cargo test -p forge_library -- --ignored`).
    #[test]
    #[ignore]
    fn live_search() {
        for p in [Provider::AmbientCg, Provider::PolyHaven] {
            let r = search(p, AssetKind::Material, "wood", 0, 5).unwrap();
            assert!(!r.is_empty(), "{}: nessun risultato", p.label());
            assert!(!thumbnail(&r[0]).unwrap().is_empty());
        }
    }
}
