//! Materiali predefiniti.
//!
//! Senza argomenti: una riga per materiale, `categoria<TAB>id<TAB>nome<TAB>mappe` (`mappe` = sì
//! se ha già colore, rugosità o normal).
//!
//! Con `--cartelle <radice>`: crea `<NN Categoria>\<Nome>\impostazioni.json` per ogni
//! materiale, con i valori degli slider dell'editor (stessi nomi e stesso ordine
//! dell'interfaccia). È la scheda da regolare per i nuovi predefiniti; un file già presente non
//! viene sovrascritto.
//!
//! ```text
//! cargo run --release -p forge_scene --example elenco_predefiniti -- --cartelle <radice>
//! ```

use forge_scene::material::{linear_to_srgb, presets, Material, Projection};
use serde::Serialize;
use std::path::Path;

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let all = presets();
    if args.first().map(String::as_str) != Some("--cartelle") {
        for m in &all {
            let maps = if m.has_textures() { "sì" } else { "no" };
            println!("{}\t{}\t{}\t{maps}", m.category, m.id.0, m.name);
        }
        return Ok(());
    }
    let root = Path::new(args.get(1).expect("--cartelle <radice>"));
    let mut categories: Vec<&str> = Vec::new();
    let (mut written, mut kept) = (0, 0);
    for m in &all {
        if !categories.contains(&m.category.as_str()) {
            categories.push(&m.category);
        }
        let n = categories.iter().position(|c| *c == m.category).unwrap() + 1;
        let dir = root.join(format!("{n:02} {}", m.category)).join(folder_name(&m.name));
        std::fs::create_dir_all(&dir)?;
        let file = dir.join("impostazioni.json");
        if file.exists() {
            kept += 1;
            continue;
        }
        std::fs::write(&file, serde_json::to_string_pretty(&Card::of(m)).unwrap() + "\n")?;
        written += 1;
    }
    println!("schede scritte: {written}, già presenti (non toccate): {kept}");
    Ok(())
}

/// Nome di cartella valido su Windows ("POM / Delrin bianco" → "POM - Delrin bianco").
fn folder_name(name: &str) -> String {
    let s: String = name.chars().map(|c| if "<>:\"/\\|?*".contains(c) { '-' } else { c }).collect();
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn hex(c: [f32; 3]) -> String {
    let b = c.map(|v| (linear_to_srgb(v.clamp(0.0, 1.0)) * 255.0).round() as u8);
    format!("#{:02x}{:02x}{:02x}", b[0], b[1], b[2])
}

/// Numero con al massimo tre decimali (0.6000000238 → 0.6).
fn r(v: f32) -> f64 {
    (v as f64 * 1000.0).round() / 1000.0
}

fn file(p: &Option<std::path::PathBuf>) -> Option<String> {
    p.as_ref().and_then(|p| p.file_name()).map(|f| f.to_string_lossy().into_owned())
}

/// Scheda del materiale: campi nello stesso ordine e con gli stessi nomi dell'editor.
#[derive(Serialize)]
struct Card {
    materiale: String,
    id: String,
    categoria: String,
    note: &'static str,
    slider: Sliders,
    mappe: Maps,
    #[serde(rename = "Scala e posizionamento texture")]
    mapping: Mapping,
}

#[derive(Serialize)]
struct Sliders {
    #[serde(rename = "Colore base")]
    base_color: String,
    #[serde(rename = "Metallic")]
    metallic: f64,
    #[serde(rename = "Roughness")]
    roughness: f64,
    #[serde(rename = "Specular")]
    specular: f64,
    #[serde(rename = "IOR")]
    ior: f64,
    #[serde(rename = "Transmission")]
    transmission: f64,
    #[serde(rename = "Colore in volume")]
    volume_color: String,
    #[serde(rename = "Profondità colore (cm)")]
    volume_depth_cm: f64,
    #[serde(rename = "Priorità volume")]
    priority: i32,
    #[serde(rename = "Emissione")]
    emission: String,
    #[serde(rename = "Intensità emissione")]
    emission_strength: f64,
}

#[derive(Serialize)]
struct Maps {
    colore: Option<String>,
    #[serde(rename = "rugosità")]
    roughness: Option<String>,
    normal: Option<String>,
    #[serde(rename = "metallicità")]
    metallic: Option<String>,
    #[serde(rename = "rilievo (height)")]
    height: Option<String>,
    #[serde(rename = "occlusione (AO)")]
    ao: Option<String>,
    #[serde(rename = "Formato DirectX")]
    directx: bool,
    #[serde(rename = "Intensità normal map")]
    normal_strength: f64,
    #[serde(rename = "Bump (mm)")]
    bump_mm: f64,
    #[serde(rename = "Displacement (mm)")]
    displacement_mm: f64,
    #[serde(rename = "Intensità AO")]
    ao_strength: f64,
}

#[derive(Serialize)]
struct Mapping {
    #[serde(rename = "Proiezione")]
    projection: &'static str,
    #[serde(rename = "Dimensione di una ripetizione (m)")]
    scale: f64,
    #[serde(rename = "Rotazione (°)")]
    rotation: f64,
    #[serde(rename = "Spostamento")]
    offset: [f64; 2],
}

impl Card {
    fn of(m: &Material) -> Card {
        let t = &m.mapping;
        Card {
            materiale: m.name.clone(),
            id: m.id.0.clone(),
            categoria: m.category.clone(),
            note: "Valori degli slider dell'editor del materiale. Colori in sRGB come nel selettore. \
                   Con una mappa, Colore base, Roughness e Metallic la moltiplicano.",
            slider: Sliders {
                base_color: hex(m.base_color),
                metallic: r(m.metallic),
                roughness: r(m.roughness),
                specular: r(m.specular),
                ior: r(m.ior),
                transmission: r(m.transmission),
                volume_color: hex(m.volume_color),
                volume_depth_cm: r(m.volume_depth_cm),
                priority: m.priority,
                emission: hex(m.emission),
                emission_strength: r(m.emission_strength),
            },
            mappe: Maps {
                colore: file(&m.base_color_texture),
                roughness: file(&m.roughness_texture),
                normal: file(&m.normal_texture),
                metallic: file(&m.metallic_texture),
                height: file(&m.height_texture),
                ao: file(&m.ao_texture),
                directx: m.normal_directx,
                normal_strength: r(m.normal_strength),
                bump_mm: r(m.bump_mm),
                displacement_mm: r(m.displacement_mm),
                ao_strength: r(m.ao_strength),
            },
            mapping: Mapping {
                projection: if t.projection == Projection::Box {
                    "Box (CAD)"
                } else {
                    "UV della mesh"
                },
                scale: r(t.scale),
                rotation: r(t.rotation_deg),
                offset: t.offset.map(r),
            },
        }
    }
}
