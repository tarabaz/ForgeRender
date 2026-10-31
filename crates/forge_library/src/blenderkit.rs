//! Materiali dai file `.blend` (scaricati da BlenderKit o salvati da Blender), senza Blender.
//!
//! Per ogni materiale si parte dallo shader collegato al Material Output (Principled BSDF o
//! Metallic BSDF, anche dentro un gruppo di nodi) e si risale ai nodi che ne alimentano gli
//! ingressi: immagini (impacchettate nel file o accanto), Normal Map, Bump, Invert, Separate
//! Color, Mapping, Value, Mix… Ogni parametro finisce in un rapporto con il valore di Blender,
//! quello per ForgeRender e lo stato della conversione (esatta, approssimata, non supportata):
//! niente si perde in silenzio.
//!
//! I nodi **procedurali** (Noise, Voronoi, Wave…) non si convertono: ForgeRender usa mappe, non
//! nodi. Un materiale interamente procedurale (nessuna mappa a immagine) non si importa; uno
//! con solo una parte procedurale (sporco o rumore mescolati alle immagini) si importa con le
//! sole immagini e l'avviso dei nodi ignorati.
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
    /// Canale usato (Separate Color: 0 rosso, 1 verde, 2 blu); `None` = immagine intera.
    pub channel: Option<u8>,
    /// Da dove viene: "impacchettata nel file" o il percorso del file esterno.
    pub origin: String,
}

impl ExtractedImage {
    /// Byte da salvare ed estensione: l'immagine così com'è, oppure (con un canale) un PNG in
    /// scala di grigi di quel canale.
    pub fn encoded(&self) -> anyhow::Result<(Vec<u8>, &'static str)> {
        let Some(c) = self.channel else {
            return Ok((self.bytes.clone(), self.ext));
        };
        let img = image::load_from_memory(&self.bytes).with_context(|| format!("«{}» non leggibile", self.name))?;
        let rgb = img.to_rgb8();
        let gray = image::GrayImage::from_fn(rgb.width(), rgb.height(), |x, y| image::Luma([rgb.get_pixel(x, y).0[c as usize]]));
        let mut out = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageLuma8(gray).write_to(&mut out, image::ImageFormat::Png)?;
        Ok((out.into_inner(), "png"))
    }
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
    pub fn file(self) -> &'static str {
        match self {
            Slot::Color => "colore",
            Slot::Metallic => "metallicita",
            Slot::Roughness => "rugosita",
            Slot::Normal => "normal",
            Slot::Height => "rilievo",
        }
    }

    /// Nome dello slot nell'editor del materiale.
    pub fn label(self) -> &'static str {
        match self {
            Slot::Color => "Colore base",
            Slot::Metallic => "Metallic",
            Slot::Roughness => "Roughness",
            Slot::Normal => "Normal map",
            Slot::Height => "Rilievo (height)",
        }
    }
}

/// Stato della conversione di un parametro.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Stesso significato in ForgeRender.
    Exact,
    /// Convertito con una semplificazione (spiegata nella nota).
    Approximate,
    /// Non convertibile: il parametro resta al valore predefinito.
    Unsupported,
}

/// Riga del rapporto: parametro, valore in Blender, valore per ForgeRender, stato e nota.
#[derive(Clone, Debug)]
pub struct Param {
    pub name: &'static str,
    pub blender: String,
    pub forge: String,
    pub status: Status,
    pub note: String,
}

/// Materiale letto da un `.blend`.
#[derive(Clone, Debug)]
pub struct Extracted {
    pub name: String,
    /// Shader di partenza (es. "ShaderNodeBsdfPrincipled").
    pub shader: String,
    /// Valori degli slider (senza percorsi delle texture).
    pub material: Material,
    pub images: Vec<ExtractedImage>,
    /// Rapporto della conversione, nell'ordine dell'editor.
    pub params: Vec<Param>,
    /// Nodi procedurali che alimentano il materiale: se non è vuoto il materiale non si importa.
    pub procedural: Vec<String>,
    /// Immagini del file non collegate a nessuno slot (o non trovate).
    pub unused_images: Vec<String>,
    /// Avvisi generali (anche quelli salvati come note del materiale).
    pub notes: Vec<String>,
    /// Lato della ripetizione stimato dal nodo Mapping (metri).
    pub scale_m: f32,
}

impl Extracted {
    /// Ha almeno un'immagine.
    pub fn has_images(&self) -> bool {
        !self.images.is_empty()
    }

    /// Si può importare: c'è almeno una mappa a immagine. Un materiale interamente
    /// procedurale no; uno con solo una parte procedurale (sporco, rumore sopra le immagini)
    /// sì, con l'avviso dei nodi ignorati in [`Self::procedural`].
    pub fn is_importable(&self) -> bool {
        !self.images.is_empty()
    }

    /// Ha parti procedurali ignorate (ma resta importabile).
    pub fn is_partly_procedural(&self) -> bool {
        self.is_importable() && !self.procedural.is_empty()
    }

    /// Materiale per ForgeRender con le mappe salvate in `dir` (con il nome dello slot).
    pub fn material_with_maps(&self, dir: &Path) -> anyhow::Result<Material> {
        let mut m = self.material.clone();
        for img in &self.images {
            let (bytes, ext) = img.encoded()?;
            let file = dir.join(format!("{}.{ext}", img.slot.file()));
            std::fs::write(&file, bytes)?;
            *slot_mut(&mut m, img.slot) = Some(file);
        }
        Ok(m)
    }
}

