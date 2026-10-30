//! Import dei materiali BlenderKit scaricati sul PC (`blenderkit_data\materials\<nome>\*.blend`)
//! nella libreria locale, senza Blender.
//!
//! Dal file si leggono le immagini impacchettate e i nodi del materiale: partendo dal Principled
//! BSDF si risale ai nodi immagine (attraverso Normal Map, Bump, Invert, Separate, Mix…) per
//! sapere quale mappa va in quale slot, e dai valori dei nodi si prendono quelli degli slider
//! (colore, metallicità, rugosità, IOR, intensità della normal, scala del nodo Mapping).
//! I materiali fatti solo di nodi procedurali (rumore, rampe) non hanno immagini da estrarre.
//!
//! I materiali BlenderKit gratuiti sono in genere "Royalty Free": si usano nei propri render,
//! ma non si ridistribuiscono. Restano nella libreria del PC, non vanno nei predefiniti.

use crate::blend::{Blend, Inst};
use crate::store::{self, LibraryMaterial};
use anyhow::Context;
use forge_scene::material::{Material, Projection, TextureMapping};
use std::path::{Path, PathBuf};

/// Immagine estratta dal file, con lo slot in cui va.
#[derive(Clone, Debug)]
pub struct ExtractedImage {
    pub slot: Slot,
    /// Nome dell'immagine in Blender (es. "Brushed Steel_Roughness.jpg").
    pub name: String,
    pub bytes: Vec<u8>,
    pub ext: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    Color,
    Metallic,
    Roughness,
    Normal,
    Height,
}

impl Slot {
    fn file(self) -> &'static str {
        match self {
            Slot::Color => "colore",
            Slot::Metallic => "metallicita",
            Slot::Roughness => "rugosita",
            Slot::Normal => "normal",
            Slot::Height => "rilievo",
        }
    }
}

/// Materiale letto da un `.blend`: valori degli slider, immagini, note.
#[derive(Clone, Debug)]
pub struct Extracted {
    pub name: String,
    /// Valori degli slider (senza percorsi delle texture).
    pub material: Material,
    pub images: Vec<ExtractedImage>,
    pub notes: Vec<String>,
    /// Lato della ripetizione stimato dal nodo Mapping (metri).
    pub scale_m: f32,
}

impl Extracted {
    /// Ha almeno un'immagine (i materiali solo procedurali no).
    pub fn has_images(&self) -> bool {
        !self.images.is_empty()
    }
}

/// Origine di un ingresso del Principled: immagine trovata risalendo i collegamenti.
struct Traced<'a> {
    image: Inst<'a>,
    invert: bool,
    /// Passa da un nodo Normal Map (intensità) o Bump (rilievo).
    normal_strength: Option<f32>,
    bump: bool,
}

fn inputs<'a>(node: &Inst<'a>) -> Vec<Inst<'a>> {
    node.list("inputs")
}

fn input<'a>(node: &Inst<'a>, name: &str) -> Option<Inst<'a>> {
    inputs(node)
        .into_iter()
        .find(|s| s.str("name") == name || s.str("identifier") == name)
}

/// Nodo collegato a un ingresso.
fn linked<'a>(socket: &Inst<'a>) -> Option<Inst<'a>> {
    socket.deref("link")?.deref("fromnode")
}

fn float_value(socket: &Inst) -> Option<f32> {
    let v = socket.deref("default_value")?;
    if v.ty == "bNodeSocketValueFloat" {
        v.f32("value")
    } else {
        None
    }
}

fn vector_value(socket: &Inst) -> Vec<f32> {
    socket.deref("default_value").map(|v| v.f32s("value")).unwrap_or_default()
}

/// Risale i collegamenti fino a un nodo immagine (profondità limitata).
fn trace<'a>(node: &Inst<'a>, depth: u32) -> Option<Traced<'a>> {
    if depth > 12 {
        return None;
    }
    let idname = node.str("idname");
    let follow = |name: &str| input(node, name).as_ref().and_then(linked).and_then(|n| trace(&n, depth + 1));
    match idname.as_str() {
        "ShaderNodeTexImage" => node.deref("id").map(|image| Traced {
            image,
            invert: false,
            normal_strength: None,
            bump: false,
        }),
        "ShaderNodeInvert" => follow("Color").map(|t| Traced { invert: !t.invert, ..t }),
        "ShaderNodeNormalMap" => {
            let strength = input(node, "Strength").and_then(|s| float_value(&s)).unwrap_or(1.0);
            follow("Color").map(|t| Traced {
                normal_strength: Some(strength),
                ..t
            })
        }
        "ShaderNodeBump" => follow("Height").or_else(|| follow("Normal")).map(|t| Traced { bump: true, ..t }),
        // Nodi che modificano il valore (curve, mix, separazioni, rampe): si segue il primo
        // ingresso collegato che porta a un'immagine.
        _ => inputs(node).iter().filter_map(linked).find_map(|n| trace(&n, depth + 1)),
    }
}

