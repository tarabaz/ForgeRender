//! Converte i materiali BlenderKit scaricati (`.blend`) in materiali predefiniti di ForgeRender:
//! per ognuno una cartella `<uscita>\<id>\` con le mappe a 1024 px (JPEG) e `materiale.json`
//! (valori degli slider, mappe con il nome del file). Il programma li legge all'avvio
//! (`forge_scene::material::presets`).
//!
//! Licenza: i materiali BlenderKit gratuiti sono "Royalty Free": vanno bene per l'uso personale
//! del programma, non per distribuirlo. Per questo stanno in una cartella a parte.
//!
//! ```text
//! cargo run --release -p forge_library --example blenderkit_predefiniti -- <cartella .blend> assets\materiali-blenderkit
//! ```

use forge_library::blenderkit::{self, Slot};
use forge_scene::material::{Material, MaterialId, Projection, TextureMapping};
use image::imageops::FilterType;
use std::path::{Path, PathBuf};

const SIZE: u32 = 1024;
const METALS: &str = "Metalli BlenderKit";
const OTHERS: &str = "Legni BlenderKit";

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    anyhow::ensure!(args.len() == 2, "uso: blenderkit_predefiniti <cartella .blend> <uscita>");
    let out = PathBuf::from(&args[1]);
    std::fs::create_dir_all(&out)?;
    let mut ids: Vec<String> = Vec::new();
    let mut lines = Vec::new();
    for f in blenderkit::scan(Path::new(&args[0])) {
        let folder = f
            .parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let x = match blenderkit::read(&f) {
            Ok(x) if !x.is_importable() => {
                println!("{folder}: {} procedurale ({}), saltato", x.name, x.procedural.join(", "));
                continue;
            }
            Ok(x) if x.has_images() => x,
            Ok(x) => {
                println!("{folder}: {} senza immagini, saltato", x.name);
                continue;
            }
            Err(e) => {
                println!("{folder}: saltato ({e:#})");
                continue;
            }
        };
        let mut name = italian_name(&x.name);
        let mut id = format!("bk-{}", slug(&name));
        let base = id.clone();
        let mut n = 2;
        while ids.contains(&id) {
            id = format!("{base}-{n}");
            name = format!("{} {n}", italian_name(&x.name));
            n += 1;
        }
        ids.push(id.clone());
        let dir = out.join(&id);
        std::fs::create_dir_all(&dir)?;
        let mut m = Material {
            id: MaterialId::new(&id),
            name: name.clone(),
            category: if x.material.metallic > 0.5 { METALS } else { OTHERS }.into(),
            ..x.material.clone()
        };
        m.mapping = TextureMapping {
            projection: Projection::Box,
            scale: x.scale_m,
            ..TextureMapping::default()
        };
        for img in &x.images {
            let decoded = match img.encoded().and_then(|(b, _)| Ok(image::load_from_memory(&b)?)) {
                Ok(i) => i,
                Err(e) => {
                    println!("{id}: «{}» non letta ({e}), saltata", img.name);
                    continue;
                }
            };
            let (file, gray) = match img.slot {
                Slot::Color => ("colore.jpg", false),
                Slot::Metallic => ("metallicita.jpg", true),
                Slot::Roughness => ("rugosita.jpg", true),
                Slot::Normal => ("normal.jpg", false),
                Slot::Height => ("rilievo.jpg", true),
            };
            let resized = decoded.resize_exact(SIZE.min(decoded.width()), SIZE.min(decoded.height()), FilterType::Lanczos3);
            let img_out: image::DynamicImage = if gray {
                image::DynamicImage::ImageLuma8(resized.to_luma8())
            } else {
                image::DynamicImage::ImageRgb8(resized.to_rgb8())
            };
            let mut w = std::io::BufWriter::new(std::fs::File::create(dir.join(file))?);
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut w, if img.slot == Slot::Normal { 95 } else { 90 })
                .encode_image(&img_out)?;
            let slot = match img.slot {
                Slot::Color => &mut m.base_color_texture,
                Slot::Metallic => &mut m.metallic_texture,
                Slot::Roughness => &mut m.roughness_texture,
                Slot::Normal => &mut m.normal_texture,
                Slot::Height => &mut m.height_texture,
            };
            *slot = Some(PathBuf::from(file));
        }
        std::fs::write(dir.join("materiale.json"), serde_json::to_string_pretty(&m)? + "\n")?;
        lines.push(format!("| `{id}` | {name} | {} | `{folder}` |", x.name));
        println!("{id}: {name} ({} mappe)", x.images.len());
    }
    let readme = format!(
        "# Metalli BlenderKit (licenza Royalty Free)\n\n\
         Convertiti dai `.blend` scaricati da BlenderKit con l'esempio `blenderkit_predefiniti` di\n\
         `forge_library`. Licenza **Royalty Free**: si usano nei propri render, ma i file non si\n\
         ridistribuiscono. Vanno bene finché ForgeRender resta per uso personale (repository privato);\n\
         prima di distribuire il programma questa cartella va tolta o sostituita con materiali CC0.\n\n\
         | id | Nome | Nome BlenderKit | Cartella scaricata |\n|---|---|---|---|\n{}\n",
        lines.join("\n")
    );
    std::fs::write(out.join("LEGGIMI.md"), readme)?;
    Ok(())
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.to_lowercase().chars() {
        let c = match c {
            'à' | 'á' => 'a',
            'è' | 'é' => 'e',
            'ì' => 'i',
            'ò' | 'ó' => 'o',
            'ù' => 'u',
            c => c,
        };
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

/// Nome italiano (quelli non in tabella restano come in BlenderKit).
fn italian_name(en: &str) -> String {
    let key = en
        .trim()
        .trim_end_matches(|c: char| c == '.' || c.is_ascii_digit())
        .trim()
        .to_lowercase();
    let it = match key.as_str() {
        "aged gunmetal steel" => "Acciaio canna di fucile invecchiato",
        "black steel" => "Acciaio nero",
        "blue steel" => "Acciaio brunito blu",
        "brushed steel" => "Acciaio spazzolato",
        "brushed stainless steel" => "Acciaio inox spazzolato",
        "galvanized steel" => "Acciaio zincato",
        "golden scratched painted steel" => "Acciaio verniciato oro graffiato",
        "heavily oxidized steel" => "Acciaio molto ossidato",
        "heavily scratched rusty iron" => "Ferro arrugginito molto graffiato",
        "metal 042a" => "Metallo 042A",
        "metal galvanized steel grime" => "Acciaio zincato sporco",
        "metal rusty grate weave" => "Griglia metallica arrugginita",
        "oak wood" => "Rovere (BlenderKit)",
        "oxidized corroded steel" => "Acciaio ossidato corroso",
        "oxidized steel" => "Acciaio ossidato",
        "painted steel" => "Acciaio verniciato",
        "phosphated galvanized steel" => "Acciaio zincato fosfatato",
        "pink scratched painted steel" => "Acciaio verniciato rosa graffiato",
        "raw steel" => "Acciaio grezzo",
        "scraped steel copper patina" => "Acciaio raschiato con patina di rame",
        "scratched steel metal" => "Acciaio graffiato",
        "silver steel" => "Acciaio argento",
        "soft pink scratched painted steel" => "Acciaio verniciato rosa tenue graffiato",
        "stainless steel" => "Acciaio inox",
        "steel copper" => "Acciaio ramato",
        "steel gold" => "Acciaio dorato",
        "steel niquel" => "Acciaio nichelato",
        "steel painted scraped" => "Acciaio verniciato scrostato",
        "steel satin metal" => "Acciaio satinato",
        "steel silver" => "Acciaio argentato",
        "steel tank painted" => "Acciaio verniciato serbatoio",
        "steel titanium" => "Acciaio titanio",
        "weathered aged steel" => "Acciaio invecchiato dalle intemperie",
        "weathered galvanized steel" => "Acciaio zincato invecchiato",
        _ => return en.trim().to_string(),
    };
    it.to_string()
}