fn slot_mut(m: &mut Material, slot: Slot) -> &mut Option<PathBuf> {
    match slot {
        Slot::Color => &mut m.base_color_texture,
        Slot::Metallic => &mut m.metallic_texture,
        Slot::Roughness => &mut m.roughness_texture,
        Slot::Normal => &mut m.normal_texture,
        Slot::Height => &mut m.height_texture,
    }
}

/// Nodi di texture procedurali di Blender.
const PROCEDURAL: [&str; 10] = [
    "ShaderNodeTexNoise",
    "ShaderNodeTexVoronoi",
    "ShaderNodeTexWave",
    "ShaderNodeTexMusgrave",
    "ShaderNodeTexGradient",
    "ShaderNodeTexMagic",
    "ShaderNodeTexChecker",
    "ShaderNodeTexBrick",
    "ShaderNodeTexWhiteNoise",
    "ShaderNodeTexGabor",
];

/// Nodi che modificano un valore: si seguono, ma la conversione diventa approssimata.
const PASS_THROUGH: [&str; 10] = [
    "ShaderNodeMath",
    "ShaderNodeMapRange",
    "ShaderNodeValToRGB",
    "ShaderNodeRGBCurve",
    "ShaderNodeGamma",
    "ShaderNodeBrightContrast",
    "ShaderNodeHueSaturation",
    "ShaderNodeClamp",
    "ShaderNodeVectorMath",
    "ShaderNodeRGBToBW",
];

/// Da dove arriva il valore di un ingresso.
#[derive(Clone)]
enum Kind<'a> {
    Image(Inst<'a>),
    Constant(Vec<f32>),
    Procedural(String),
    Unsupported(String),
}

#[derive(Clone)]
struct Origin<'a> {
    kind: Kind<'a>,
    invert: bool,
    channel: Option<u8>,
    normal_strength: Option<f32>,
    bump: bool,
    /// Nodi attraversati che rendono la conversione approssimata.
    approx: Vec<String>,
    /// Ripetizioni del nodo Mapping davanti all'immagine.
    repeats: Option<f32>,
    /// Coordinate della texture (UV, Generated, Object…), se indicate.
    coords: Option<String>,
    /// Normal DirectX ricomposta in OpenGL con i nodi (canale verde invertito).
    directx: bool,
    /// Nodi procedurali scartati lungo la strada (es. rumore mescolato all'immagine).
    procedural: Vec<String>,
}

impl<'a> Origin<'a> {
    fn of(kind: Kind<'a>) -> Self {
        Origin {
            kind,
            invert: false,
            channel: None,
            normal_strength: None,
            bump: false,
            approx: Vec::new(),
            repeats: None,
            coords: None,
            directx: false,
            procedural: Vec::new(),
        }
    }

    fn procedural_nodes(&self) -> Option<&str> {
        match &self.kind {
            Kind::Procedural(n) => Some(n),
            _ => None,
        }
    }
}

fn inputs<'a>(node: &Inst<'a>) -> Vec<Inst<'a>> {
    node.list("inputs")
}

fn input<'a>(node: &Inst<'a>, name: &str) -> Option<Inst<'a>> {
    inputs(node)
        .into_iter()
        .find(|s| s.str("identifier") == name || s.str("name") == name)
}

fn default_values(socket: &Inst) -> Vec<f32> {
    socket.deref("default_value").map(|v| v.f32s("value")).unwrap_or_default()
}

fn id_name(i: &Inst) -> String {
    let n = i.sub("id").map(|id| id.str("name")).unwrap_or_default();
    n.get(2..).unwrap_or(&n).trim().to_string()
}

/// Valore di un ingresso: collegamento risalito, oppure il valore scritto nel nodo.
/// `stack`: nodi gruppo attraversati (per tornare fuori dal gruppo dai Group Input).
fn resolve<'a>(socket: &Inst<'a>, stack: &mut Vec<Inst<'a>>, depth: u32) -> Origin<'a> {
    match socket.deref("link") {
        Some(link) => {
            let Some(from) = link.deref("fromnode") else {
                return Origin::of(Kind::Constant(default_values(socket)));
            };
            let out = link.deref("fromsock").map(|s| s.str("identifier")).unwrap_or_default();
            trace(&from, &out, stack, depth + 1)
        }
        None => Origin::of(Kind::Constant(default_values(socket))),
    }
}