/// Byte di un'immagine impacchettata (campo `packedfile` o lista `packedfiles`).
fn packed_bytes(image: &Inst) -> Option<Vec<u8>> {
    let pf = image
        .deref("packedfile")
        .or_else(|| image.list("packedfiles").first().and_then(|p| p.deref("packedfile")))?;
    let size = pf.i32("size")? as usize;
    pf.data_of("data", size).map(<[u8]>::to_vec).filter(|b| b.len() == size)
}

fn extension(bytes: &[u8], name: &str) -> &'static str {
    if bytes.starts_with(&[0xFF, 0xD8]) {
        "jpg"
    } else if bytes.starts_with(b"\x89PNG") {
        "png"
    } else if bytes.starts_with(b"II*\0") || bytes.starts_with(b"MM\0*") {
        "tif"
    } else if bytes.starts_with(&[0x76, 0x2F, 0x31, 0x01]) {
        "exr"
    } else {
        match Path::new(name)
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("png") => "png",
            Some("tif" | "tiff") => "tif",
            Some("exr") => "exr",
            _ => "jpg",
        }
    }
}

/// Legge il primo materiale con un Principled BSDF del file.
pub fn read(path: &Path) -> anyhow::Result<Extracted> {
    let blend = Blend::open(path)?;
    let materials: Vec<Inst> = blend.blocks.iter().filter(|b| &b.code[..2] == b"MA").map(|b| blend.at(b)).collect();
    let (mat, principled) = materials
        .iter()
        .find_map(|m| {
            let tree = m.deref("nodetree")?;
            let p = tree
                .list("nodes")
                .into_iter()
                .find(|n| n.str("idname") == "ShaderNodeBsdfPrincipled")?;
            Some((m.clone(), p))
        })
        .context("nessun materiale con un nodo Principled BSDF")?;
    let name = mat.sub("id").map(|id| id.str("name")).unwrap_or_default();
    let name = name.get(2..).unwrap_or(&name).trim().to_string();
    let mut m = Material {
        name: name.clone(),
        ..Default::default()
    };
    let mut images: Vec<ExtractedImage> = Vec::new();
    let mut notes = Vec::new();
    let take = |slot: Slot, t: &Traced, images: &mut Vec<ExtractedImage>, notes: &mut Vec<String>| -> bool {
        let iname = t.image.sub("id").map(|id| id.str("name")).unwrap_or_default();
        let iname = iname.get(2..).unwrap_or(&iname).to_string();
        match packed_bytes(&t.image) {
            Some(bytes) => {
                let ext = extension(&bytes, &iname);
                images.push(ExtractedImage {
                    slot,
                    name: iname,
                    bytes,
                    ext,
                });
                true
            }
            None => {
                notes.push(format!("immagine «{iname}» non impacchettata nel file: saltata"));
                false
            }
        }
    };
    // Colore base.
    if let Some(s) = input(&principled, "Base Color") {
        match linked(&s).and_then(|n| trace(&n, 0)) {
            Some(t) if take(Slot::Color, &t, &mut images, &mut notes) => m.base_color = [1.0; 3],
            other => {
                let v = vector_value(&s);
                if v.len() >= 3 {
                    m.base_color = [v[0], v[1], v[2]];
                }
                if other.is_none() && linked(&s).is_some() {
                    notes.push("colore fatto con nodi procedurali: usato il colore del nodo".into());
                }
            }
        }
    }
    // Metallicità e rugosità: mappa (il valore la moltiplica, quindi 1) oppure valore.
    for (slot, socket) in [(Slot::Metallic, "Metallic"), (Slot::Roughness, "Roughness")] {
        let Some(s) = input(&principled, socket) else { continue };
        let value = float_value(&s).unwrap_or(if slot == Slot::Metallic { 0.0 } else { 0.5 });
        match linked(&s).and_then(|n| trace(&n, 0)) {
            Some(t) if take(slot, &t, &mut images, &mut notes) => {
                if slot == Slot::Metallic {
                    m.metallic = 1.0;
                } else {
                    m.roughness = 1.0;
                    m.roughness_is_gloss = t.invert;
                }
            }
            other => {
                if slot == Slot::Metallic {
                    m.metallic = value;
                } else {
                    m.roughness = value;
                }
                if other.is_none() && linked(&s).is_some() {
                    notes.push(format!("{socket} fatto con nodi procedurali: usato il valore del nodo"));
                }
            }
        }
    }
    // Normal (Normal Map) o rilievo (Bump).
    if let Some(t) = input(&principled, "Normal").and_then(|s| linked(&s)).and_then(|n| trace(&n, 0)) {
        if t.normal_strength.is_some() && !t.bump {
            if take(Slot::Normal, &t, &mut images, &mut notes) {
                m.normal_strength = t.normal_strength.unwrap_or(1.0).clamp(0.0, 4.0);
            }
        } else if take(Slot::Height, &t, &mut images, &mut notes) {
            m.bump_mm = 0.3;
        }
    }
    if let Some(ior) = input(&principled, "IOR").and_then(|s| float_value(&s)) {
        m.ior = ior.clamp(1.0, 3.0);
    }
    // Scala: nodo Mapping (ripetizioni per unità); senza, 50 cm.
    let tree = mat.deref("nodetree").expect("albero già letto");
    let repeats = tree
        .list("nodes")
        .into_iter()
        .filter(|n| n.str("idname") == "ShaderNodeMapping")
        .find_map(|n| input(&n, "Scale").map(|s| vector_value(&s)))
        .filter(|v| v.len() >= 2 && v[0] > 0.0 && v[1] > 0.0)
        .map(|v| (v[0] + v[1]) * 0.5);
    let scale_m = repeats.map(|r| (1.0 / r).clamp(0.05, 5.0)).unwrap_or(0.5);
    notes.push(format!(
        "importato da BlenderKit ({}): lato della ripetizione stimato {:.2} m, da regolare",
        path.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default(),
        scale_m
    ));
    Ok(Extracted {
        name,
        material: m.sanitized(),
        images,
        notes,
        scale_m,
    })
}

