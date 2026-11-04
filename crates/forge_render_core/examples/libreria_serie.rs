//! Prepara la libreria dei materiali di serie dagli zip di "Esporta zip…" seguendo
//! `tools/libreria-serie/tabella.json` (id, nome italiano, categoria, dispersione, compreso o
//! aggiuntivo per ogni materiale).
//!
//! Uscite:
//! - `<materiali>/<NN Categoria>/<id>/`: i materiali compresi nel programma, mappe al massimo
//!   di [`COMPRESI_PX`] px (la libreria di serie resta leggera);
//! - `<aggiuntivi>/<NN Categoria>/<id>/`: il pacchetto aggiuntivo, mappe a piena risoluzione;
//! - `<zip>/MATERIALI COMPRESI|MATERIALI AGGIUNTIVI/<NN Categoria>/<id>.zip`: gli stessi
//!   materiali a piena risoluzione come zip di "Esporta zip…" (gli originali da conservare).
//!
//! Le mappe EXR si convertono senza cambiare il render: il motore usa rugosità, normal,
//! metallicità e occlusione a 8 bit (vedi `Texture::load_with`), quindi diventano JPEG di qualità
//! 95; la height è letta a 16 bit e diventa un PNG a 16 bit; il colore lineare diventa sRGB.
//!
//! ```text
//! cargo run --release -p forge_render_core --example libreria_serie -- <tabella.json> <zip originali> <zip sostituti> <materiali> <aggiuntivi> <zip>
//! ```

use forge_render_core::texture::open_image;
use image::{imageops::FilterType, DynamicImage, GrayImage, ImageBuffer, Luma, RgbImage};
use rayon::prelude::*;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Lato massimo delle mappe dei materiali compresi nel programma.
const COMPRESI_PX: u32 = 1024;

/// Campi del json che indicano le mappe.
const SLOTS: [&str; 7] = [
    "base_color_texture",
    "roughness_texture",
    "normal_texture",
    "metallic_texture",
    "height_texture",
    "ao_texture",
    "opacity_texture",
];

struct Entry {
    src: String,
    id: String,
    name: String,
    cat_dir: String,
    cat: String,
    included: bool,
    extra: Value,
}

fn main() -> anyhow::Result<()> {
    let a: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).collect();
    anyhow::ensure!(
        a.len() == 6,
        "uso: libreria_serie <tabella.json> <zip originali> <zip sostituti> <materiali> <aggiuntivi> <zip>"
    );
    let table: Value = serde_json::from_str(&std::fs::read_to_string(&a[0])?)?;
    let cats = table["categorie"].as_object().unwrap();
    let mut entries = Vec::new();
    for e in table["materiali"].as_array().unwrap() {
        let num = e[3].as_str().unwrap();
        let cat = cats[num].as_str().unwrap().to_string();
        entries.push(Entry {
            src: e[0].as_str().unwrap().into(),
            id: e[1].as_str().unwrap().into(),
            name: e[2].as_str().unwrap().into(),
            cat_dir: format!("{num} {cat}"),
            cat,
            included: e[4].as_bool().unwrap(),
            extra: e.get(5).cloned().unwrap_or(json!({})),
        });
    }
    check_table(&entries, &a[1])?;
    for d in [
        a[3].clone(),
        a[4].clone(),
        a[5].join("MATERIALI COMPRESI"),
        a[5].join("MATERIALI AGGIUNTIVI"),
    ] {
        clean(&d)?;
    }
    let tmp = std::env::temp_dir().join(format!("forge_libreria_serie_{}", std::process::id()));
    let results: Vec<anyhow::Result<(bool, u64)>> = entries
        .par_iter()
        .map(|e| {
            let work = tmp.join(&e.id);
            let r = build(e, &a, &work);
            std::fs::remove_dir_all(&work).ok();
            r.map_err(|err| err.context(format!("{} ({})", e.id, e.src)))
        })
        .collect();
    std::fs::remove_dir_all(&tmp).ok();
    let (mut inc, mut ext, mut inc_b, mut ext_b, mut errors) = (0, 0, 0u64, 0u64, 0);
    for (r, e) in results.iter().zip(&entries) {
        match r {
            Ok((_, bytes)) if e.included => {
                inc += 1;
                inc_b += bytes;
            }
            Ok((_, bytes)) => {
                ext += 1;
                ext_b += bytes;
            }
            Err(err) => {
                eprintln!("ERRORE {err:#}");
                errors += 1;
            }
        }
    }
    println!(
        "compresi: {inc} materiali, {:.0} MB · aggiuntivi: {ext} materiali, {:.0} MB",
        inc_b as f64 / 1e6,
        ext_b as f64 / 1e6
    );
    anyhow::ensure!(errors == 0, "{errors} materiali con errori");
    Ok(())
}