/// Risale dall'uscita `out` del nodo.
fn trace<'a>(node: &Inst<'a>, out: &str, stack: &mut Vec<Inst<'a>>, depth: u32) -> Origin<'a> {
    let idname = node.str("idname");
    if depth > 40 {
        return Origin::of(Kind::Unsupported(format!("{idname} (catena troppo lunga)")));
    }
    let follow = |name: &str, stack: &mut Vec<Inst<'a>>| match input(node, name) {
        Some(s) => resolve(&s, stack, depth),
        None => Origin::of(Kind::Unsupported(format!("{idname}: manca l'ingresso {name}"))),
    };
    match idname.as_str() {
        // Punto di snodo sul filo: trasparente.
        "NodeReroute" => follow("Input", stack),
        "ShaderNodeTexImage" => {
            let Some(image) = node.deref("id") else {
                return Origin::of(Kind::Unsupported("Image Texture senza immagine".into()));
            };
            let mut o = Origin::of(Kind::Image(image));
            // Mapping davanti all'immagine: ripetizioni (Scale) e coordinate.
            if let Some(v) = input(node, "Vector")
                .and_then(|s| s.deref("link"))
                .and_then(|l| l.deref("fromnode"))
            {
                let mut cur = Some(v);
                while let Some(n) = cur.take() {
                    match n.str("idname").as_str() {
                        "ShaderNodeMapping" => {
                            if let Some(s) = input(&n, "Scale") {
                                if let Kind::Constant(v) = resolve(&s, stack, depth).kind {
                                    let v: Vec<f32> = v.into_iter().take(2).filter(|x| *x > 0.0).collect();
                                    if !v.is_empty() {
                                        o.repeats = Some(v.iter().sum::<f32>() / v.len() as f32);
                                    }
                                }
                            }
                            cur = input(&n, "Vector").and_then(|s| s.deref("link")).and_then(|l| l.deref("fromnode"));
                        }
                        "ShaderNodeTexCoord" | "ShaderNodeUVMap" => {
                            o.coords = Some(if n.str("idname") == "ShaderNodeUVMap" {
                                "UV".into()
                            } else {
                                "Texture Coordinate".into()
                            });
                        }
                        "NodeReroute" => {
                            cur = input(&n, "Input").and_then(|s| s.deref("link")).and_then(|l| l.deref("fromnode"));
                        }
                        other => {
                            o.approx.push(format!("coordinate da {other}"));
                        }
                    }
                }
            }
            if out == "Alpha" {
                o.channel = Some(3);
            }
            o
        }
        "ShaderNodeInvert" => {
            let mut o = follow("Color", stack);
            o.invert = !o.invert;
            o
        }
        "ShaderNodeNormalMap" => {
            let strength = input(node, "Strength").map(|s| default_values(&s)).and_then(|v| v.first().copied());
            let mut o = follow("Color", stack);
            o.normal_strength = Some(strength.unwrap_or(1.0));
            o
        }
        "ShaderNodeBump" | "ShaderNodeDisplacement" => {
            let mut o = follow("Height", stack);
            o.bump = true;
            o
        }
        "ShaderNodeSeparateColor" | "ShaderNodeSeparateRGB" | "ShaderNodeSeparateXYZ" => {
            let channel = match out.chars().next() {
                Some('R' | 'X') => 0,
                Some('G' | 'Y') => 1,
                _ => 2,
            };
            let source = match idname.as_str() {
                "ShaderNodeSeparateXYZ" => "Vector",
                "ShaderNodeSeparateRGB" => "Image",
                _ => "Color",
            };
            let mut o = follow(source, stack);
            o.channel = Some(channel);
            o
        }
        "ShaderNodeValue" | "ShaderNodeRGB" => {
            let v = node.list("outputs").first().map(default_values).unwrap_or_default();
            Origin::of(Kind::Constant(v))
        }
        "ShaderNodeGroup" => {
            let Some(tree) = node.deref("id") else {
                return Origin::of(Kind::Unsupported("gruppo di nodi vuoto".into()));
            };
            let Some(group_out) = tree.list("nodes").into_iter().find(|n| n.str("idname") == "NodeGroupOutput") else {
                return Origin::of(Kind::Unsupported(format!("gruppo «{}» senza uscita", id_name(&tree))));
            };
            let Some(s) = input(&group_out, out) else {
                return Origin::of(Kind::Unsupported(format!("gruppo «{}»: uscita {out}", id_name(&tree))));
            };
            stack.push(node.clone());
            let o = resolve(&s, stack, depth);
            stack.pop();
            o
        }
        "NodeGroupInput" => match stack.pop() {
            Some(group) => {
                let o = match input(&group, out) {
                    Some(s) => resolve(&s, stack, depth),
                    None => Origin::of(Kind::Unsupported(format!("ingresso del gruppo {out}"))),
                };
                stack.push(group);
                o
            }
            None => Origin::of(Kind::Unsupported("Group Input fuori da un gruppo".into())),
        },
        "ShaderNodeMix" | "ShaderNodeMixRGB" => {
            // Mix: se uno dei due ingressi è un'immagine si usa quella (approssimato); un
            // ingresso procedurale (sporco, rumore sopra l'immagine) si scarta con un avviso.
            // Solo se non resta niente di utilizzabile il valore è procedurale.
            let names: [String; 2] = if idname == "ShaderNodeMix" {
                let t = if out.ends_with("_Float") {
                    "Float"
                } else if out.ends_with("_Vector") {
                    "Vector"
                } else {
                    "Color"
                };
                [format!("A_{t}"), format!("B_{t}")]
            } else {
                ["Color1".into(), "Color2".into()]
            };
            let a = input(node, &names[0]).map(|s| resolve(&s, stack, depth));
            let b = input(node, &names[1]).map(|s| resolve(&s, stack, depth));
            let both: Vec<Origin> = [a, b].into_iter().flatten().collect();
            let discarded: Vec<String> = both
                .iter()
                .flat_map(|o| {
                    o.procedural_nodes()
                        .map(str::to_string)
                        .into_iter()
                        .chain(o.procedural.iter().cloned())
                })
                .collect();
            let usable = both
                .iter()
                .find(|o| matches!(o.kind, Kind::Image(_)))
                .or_else(|| both.iter().find(|o| o.procedural_nodes().is_none()))
                .cloned();
            let Some(mut pick) = usable else {
                return Origin::of(Kind::Procedural(discarded.join(", ")));
            };
            pick.procedural = discarded;
            pick.approx.push(format!("Mix «{}» (si usa un solo ingresso)", node.str("name")));
            pick
        }
        "ShaderNodeCombineColor" | "ShaderNodeCombineRGB" | "ShaderNodeCombineXYZ" => {
            // Canali ricomposti dalla stessa immagine (tipico: normal DirectX → OpenGL con il
            // verde invertito): si usa l'immagine intera, segnando la conversione.
            let names: [&str; 3] = match idname.as_str() {
                "ShaderNodeCombineXYZ" => ["X", "Y", "Z"],
                "ShaderNodeCombineRGB" => ["R", "G", "B"],
                _ => ["Red", "Green", "Blue"],
            };
            let parts: Vec<Origin> = names
                .iter()
                .filter_map(|n| input(node, n).map(|s| resolve(&s, stack, depth)))
                .collect();
            let image = |o: &Origin<'a>| match &o.kind {
                Kind::Image(i) => Some(id_name(i)),
                _ => None,
            };
            let first = parts.first().and_then(image);
            let same = first.is_some() && parts.len() == 3 && parts.iter().all(|p| image(p) == first);
            if !same {
                return Origin::of(Kind::Unsupported(format!(
                    "{idname} «{}» con canali da sorgenti diverse",
                    node.str("name")
                )));
            }
            let mut o = parts[0].clone();
            o.channel = None;
            o.directx = parts[1].invert || parts[1].approx.iter().any(|a| a.contains("Math"));
            o.approx = vec![format!("canali ricomposti da {idname} «{}»", node.str("name"))];
            o
        }
        id if PROCEDURAL.contains(&id) => Origin::of(Kind::Procedural(format!("{id} «{}»", node.str("name")))),
        id if PASS_THROUGH.contains(&id) => {
            let all: Vec<Origin> = inputs(node)
                .iter()
                .filter(|s| s.deref("link").is_some())
                .map(|s| resolve(s, stack, depth))
                .collect();
            let discarded: Vec<String> = all
                .iter()
                .flat_map(|o| {
                    o.procedural_nodes()
                        .map(str::to_string)
                        .into_iter()
                        .chain(o.procedural.iter().cloned())
                })
                .collect();
            let mut o = all
                .iter()
                .find(|o| matches!(o.kind, Kind::Image(_)))
                .or_else(|| all.iter().find(|o| !matches!(o.kind, Kind::Constant(_) | Kind::Procedural(_))))
                .or_else(|| all.iter().find(|o| o.procedural_nodes().is_some()))
                .cloned()
                .unwrap_or_else(|| Origin::of(Kind::Unsupported(format!("{id} senza ingressi collegati"))));
            if o.procedural_nodes().is_none() {
                o.procedural = discarded;
            }
            o.approx.push(format!("{id} «{}» ignorato", node.str("name")));
            o
        }
        other => Origin::of(Kind::Unsupported(format!("nodo {other} «{}»", node.str("name")))),
    }
}