/// Cartelle con un `.blend` sotto `root` (la struttura di `blenderkit_data\materials`).
pub fn scan(root: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = crate::paths::list_files_all(root, 3)
        .into_iter()
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("blend")))
        .collect();
    out.sort();
    out
}

/// Provenienza salvata nel materiale: cartella BlenderKit (con l'identificativo dell'asset).
fn source_of(path: &Path) -> String {
    let folder = path
        .parent()
        .and_then(|p| p.file_name())
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();
    format!("BlenderKit {folder} (Royalty Free: solo uso personale)")
}

/// Esito dell'import di un file.
pub enum Outcome {
    Imported(Box<LibraryMaterial>),
    AlreadyPresent(String),
    /// Solo nodi procedurali o nessuna immagine impacchettata.
    NoImages(String),
}

/// Importa un `.blend` nella libreria: immagini nella cartella del materiale, valori degli
/// slider, categoria "metal" se è un metallo. Un materiale con la stessa provenienza non si
/// importa due volte.
pub fn import(path: &Path) -> anyhow::Result<Outcome> {
    let source = source_of(path);
    if store::list().iter().any(|m| m.source == source) {
        return Ok(Outcome::AlreadyPresent(source));
    }
    let x = read(path)?;
    if !x.has_images() {
        return Ok(Outcome::NoImages(x.name));
    }
    let tmp = std::env::temp_dir().join(format!("forge_blenderkit_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp)?;
    let mut m = x.material.clone();
    for img in &x.images {
        let file = tmp.join(format!("{}.{}", img.slot.file(), img.ext));
        std::fs::write(&file, &img.bytes)?;
        let slot = match img.slot {
            Slot::Color => &mut m.base_color_texture,
            Slot::Metallic => &mut m.metallic_texture,
            Slot::Roughness => &mut m.roughness_texture,
            Slot::Normal => &mut m.normal_texture,
            Slot::Height => &mut m.height_texture,
        };
        *slot = Some(file);
    }
    m.mapping = TextureMapping {
        projection: Projection::Box,
        scale: x.scale_m,
        ..TextureMapping::default()
    };
    let result = store::save_as(&m, &x.name, &source);
    let _ = std::fs::remove_dir_all(&tmp);
    let saved = result?;
    let saved = store::set_notes(&saved.dir, &x.notes)?;
    let metal = m.metallic > 0.5 || m.metallic_texture.is_some();
    let categories: Vec<String> = if metal { vec!["metal".into()] } else { Vec::new() };
    let saved = store::set_catalog_info(&saved.dir, &categories, &["blenderkit".to_string()])?;
    Ok(Outcome::Imported(Box::new(saved)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_types_from_signature() {
        assert_eq!(extension(&[0xFF, 0xD8, 0xFF, 0xE0], "x.png"), "jpg");
        assert_eq!(extension(b"\x89PNG\r\n", "x.jpg"), "png");
        assert_eq!(extension(b"II*\0", "x"), "tif");
        assert_eq!(extension(b"????", "mappa.EXR"), "exr");
    }

    /// Prova su un materiale BlenderKit vero (non nel repository: licenza Royalty Free):
    /// `FORGE_BLENDERKIT_BLEND=…\file.blend cargo test -p forge_library -- --ignored blenderkit`.
    #[test]
    #[ignore]
    fn blenderkit_real_file() {
        let p = std::env::var("FORGE_BLENDERKIT_BLEND").expect("FORGE_BLENDERKIT_BLEND");
        let x = read(Path::new(&p)).unwrap();
        eprintln!(
            "{}: {:?}",
            x.name,
            x.images.iter().map(|i| (i.slot, &i.name, i.bytes.len())).collect::<Vec<_>>()
        );
        assert!(x.has_images());
        assert!(x.images.iter().any(|i| i.slot == Slot::Color));
    }
}