/// Ogni zip della libreria originale è nella tabella (o è un BlenderKit escluso), niente id doppi.
fn check_table(entries: &[Entry], src: &Path) -> anyhow::Result<()> {
    let mut ids: Vec<&str> = entries.iter().map(|e| e.id.as_str()).collect();
    ids.sort();
    let n = ids.len();
    ids.dedup();
    anyhow::ensure!(ids.len() == n, "id doppi nella tabella");
    let mut missing = Vec::new();
    for cat in std::fs::read_dir(src)?.flatten().filter(|e| e.path().is_dir()) {
        for z in std::fs::read_dir(cat.path())?.flatten() {
            let p = z.path();
            if !p.extension().is_some_and(|x| x.eq_ignore_ascii_case("zip")) {
                continue;
            }
            let key = format!("{}/{}", cat.file_name().to_string_lossy(), p.file_stem().unwrap().to_string_lossy());
            if !entries.iter().any(|e| e.src == key) && !is_blenderkit(&p)? {
                missing.push(key);
            }
        }
    }
    if !missing.is_empty() {
        println!("non nella tabella (esclusi):");
        for m in &missing {
            println!("  {m}");
        }
    }
    Ok(())
}

/// Materiale BlenderKit: categoria "… BlenderKit" o mappe con i nomi dell'import BlenderKit.
fn is_blenderkit(zip: &Path) -> anyhow::Result<bool> {
    let mut a = zip::ZipArchive::new(std::fs::File::open(zip)?)?;
    let mut s = String::new();
    a.by_name("materiale.json")?.read_to_string(&mut s)?;
    let j: Value = serde_json::from_str(&s)?;
    let cat = j["category"].as_str().unwrap_or("");
    let names: Vec<String> = (0..a.len())
        .filter_map(|i| a.by_index(i).ok().map(|f| f.name().to_string()))
        .collect();
    Ok(cat.contains("BlenderKit") || names.iter().any(|n| n == "colore.jpg" || n == "rilievo.jpg"))
}