/// Byte di un'immagine: impacchettata nel file, oppure il file esterno (percorso relativo al
/// `.blend` con `//`, o una cartella `textures` accanto).
fn image_bytes(image: &Inst, blend_dir: &Path) -> Result<(Vec<u8>, String), String> {
    let pf = image
        .deref("packedfile")
        .or_else(|| image.list("packedfiles").first().and_then(|p| p.deref("packedfile")));
    if let Some(pf) = pf {
        let size = pf.i32("size").unwrap_or(0) as usize;
        if let Some(b) = pf.data_of("data", size).filter(|b| b.len() == size && size > 0) {
            return Ok((b.to_vec(), "impacchettata nel file".into()));
        }
    }
    let raw = [image.str("name"), image.str("filepath")]
        .into_iter()
        .find(|s| !s.is_empty())
        .unwrap_or_default();
    if raw.is_empty() {
        return Err("né impacchettata né con un percorso".into());
    }
    let rel = raw.trim_start_matches("//").replace('\\', "/");
    let file_name = Path::new(&rel).file_name().map(PathBuf::from).unwrap_or_default();
    let candidates = [
        blend_dir.join(&rel),
        PathBuf::from(&raw),
        blend_dir.join("textures").join(&file_name),
        blend_dir.join(&file_name),
    ];
    for c in candidates {
        if let Ok(b) = std::fs::read(&c) {
            return Ok((b, c.display().to_string()));
        }
    }
    Err(format!("file esterno non trovato: {raw}"))
}

/// Colori medi di una normal map tangente: rosso e verde intorno a 0,5, blu alto. Se
/// l'immagine non si legge si accetta (non si scarta per un dubbio).
fn looks_like_normal_map(bytes: &[u8]) -> bool {
    let Ok(img) = image::load_from_memory(bytes) else { return true };
    let small = img.thumbnail(64, 64).to_rgb8();
    let n = (small.width() * small.height()).max(1) as f32;
    let mut mean = [0.0f32; 3];
    for p in small.pixels() {
        for (m, c) in mean.iter_mut().zip(p.0) {
            *m += c as f32 / 255.0;
        }
    }
    let [r, g, b] = mean.map(|m| m / n);
    b > 0.65 && (r - 0.5).abs() < 0.2 && (g - 0.5).abs() < 0.2
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

fn fmt_values(v: &[f32]) -> String {
    match v.len() {
        0 => "—".into(),
        1 => format!("{:.3}", v[0]),
        _ => format!("[{}]", v.iter().take(3).map(|x| format!("{x:.3}")).collect::<Vec<_>>().join(", ")),
    }
}

/// Shader collegato al Material Output (anche dentro un gruppo): nodo, pila dei gruppi e il
/// Material Output stesso (per il displacement).
fn shader_of<'a>(tree: &Inst<'a>) -> Option<(Inst<'a>, Vec<Inst<'a>>, Inst<'a>)> {
    let nodes = tree.list("nodes");
    let output = nodes
        .iter()
        .filter(|n| n.str("idname") == "ShaderNodeOutputMaterial")
        // Il Material Output attivo ha il bit 1 (NODE_DO_OUTPUT) nei flag.
        .max_by_key(|n| n.i32("flag").unwrap_or(0) & 2)?
        .clone();
    let surface = input(&output, "Surface")?;
    let mut node = surface.deref("link")?.deref("fromnode")?;
    let mut out = surface
        .deref("link")?
        .deref("fromsock")
        .map(|s| s.str("identifier"))
        .unwrap_or_default();
    let mut stack = Vec::new();
    for _ in 0..8 {
        if node.str("idname") != "ShaderNodeGroup" {
            return Some((node, stack, output));
        }
        let inner = node.deref("id")?;
        let go = inner.list("nodes").into_iter().find(|n| n.str("idname") == "NodeGroupOutput")?;
        let s = input(&go, &out)?;
        let link = s.deref("link")?;
        stack.push(node.clone());
        out = link.deref("fromsock").map(|s| s.str("identifier")).unwrap_or_default();
        node = link.deref("fromnode")?;
    }
    None
}

/// Tutti i materiali del file con uno shader convertibile; gli altri vanno negli errori.
pub fn read_all(path: &Path) -> anyhow::Result<(Vec<Extracted>, Vec<String>)> {
    let blend = Blend::open(path)?;
    let dir = path.parent().unwrap_or(Path::new("."));
    let mut out = Vec::new();
    let mut skipped = Vec::new();
    for m in blend.blocks.iter().filter(|b| &b.code[..2] == b"MA").map(|b| blend.at(b)) {
        let name = id_name(&m);
        let Some(tree) = m.deref("nodetree") else {
            skipped.push(format!("{name}: senza nodi"));
            continue;
        };
        match shader_of(&tree) {
            Some((shader, stack, output)) => match convert(&name, &shader, stack, &output, &blend, dir) {
                Ok(x) => out.push(x),
                Err(e) => skipped.push(format!("{name}: {e:#}")),
            },
            None => skipped.push(format!("{name}: nessuno shader collegato al Material Output")),
        }
    }
    Ok((out, skipped))
}

/// Il primo materiale convertibile del file.
pub fn read(path: &Path) -> anyhow::Result<Extracted> {
    let (mut all, skipped) = read_all(path)?;
    if all.is_empty() {
        anyhow::bail!("nessun materiale convertibile ({})", skipped.join("; "));
    }
    Ok(all.remove(0))
}