/// Prepara un materiale: estrae, sistema i campi, converte le mappe e scrive le uscite.
/// Restituisce (ha mappe, byte scritti nella libreria del programma).
fn build(e: &Entry, a: &[PathBuf], work: &Path) -> anyhow::Result<(bool, u64)> {
    std::fs::create_dir_all(work)?;
    let mut j = if let Some(rest) = e.src.strip_prefix("nuovo:") {
        let _ = rest;
        serde_json::to_value(forge_scene::Material::default())?
    } else {
        let (root, key) = match e.src.strip_prefix("sost:") {
            Some(k) => (&a[2], k),
            None => (&a[1], e.src.as_str()),
        };
        extract(&root.join(format!("{key}.zip")), work)?
    };
    j["id"] = json!(e.id);
    j["name"] = json!(e.name);
    j["category"] = json!(e.cat);
    if let Some(set) = e.extra["set"].as_object() {
        for (k, v) in set {
            j[k] = v.clone();
        }
    }
    if let Some(c) = e.extra.get("anodizzato") {
        // Alluminio anodizzato: la trama dell'alluminio spazzolato (rugosità e normal), la tinta
        // dal colore base. Colore, metallicità e height della base non servono.
        for s in ["base_color_texture", "metallic_texture", "height_texture", "ao_texture"] {
            j[s] = Value::Null;
        }
        j["base_color"] = c.clone();
        j["metallic"] = json!(1.0);
        j["roughness"] = json!(0.8);
        j["normal_strength"] = json!(0.5);
        j["mapping"] = json!({"projection": "Box", "scale": 0.15, "rotation_deg": 0.0, "offset": [0.0, 0.0], "anchor": "Component", "fit_object": false});
    }
    let transparent = j["transmission"].as_f64().unwrap_or(0.0) > 0.0;
    j["abbe"] = match e.extra.get("abbe") {
        Some(v) => v.clone(),
        None => json!(0.0),
    };
    anyhow::ensure!(
        !transparent || j["abbe"].as_f64().unwrap_or(0.0) > 0.0,
        "trasparente senza dispersione"
    );
    // Mappe: EXR convertite, a piena risoluzione nella cartella di lavoro.
    let mut maps = Vec::new();
    for slot in SLOTS {
        let Some(file) = j[slot].as_str().map(str::to_string) else {
            continue;
        };
        let path = work.join(&file);
        anyhow::ensure!(path.is_file(), "manca la mappa {file}");
        let ext = path.extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
        let new = if ext == "exr" {
            convert_exr(&path, slot)?
        } else if ext != "jpg" && ext != "jpeg" && slot != "height_texture" && slot != "opacity_texture" {
            // PNG, TIFF…: il motore le usa a 8 bit come le EXR.
            convert_exr(&path, slot)?
        } else {
            file
        };
        j[slot] = json!(new);
        maps.push((slot, new));
    }
    let has_maps = !maps.is_empty();
    // Il json deve essere un materiale valido.
    let m: forge_scene::Material = serde_json::from_value(j.clone())?;
    anyhow::ensure!(m.sanitized().abbe == m.abbe, "dispersione fuori intervallo");
    let pretty = serde_json::to_string_pretty(&j)?;

    let zip_dir = a[5]
        .join(if e.included { "MATERIALI COMPRESI" } else { "MATERIALI AGGIUNTIVI" })
        .join(&e.cat_dir);
    std::fs::create_dir_all(&zip_dir)?;
    write_zip(&zip_dir.join(format!("{}.zip", e.id)), &pretty, work, &maps)?;

    let out = a[if e.included { 3 } else { 4 }].join(&e.cat_dir).join(&e.id);
    std::fs::create_dir_all(&out)?;
    std::fs::write(out.join("materiale.json"), &pretty)?;
    let mut bytes = pretty.len() as u64;
    for (slot, file) in &maps {
        let src = work.join(file);
        let dst = out.join(file);
        if e.included {
            shrink(&src, &dst, slot)?;
        } else {
            std::fs::copy(&src, &dst)?;
        }
        bytes += std::fs::metadata(&dst)?.len();
    }
    Ok((has_maps, bytes))
}

/// Svuota una cartella delle uscite. Le cartelle aperte in Esplora risorse non si cancellano su
/// Windows: si tolgono i file e le cartelle che si lasciano togliere.
fn clean(dir: &Path) -> anyhow::Result<()> {
    let Ok(list) = std::fs::read_dir(dir) else { return Ok(()) };
    for e in list.flatten() {
        let p = e.path();
        // `_vecchi` e simili: cartelle tenute a mano, non uscite dello strumento.
        if e.file_name().to_string_lossy().starts_with('_') {
            continue;
        }
        if p.is_dir() {
            clean(&p)?;
            std::fs::remove_dir(&p).ok();
        } else {
            std::fs::remove_file(&p).map_err(|err| anyhow::anyhow!("{}: {err}", p.display()))?;
        }
    }
    Ok(())
}

fn extract(zip: &Path, dir: &Path) -> anyhow::Result<Value> {
    let mut a = zip::ZipArchive::new(std::fs::File::open(zip)?)?;
    let mut json = None;
    for i in 0..a.len() {
        let mut f = a.by_index(i)?;
        let name = f.name().to_string();
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        if name == "materiale.json" {
            json = Some(serde_json::from_slice(&buf)?);
        } else {
            std::fs::write(dir.join(Path::new(&name).file_name().unwrap()), &buf)?;
        }
    }
    json.ok_or_else(|| anyhow::anyhow!("manca materiale.json in {}", zip.display()))
}

fn write_zip(path: &Path, json: &str, dir: &Path, maps: &[(&str, String)]) -> anyhow::Result<()> {
    let mut z = zip::ZipWriter::new(std::fs::File::create(path)?);
    let opt = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let stored = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    z.start_file("materiale.json", opt)?;
    z.write_all(json.as_bytes())?;
    let mut done: Vec<&str> = Vec::new();
    for (_, f) in maps {
        if done.contains(&f.as_str()) {
            continue;
        }
        done.push(f);
        // JPEG e PNG sono già compressi.
        z.start_file(f.as_str(), stored)?;
        z.write_all(&std::fs::read(dir.join(f))?)?;
    }
    z.finish()?;
    Ok(())
}