fn convert<'a>(
    name: &str,
    shader: &Inst<'a>,
    stack: Vec<Inst<'a>>,
    output: &Inst<'a>,
    blend: &Blend,
    dir: &Path,
) -> anyhow::Result<Extracted> {
    let idname = shader.str("idname");
    let principled = idname == "ShaderNodeBsdfPrincipled";
    anyhow::ensure!(
        principled || idname == "ShaderNodeBsdfMetallic",
        "shader {idname} non supportato (servono Principled BSDF o Metallic BSDF)"
    );
    let mut x = Extracted {
        name: name.to_string(),
        shader: idname.clone(),
        material: Material {
            name: name.to_string(),
            ..Default::default()
        },
        images: Vec::new(),
        params: Vec::new(),
        procedural: Vec::new(),
        unused_images: Vec::new(),
        notes: Vec::new(),
        scale_m: 0.5,
    };
    let mut used_images: Vec<String> = Vec::new();
    let mut repeats: Option<(f32, Option<String>)> = None;
    let get = |socket: &str| -> Option<Origin<'a>> {
        let s = input(shader, socket)?;
        let mut st = stack.clone();
        Some(resolve(&s, &mut st, 0))
    };
    // Ingresso → slot: (nome in Blender, slot, scalare usato senza mappa).
    let mut handle = |label: &'static str, o: Option<Origin<'a>>, slot: Slot, x: &mut Extracted| -> Option<Vec<f32>> {
        let o = o?;
        // Parti procedurali scartate (sporco, rumore sopra l'immagine): avviso, non errore.
        x.procedural.extend(o.procedural.iter().cloned());
        let mut approx_parts = o.approx.clone();
        if !o.procedural.is_empty() {
            approx_parts.push(format!("parte procedurale ignorata: {}", o.procedural.join(", ")));
        }
        let approx = (!approx_parts.is_empty()).then(|| approx_parts.join("; "));
        match &o.kind {
            Kind::Constant(v) => {
                x.params.push(Param {
                    name: label,
                    blender: fmt_values(v),
                    forge: fmt_values(v),
                    status: if approx.is_some() { Status::Approximate } else { Status::Exact },
                    note: approx.unwrap_or_default(),
                });
                Some(v.clone())
            }
            Kind::Procedural(n) => {
                x.procedural.push(n.clone());
                x.params.push(Param {
                    name: label,
                    blender: n.clone(),
                    forge: "—".into(),
                    status: Status::Unsupported,
                    note: "nodo procedurale: non convertibile in una mappa".into(),
                });
                None
            }
            Kind::Unsupported(n) => {
                x.params.push(Param {
                    name: label,
                    blender: n.clone(),
                    forge: "valore predefinito".into(),
                    status: Status::Unsupported,
                    note: "nodo non interpretabile: parametro non convertito".into(),
                });
                None
            }
            Kind::Image(image) => {
                let iname = id_name(image);
                used_images.push(iname.clone());
                // Nello slot normal va solo un'immagine che sia davvero una normal map (es. con
                // un Mix procedurale si potrebbe arrivare alla mappa di rugosità).
                let got = image_bytes(image, dir).and_then(|(b, origin)| {
                    if slot == Slot::Normal && o.channel.is_none() && !looks_like_normal_map(&b) {
                        Err(format!("«{iname}» non è una normal map (colori non azzurri): scartata"))
                    } else {
                        Ok((b, origin))
                    }
                });
                match got {
                    Ok((bytes, origin)) => {
                        let ext = extension(&bytes, &iname);
                        let mut notes = Vec::new();
                        if let Some(c) = o.channel {
                            notes.push(format!("canale {}", ["rosso", "verde", "blu", "alfa"][c as usize]));
                        }
                        if o.invert {
                            notes.push("invertita".into());
                        }
                        if let Some(a) = &approx {
                            notes.push(a.clone());
                        }
                        let inverted_gloss = slot == Slot::Roughness && o.invert;
                        x.params.push(Param {
                            name: label,
                            blender: format!("immagine «{iname}»"),
                            forge: if inverted_gloss {
                                format!("mappa {} (glossiness)", slot.label())
                            } else {
                                format!("mappa {}", slot.label())
                            },
                            status: if approx.is_some() || (o.invert && slot != Slot::Roughness) {
                                Status::Approximate
                            } else {
                                Status::Exact
                            },
                            note: notes.join(", "),
                        });
                        if let Some(r) = o.repeats {
                            if repeats.is_none() || slot == Slot::Color {
                                repeats = Some((r, o.coords.clone()));
                            }
                        }
                        if slot == Slot::Roughness {
                            x.material.roughness_is_gloss = o.invert;
                        }
                        if slot == Slot::Normal {
                            x.material.normal_directx = o.directx;
                            x.material.normal_strength = o.normal_strength.unwrap_or(1.0).clamp(0.0, 4.0);
                        }
                        x.images.push(ExtractedImage {
                            slot,
                            name: iname,
                            bytes,
                            ext,
                            channel: o.channel.filter(|c| *c < 3),
                            origin,
                        });
                    }
                    Err(e) => x.params.push(Param {
                        name: label,
                        blender: format!("immagine «{iname}»"),
                        forge: "—".into(),
                        status: Status::Unsupported,
                        note: e,
                    }),
                }
                Some(Vec::new())
            }
        }
    };
    // Colore base.
    if let Some(v) = handle("Colore base", get("Base Color"), Slot::Color, &mut x) {
        x.material.base_color = if v.len() >= 3 { [v[0], v[1], v[2]] } else { [1.0; 3] };
    }
    // Metallic: il Metallic BSDF è un metallo per definizione.
    if principled {
        if let Some(v) = handle("Metallic", get("Metallic"), Slot::Metallic, &mut x) {
            x.material.metallic = v.first().copied().unwrap_or(1.0);
        }
    } else {
        x.material.metallic = 1.0;
        x.params.push(Param {
            name: "Metallic",
            blender: "Metallic BSDF".into(),
            forge: "1".into(),
            status: Status::Approximate,
            note: "IOR complesso ed Edge Tint del Metallic BSDF non hanno un equivalente: conta il colore base".into(),
        });
    }
    if let Some(v) = handle("Roughness", get("Roughness"), Slot::Roughness, &mut x) {
        x.material.roughness = v.first().copied().unwrap_or(1.0);
    }
    // Normal: Normal Map → mappa normal; Bump → mappa di rilievo.
    if let Some(o) = get("Normal").filter(|o| !matches!(o.kind, Kind::Constant(_))) {
        let slot = if o.bump || o.normal_strength.is_none() {
            Slot::Height
        } else {
            Slot::Normal
        };
        let label = if slot == Slot::Height { "Rilievo (Bump)" } else { "Normal map" };
        handle(label, Some(o), slot, &mut x);
        if slot == Slot::Height && x.images.iter().any(|i| i.slot == Slot::Height) {
            x.material.bump_mm = 0.3;
        }
    }
    // Displacement del Material Output: la mappa si salva nello slot rilievo, spenta (come
    // per i materiali scaricati), così non si perde e si attiva dall'editor se serve.
    if !x.images.iter().any(|i| i.slot == Slot::Height) {
        let disp = input(output, "Displacement").map(|s| resolve(&s, &mut Vec::new(), 0));
        if let Some(o) = disp.filter(|o| matches!(o.kind, Kind::Image(_) | Kind::Procedural(_))) {
            let before = x.params.len();
            handle("Displacement", Some(o), Slot::Height, &mut x);
            // Rilievo spento: la mappa resta disponibile nello slot.
            x.material.bump_mm = 0.0;
            x.material.displacement_mm = 0.0;
            if let Some(p) = x.params.get_mut(before).filter(|p| p.status == Status::Exact) {
                p.status = Status::Approximate;
                p.note = "mappa salvata nello slot rilievo, displacement spento: attivalo nell'editor se serve".into();
            }
        }
    }
    if principled {
        if let Some(v) = get("IOR").and_then(|o| match o.kind {
            Kind::Constant(v) => v.first().copied(),
            _ => None,
        }) {
            x.material.ior = v.clamp(1.0, 3.0);
            x.params.push(Param {
                name: "IOR",
                blender: format!("{v:.3}"),
                forge: format!("{:.3}", x.material.ior),
                status: Status::Exact,
                note: String::new(),
            });
        }
        // Parametri che ForgeRender non usa da una mappa: si segnalano se collegati.
        for (socket, label) in [
            ("Alpha", "Alpha"),
            ("Transmission Weight", "Transmission"),
            ("Emission Color", "Emissione"),
            ("Coat Weight", "Coat"),
            ("Sheen Weight", "Sheen"),
            ("Subsurface Weight", "Subsurface"),
        ] {
            let Some(o) = get(socket) else { continue };
            match o.kind {
                Kind::Constant(v) => {
                    let active = v.iter().take(3).any(|c| *c > 1e-4);
                    match socket {
                        "Transmission Weight" if active => x.material.transmission = v[0].clamp(0.0, 1.0),
                        "Coat Weight" | "Sheen Weight" | "Subsurface Weight" if active => x.params.push(Param {
                            name: label,
                            blender: fmt_values(&v),
                            forge: "—".into(),
                            status: Status::Unsupported,
                            note: "strato non presente in ForgeRender".into(),
                        }),
                        _ => {}
                    }
                }
                Kind::Procedural(n) => x.procedural.push(n),
                _ => x.params.push(Param {
                    name: label,
                    blender: "collegato a un nodo".into(),
                    forge: "—".into(),
                    status: Status::Unsupported,
                    note: "da una mappa non è supportato: ignorato".into(),
                }),
            }
        }
    }
    // Scala: lato della ripetizione dalle ripetizioni del Mapping (stima su 1 m di UV).
    if let Some((r, coords)) = repeats {
        x.scale_m = (1.0 / r).clamp(0.05, 5.0);
        x.params.push(Param {
            name: "Scala texture",
            blender: format!("Mapping Scale {r:.3}{}", coords.map(|c| format!(" ({c})")).unwrap_or_default()),
            forge: format!("{:.2} m per ripetizione (Box)", x.scale_m),
            status: Status::Approximate,
            note: "le UV di Blender non hanno una misura reale: scala stimata, da regolare".into(),
        });
    } else if !x.images.is_empty() {
        x.params.push(Param {
            name: "Scala texture",
            blender: "nessun Mapping".into(),
            forge: format!("{:.2} m per ripetizione (Box)", x.scale_m),
            status: Status::Approximate,
            note: "scala non indicata nel file: valore di partenza, da regolare".into(),
        });
    }
    x.material.mapping = TextureMapping {
        projection: Projection::Box,
        scale: x.scale_m,
        ..TextureMapping::default()
    };
    // Immagini del file che non vanno in nessuno slot: si elencano, non si perdono in silenzio.
    for im in blend.blocks.iter().filter(|b| &b.code[..2] == b"IM").map(|b| blend.at(b)) {
        let n = id_name(&im);
        if n != "Render Result" && n != "Viewer Node" && !used_images.contains(&n) {
            x.unused_images.push(n);
        }
    }
    x.procedural.sort();
    x.procedural.dedup();
    if x.images.is_empty() {
        let nodes = if x.procedural.is_empty() {
            "nessuna immagine".to_string()
        } else {
            x.procedural.join(", ")
        };
        x.notes.push(format!(
            "materiale interamente procedurale o senza immagini ({nodes}): non si importa"
        ));
    } else if !x.procedural.is_empty() {
        x.notes.push(format!(
            "attenzione: parti procedurali non convertite ({}); si usano solo le mappe a immagine",
            x.procedural.join(", ")
        ));
    }
    x.material = x.material.sanitized();
    Ok(x)
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

/// Il file viene da un download BlenderKit (cartella `nome_<identificativo>` o percorso con
/// "blenderkit"): di solito licenza Royalty Free.
pub fn is_blenderkit_download(path: &Path) -> bool {
    let folder = path
        .parent()
        .and_then(|p| p.file_name())
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();
    path.to_string_lossy().to_lowercase().contains("blenderkit")
        || folder
            .rsplit('_')
            .next()
            .is_some_and(|id| id.len() == 36 && id.matches('-').count() == 4)
}

/// Provenienza salvata nel materiale: cartella del file e nome del materiale (con la nota sulla
/// licenza per i download BlenderKit).
pub fn source_of(path: &Path, material: &str) -> String {
    let folder = path
        .parent()
        .and_then(|p| p.file_name())
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();
    if is_blenderkit_download(path) {
        format!("Blender {folder} / {material} (BlenderKit Royalty Free: solo uso personale)")
    } else {
        format!("Blender {folder} / {material}")
    }
}

/// Esito dell'import di un file.
pub enum Outcome {
    Imported(Box<LibraryMaterial>),
    AlreadyPresent(String),
    /// Nessuna immagine da estrarre.
    NoImages(String),
    /// Materiale procedurale: non si importa.
    Procedural(String, Vec<String>),
}