/// Converte una mappa EXR come la userebbe il motore. Restituisce il nome del nuovo file.
fn convert_exr(path: &Path, slot: &str) -> anyhow::Result<String> {
    let img = open_image(path)?;
    let stem = path.file_stem().unwrap().to_string_lossy().to_string();
    let f = img.to_rgb32f();
    let (w, h) = f.dimensions();
    let float = matches!(img, DynamicImage::ImageRgb32F(_) | DynamicImage::ImageRgba32F(_));
    let out = if slot == "base_color_texture" && !float {
        DynamicImage::ImageRgb8(img.into_rgb8())
    } else if slot == "base_color_texture" {
        // Colore lineare → sRGB a 8 bit, come `Texture::load_with`.
        DynamicImage::ImageRgb8(RgbImage::from_fn(w, h, |x, y| {
            image::Rgb(
                f.get_pixel(x, y)
                    .0
                    .map(|c| (forge_scene::material::linear_to_srgb(c.clamp(0.0, 1.0)) * 255.0).round() as u8),
            )
        }))
    } else if slot == "height_texture" {
        // Height: 16 bit, come la legge il motore.
        let g: ImageBuffer<Luma<u16>, Vec<u16>> = ImageBuffer::from_fn(w, h, |x, y| {
            Luma([(f.get_pixel(x, y).0[0].clamp(0.0, 1.0) * 65535.0).round() as u16])
        });
        let name = format!("{stem}.png");
        DynamicImage::ImageLuma16(g).save(path.with_file_name(&name))?;
        std::fs::remove_file(path)?;
        return Ok(name);
    } else {
        let gray = f.pixels().all(|p| p.0[0] == p.0[1] && p.0[1] == p.0[2]);
        let rgb8 = img.into_rgb8();
        if gray {
            DynamicImage::ImageLuma8(GrayImage::from_fn(w, h, |x, y| Luma([rgb8.get_pixel(x, y).0[0]])))
        } else {
            DynamicImage::ImageRgb8(rgb8)
        }
    };
    let name = format!("{stem}.jpg");
    save_jpg(&out, &path.with_file_name(&name))?;
    std::fs::remove_file(path)?;
    Ok(name)
}

fn save_jpg(img: &DynamicImage, path: &Path) -> anyhow::Result<()> {
    let file = std::fs::File::create(path)?;
    let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(std::io::BufWriter::new(file), 95);
    enc.encode_image(img)?;
    Ok(())
}

/// Copia la mappa ridotta a [`COMPRESI_PX`] px sul lato lungo (se è più grande), nello stesso
/// formato: JPEG di qualità 92, PNG con la stessa profondità (la height resta a 16 bit).
fn shrink(src: &Path, dst: &Path, slot: &str) -> anyhow::Result<()> {
    let img = image::open(src)?;
    if img.width().max(img.height()) <= COMPRESI_PX {
        std::fs::copy(src, dst)?;
        return Ok(());
    }
    let small = img.resize(COMPRESI_PX, COMPRESI_PX, FilterType::Lanczos3);
    let jpg = dst
        .extension()
        .is_some_and(|x| x.eq_ignore_ascii_case("jpg") || x.eq_ignore_ascii_case("jpeg"));
    if jpg {
        let small = match small {
            DynamicImage::ImageLuma8(_) | DynamicImage::ImageRgb8(_) => small,
            DynamicImage::ImageLumaA8(_) | DynamicImage::ImageLuma16(_) => DynamicImage::ImageLuma8(small.to_luma8()),
            _ => DynamicImage::ImageRgb8(small.to_rgb8()),
        };
        let file = std::fs::File::create(dst)?;
        let q = if slot == "base_color_texture" { 92 } else { 95 };
        image::codecs::jpeg::JpegEncoder::new_with_quality(std::io::BufWriter::new(file), q).encode_image(&small)?;
    } else {
        small.save(dst)?;
    }
    Ok(())
}