/// Salva nella libreria un materiale estratto (con eventuali modifiche ai valori in
/// `material`): mappe nella cartella del materiale, note, categoria. Non tocca la scena.
pub fn save_to_library(
    x: &Extracted,
    material: &Material,
    name: &str,
    source: &str,
    categories: &[String],
) -> anyhow::Result<LibraryMaterial> {
    anyhow::ensure!(
        x.is_importable(),
        "materiale interamente procedurale o senza immagini: non si importa"
    );
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default();
    let tmp = std::env::temp_dir().join(format!("forge_blend_{}_{stamp}", std::process::id()));
    std::fs::create_dir_all(&tmp)?;
    let result = (|| {
        let mut maps = x.material_with_maps(&tmp)?;
        let mut m = material.clone();
        for slot in [Slot::Color, Slot::Metallic, Slot::Roughness, Slot::Normal, Slot::Height] {
            *slot_mut(&mut m, slot) = slot_mut(&mut maps, slot).take();
        }
        store::save_as(&m, name, source)
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    let saved = result?;
    let mut notes = x.notes.clone();
    for p in x.params.iter().filter(|p| p.status != Status::Exact) {
        notes.push(format!("{}: {} -> {} ({})", p.name, p.blender, p.forge, p.note));
    }
    let saved = store::set_notes(&saved.dir, &notes)?;
    store::set_catalog_info(&saved.dir, categories, &["blender".to_string()])
}

/// Importa nella libreria il primo materiale di un `.blend` così com'è (comando CLI).
/// Un materiale con la stessa provenienza non si importa due volte.
pub fn import(path: &Path) -> anyhow::Result<Outcome> {
    let x = read(path)?;
    let source = source_of(path, &x.name);
    if store::list().iter().any(|m| m.source == source) {
        return Ok(Outcome::AlreadyPresent(source));
    }
    if !x.is_importable() {
        return Ok(Outcome::Procedural(x.name, x.procedural));
    }
    if !x.has_images() {
        return Ok(Outcome::NoImages(x.name));
    }
    let metal = x.material.metallic > 0.5 || x.images.iter().any(|i| i.slot == Slot::Metallic);
    let categories: Vec<String> = if metal { vec!["metal".into()] } else { Vec::new() };
    let saved = save_to_library(&x, &x.material, &x.name, &source, &categories)?;
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

    /// Separate Color: si salva il solo canale, in grigio.
    #[test]
    fn channel_is_extracted_as_gray() {
        let mut rgb = image::RgbImage::new(2, 1);
        rgb.put_pixel(0, 0, image::Rgb([10, 200, 30]));
        rgb.put_pixel(1, 0, image::Rgb([40, 50, 250]));
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(rgb)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let img = ExtractedImage {
            slot: Slot::Roughness,
            name: "orm.png".into(),
            bytes: png.into_inner(),
            ext: "png",
            channel: Some(1),
            origin: "prova".into(),
        };
        let (bytes, ext) = img.encoded().unwrap();
        assert_eq!(ext, "png");
        let g = image::load_from_memory(&bytes).unwrap().to_luma8();
        assert_eq!((g.get_pixel(0, 0).0[0], g.get_pixel(1, 0).0[0]), (200, 50));
    }

    /// Prova su tutti i `.blend` di una cartella (non nel repository: licenze Royalty Free):
    /// `FORGE_BLENDERKIT_DIR=…\materials cargo test -p forge_library -- --ignored blend_real`.
    #[test]
    #[ignore]
    fn blend_real_files() {
        let dir = std::env::var("FORGE_BLENDERKIT_DIR").expect("FORGE_BLENDERKIT_DIR");
        let files = scan(Path::new(&dir));
        assert!(!files.is_empty());
        let (mut ok, mut procedural, mut failed) = (0, 0, Vec::new());
        for f in &files {
            match read_all(f) {
                Ok((all, skipped)) if !all.is_empty() => {
                    for x in &all {
                        if x.is_importable() {
                            ok += 1;
                            assert!(x.has_images(), "{}: nessuna immagine", x.name);
                            if !x.images.iter().any(|i| i.slot == Slot::Color) {
                                eprintln!("{}: colore dal nodo (nessuna mappa colore)", x.name);
                            }
                        } else {
                            procedural += 1;
                        }
                    }
                    if !skipped.is_empty() {
                        eprintln!("{}: saltati {skipped:?}", f.display());
                    }
                }
                Ok((_, skipped)) => failed.push(format!("{}: {skipped:?}", f.display())),
                Err(e) => failed.push(format!("{}: {e:#}", f.display())),
            }
        }
        eprintln!(
            "file {}: importabili {ok}, procedurali {procedural}, falliti {failed:?}",
            files.len()
        );
        assert!(failed.is_empty());
    }

    /// Rilievo nei file di Valerio (`dist/MCP/blender material`): valori dei nodi Displacement,
    /// Bump e Normal Map, metodo di displacement del materiale e sfumatura Box delle immagini.
    /// Solo lettura, per decidere la conversione (`cargo test -- --ignored --nocapture`).
    #[test]
    #[ignore = "legge i file .blend di Valerio"]
    fn survey_relief_values() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dist/MCP/blender material");
        let mut files = Vec::new();
        let mut stack = vec![root];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "blend") {
                    files.push(p);
                }
            }
        }
        files.sort();
        let val = |n: &Inst, s: &str| input(n, s).map(|s| fmt_values(&default_values(&s))).unwrap_or("-".into());
        let linked = |n: &Inst, s: &str| input(n, s).is_some_and(|s| s.deref("link").is_some());
        for path in files {
            let Ok(blend) = Blend::open(&path) else { continue };
            for m in blend.blocks.iter().filter(|b| &b.code[..2] == b"MA").map(|b| blend.at(b)) {
                let Some(tree) = m.deref("nodetree") else { continue };
                let method = m.i32("displacement_method").map(|v| v & 0xFF);
                let mut line = format!("{} | metodo {:?}", id_name(&m), method);
                for n in tree.list("nodes") {
                    match n.str("idname").as_str() {
                        "ShaderNodeDisplacement" => {
                            line += &format!(
                                " | Displacement scale {} mid {} (height collegata {})",
                                val(&n, "Scale"),
                                val(&n, "Midlevel"),
                                linked(&n, "Height")
                            )
                        }
                        "ShaderNodeBump" => {
                            line += &format!(
                                " | Bump strength {} distance {} (normal collegata {})",
                                val(&n, "Strength"),
                                val(&n, "Distance"),
                                linked(&n, "Normal")
                            )
                        }
                        "ShaderNodeNormalMap" => line += &format!(" | NormalMap strength {}", val(&n, "Strength")),
                        "ShaderNodeTexImage" => {
                            if let Some(st) = n.deref("storage") {
                                let proj = st.i32("projection").unwrap_or(-1);
                                if proj != 0 {
                                    line += &format!(" | Image proj {proj} blend {:?}", st.f32("projection_blend"));
                                }
                            }
                        }
                        _ => {}
                    }
                }
                println!("{line}");
            }
        }
    }
}
