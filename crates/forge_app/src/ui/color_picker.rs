//! Selettore colore delle proprietà dei materiali: un solo pannello con le schede RGB, CMYK e
//! Palette (RAL), che si apre cliccando il campione del colore.
//!
//! - **RGB**: quadrato saturazione/valore e barra della tinta (disegnati qui, senza texture), i
//!   valori R, G, B (0–255, sRGB) e il codice esadecimale.
//!
//! Lo stesso pannello si apre da ogni pulsante colore del programma ([`rgb_edit`]: luci,
//! ambiente, foschia, fumo, polvere, stili), al posto di quello di egui.
//! - **CMYK**: ciano, magenta, giallo e nero in percentuale, con la conversione matematica
//!   semplice R = 255 · (1 − C) · (1 − K) (e così via). Non è una conversione colorimetrica per
//!   la stampa (nessun profilo ICC): serve a inserire un colore dato in CMYK.
//! - **Palette**: colori con codice e nome. Di serie c'è "RAL Classic (valori indicativi)"
//!   (`assets/colori/RAL_Classic.json`, preparato da `tools/colori/ral_classic.js`): valori sRGB
//!   approssimati da fonti pubbliche, non quelli ufficiali RAL (venduti da RAL gGmbH come libreria
//!   digitale). In più si leggono i file che l'utente importa nella cartella `Colori` della
//!   libreria ([`palettes_dir`]): `.ase` (Adobe Swatch Exchange, il formato della libreria digitale
//!   RAL, con valori RGB o Lab), `.json` o `.csv`.
//!
//! Tutte le schede cambiano lo stesso valore: il colore lineare della proprietà, usato dal render.
//! Durante la modifica la proprietà cambia subito (l'anteprima e il render la mostrano);
//! **Applica** la conferma, **Annulla** (o Esc) rimette il colore e il codice di prima.
//!
//! Il codice della palette (per esempio "RAL 7016") si ricorda accanto al colore e si mostra
//! vicino al campione; cambiando il colore a mano (RGB, CMYK) il codice si toglie: da quel momento
//! è un colore personalizzato.

use crate::i18n::tr;
use eframe::egui;
use egui::ecolor::Hsva;
use forge_scene::material::{linear_to_srgb, srgb_to_linear};
use std::path::{Path, PathBuf};
use std::sync::Arc;

// ------------------------------------------------------------------------------------------
// Conversioni
// ------------------------------------------------------------------------------------------

/// Colore lineare (0–1) → sRGB a 8 bit.
pub fn to_srgb8(c: [f32; 3]) -> [u8; 3] {
    c.map(|v| (linear_to_srgb(v.clamp(0.0, 1.0)) * 255.0).round() as u8)
}

/// sRGB a 8 bit → colore lineare.
pub fn from_srgb8(c: [u8; 3]) -> [f32; 3] {
    c.map(|v| srgb_to_linear(v as f32 / 255.0))
}

/// Codice esadecimale `#RRGGBB` (sRGB).
pub fn to_hex(c: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}

/// Legge `#RRGGBB`, `RRGGBB` o `#RGB` (spazi ammessi). `None` se non valido.
pub fn parse_hex(s: &str) -> Option<[u8; 3]> {
    let h = s.trim().trim_start_matches('#');
    if !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    match h.len() {
        6 => Some([0, 2, 4].map(|i| u8::from_str_radix(&h[i..i + 2], 16).unwrap())),
        3 => Some([0, 1, 2].map(|i| u8::from_str_radix(&h[i..i + 1], 16).unwrap() * 17)),
        _ => None,
    }
}

/// sRGB a 8 bit → CMYK in percentuale (0–100), conversione matematica semplice.
pub fn rgb_to_cmyk(c: [u8; 3]) -> [f32; 4] {
    let [r, g, b] = c.map(|v| v as f32 / 255.0);
    let k = 1.0 - r.max(g).max(b);
    if k >= 1.0 - 1e-6 {
        return [0.0, 0.0, 0.0, 100.0];
    }
    let f = |x: f32| (1.0 - x - k) / (1.0 - k) * 100.0;
    [f(r), f(g), f(b), k * 100.0]
}

/// CMYK in percentuale → sRGB a 8 bit.
pub fn cmyk_to_rgb(c: [f32; 4]) -> [u8; 3] {
    let [cy, m, y, k] = c.map(|v| (v / 100.0).clamp(0.0, 1.0));
    [cy, m, y].map(|v| (255.0 * (1.0 - v) * (1.0 - k)).round() as u8)
}

/// CIE Lab (D50, come nelle librerie .ase) → sRGB lineare (D65, adattamento di Bradford).
pub fn lab_d50_to_linear_srgb(l: f32, a: f32, b: f32) -> [f32; 3] {
    let fy = (l + 16.0) / 116.0;
    let fx = fy + a / 500.0;
    let fz = fy - b / 200.0;
    let inv = |t: f32| {
        if t > 6.0 / 29.0 {
            t * t * t
        } else {
            3.0 * (6.0f32 / 29.0).powi(2) * (t - 4.0 / 29.0)
        }
    };
    let (x, y, z) = (0.964_22 * inv(fx), inv(fy), 0.825_21 * inv(fz));
    // Bradford D50 → D65.
    let x65 = 0.955_576_6 * x - 0.023_039_3 * y + 0.063_163_6 * z;
    let y65 = -0.028_289_5 * x + 1.009_941_6 * y + 0.021_007_7 * z;
    let z65 = 0.012_298_2 * x - 0.020_483 * y + 1.329_909_8 * z;
    [
        3.240_454_2 * x65 - 1.537_138_5 * y65 - 0.498_531_4 * z65,
        -0.969_266 * x65 + 1.876_010_8 * y65 + 0.041_556 * z65,
        0.055_643_4 * x65 - 0.204_025_9 * y65 + 1.057_225_2 * z65,
    ]
    .map(|v| v.clamp(0.0, 1.0))
}

// ------------------------------------------------------------------------------------------
// Palette (RAL o altre), da file importati dall'utente
// ------------------------------------------------------------------------------------------

/// Un colore di una palette: codice ("RAL 7016"), nome e colore lineare.
#[derive(Clone, Debug, PartialEq)]
pub struct Swatch {
    pub code: String,
    pub name: String,
    /// Nome inglese (mostrato con l'interfaccia in inglese), se c'è.
    pub name_en: String,
    pub color: [f32; 3],
}

impl Swatch {
    /// Nome nella lingua dell'interfaccia.
    pub fn local_name(&self) -> &str {
        if crate::i18n::current_code() == "en" && !self.name_en.is_empty() {
            &self.name_en
        } else {
            &self.name
        }
    }
}

/// Una palette (compresa o letta da un file).
#[derive(Clone, Debug)]
pub struct Palette {
    pub name: String,
    /// Avvertenza della palette (per esempio "valori indicativi").
    pub note: String,
    pub swatches: Vec<Swatch>,
}

/// Palette compresa nel programma: codici RAL Classic con valori sRGB indicativi.
const RAL_CLASSIC: &str = include_str!("../../../../assets/colori/RAL_Classic.json");

/// La palette RAL Classic compresa.
pub fn builtin_ral() -> Palette {
    let v: serde_json::Value = serde_json::from_str(RAL_CLASSIC).expect("RAL_Classic.json");
    Palette {
        name: v["name"].as_str().unwrap_or("RAL Classic").to_string(),
        note: v["note"].as_str().unwrap_or_default().to_string(),
        swatches: parse_json(RAL_CLASSIC).expect("RAL_Classic.json"),
    }
}

/// Cartella delle palette importate: `Colori` nella cartella della libreria.
pub fn palettes_dir() -> PathBuf {
    forge_library::paths::root().join("Colori")
}

/// Separa "RAL 7016 Grigio antracite" in codice e nome (il codice sono le prime due parole se
/// la prima è un prefisso in maiuscolo come RAL, altrimenti la prima).
fn split_code(full: &str) -> (String, String) {
    let words: Vec<&str> = full.split_whitespace().collect();
    let n = if words.len() >= 2 && words[0].chars().all(|c| c.is_ascii_uppercase()) && words[1].chars().any(|c| c.is_ascii_digit()) {
        2
    } else {
        1.min(words.len())
    };
    (words[..n].join(" "), words[n..].join(" "))
}

/// Legge un file Adobe Swatch Exchange (.ase): colori RGB, Lab, CMYK o grigi.
pub fn parse_ase(bytes: &[u8]) -> Result<Vec<Swatch>, String> {
    let rd_u16 = |o: usize| bytes.get(o..o + 2).map(|b| u16::from_be_bytes([b[0], b[1]]));
    let rd_u32 = |o: usize| bytes.get(o..o + 4).map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]));
    let rd_f32 = |o: usize| rd_u32(o).map(f32::from_bits);
    if bytes.get(0..4) != Some(b"ASEF") {
        return Err("non è un file .ase (intestazione ASEF mancante)".into());
    }
    let blocks = rd_u32(8).ok_or("file troppo corto")? as usize;
    let mut o = 12;
    let mut out = Vec::new();
    for _ in 0..blocks {
        let kind = rd_u16(o).ok_or("blocco troncato")?;
        let len = rd_u32(o + 2).ok_or("blocco troncato")? as usize;
        let body = o + 6;
        if kind == 0x0001 {
            let nlen = rd_u16(body).ok_or("nome troncato")? as usize;
            let name: Vec<u16> = (0..nlen).filter_map(|i| rd_u16(body + 2 + 2 * i)).take_while(|&c| c != 0).collect();
            let name = String::from_utf16_lossy(&name);
            let m = body + 2 + 2 * nlen;
            let model = bytes.get(m..m + 4).ok_or("modello troncato")?;
            let v = |k: usize| rd_f32(m + 4 + 4 * k).ok_or("valori troncati");
            let color = match model {
                b"RGB " => [v(0)?, v(1)?, v(2)?].map(|c| srgb_to_linear(c.clamp(0.0, 1.0))),
                b"LAB " => lab_d50_to_linear_srgb(v(0)? * 100.0, v(1)?, v(2)?),
                b"CMYK" => from_srgb8(cmyk_to_rgb([v(0)? * 100.0, v(1)? * 100.0, v(2)? * 100.0, v(3)? * 100.0])),
                b"Gray" => [srgb_to_linear(v(0)?.clamp(0.0, 1.0)); 3],
                _ => {
                    o = body + len;
                    continue;
                }
            };
            let (code, rest) = split_code(&name);
            out.push(Swatch {
                code,
                name: rest,
                name_en: String::new(),
                color,
            });
        }
        o = body + len;
    }
    Ok(out)
}

/// Legge una palette JSON: `[{"code": "RAL 7016", "name": "…", "hex": "#383E42"}]` (oppure
/// `"rgb": [56, 62, 66]` in sRGB 0–255).
pub fn parse_json(text: &str) -> Result<Vec<Swatch>, String> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let list = v
        .as_array()
        .or_else(|| v.get("colors").and_then(|c| c.as_array()))
        .ok_or("serve un elenco di colori")?;
    let mut out = Vec::new();
    for e in list {
        let code = e["code"].as_str().unwrap_or_default().to_string();
        let name = e["name"].as_str().unwrap_or_default().to_string();
        let name_en = e["name_en"].as_str().unwrap_or_default().to_string();
        let rgb = if let Some(h) = e["hex"].as_str() {
            parse_hex(h).ok_or_else(|| format!("{code}: hex non valido"))?
        } else if let Some(a) = e["rgb"].as_array().filter(|a| a.len() == 3) {
            let c = |i: usize| a[i].as_f64().unwrap_or(0.0).clamp(0.0, 255.0) as u8;
            [c(0), c(1), c(2)]
        } else {
            return Err(format!("{code}: manca \"hex\" o \"rgb\""));
        };
        if code.is_empty() && name.is_empty() {
            continue;
        }
        out.push(Swatch {
            code,
            name,
            name_en,
            color: from_srgb8(rgb),
        });
    }
    Ok(out)
}

/// Legge una palette CSV (separatore `;` o `,` o tab): codice, nome, colore (`#RRGGBB` oppure
/// R, G, B in colonne separate). Le righe che non tornano (intestazione) si saltano.
pub fn parse_csv(text: &str) -> Vec<Swatch> {
    let mut out = Vec::new();
    for line in text.lines() {
        let sep = [';', '\t', ','].into_iter().find(|s| line.contains(*s)).unwrap_or(';');
        let f: Vec<&str> = line.split(sep).map(str::trim).collect();
        if f.len() < 3 {
            continue;
        }
        let rgb = parse_hex(f[2]).or_else(|| {
            let n: Vec<u8> = f.get(2..5)?.iter().filter_map(|x| x.parse().ok()).collect();
            (n.len() == 3).then(|| [n[0], n[1], n[2]])
        });
        if let Some(rgb) = rgb {
            out.push(Swatch {
                code: f[0].to_string(),
                name: f[1].to_string(),
                name_en: String::new(),
                color: from_srgb8(rgb),
            });
        }
    }
    out
}

/// Legge un file di palette secondo l'estensione.
pub fn load_palette(path: &Path) -> Result<Palette, String> {
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let swatches = match ext.as_str() {
        "ase" => parse_ase(&std::fs::read(path).map_err(|e| e.to_string())?)?,
        "json" => parse_json(&std::fs::read_to_string(path).map_err(|e| e.to_string())?)?,
        "csv" | "txt" => parse_csv(&std::fs::read_to_string(path).map_err(|e| e.to_string())?),
        _ => return Err(format!("formato non supportato: .{ext}")),
    };
    if swatches.is_empty() {
        return Err("nessun colore nel file".into());
    }
    Ok(Palette {
        name: path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default(),
        note: String::new(),
        swatches,
    })
}

/// La palette compresa e tutte quelle della cartella `Colori` (e gli errori dei file non letti).
pub fn load_palettes() -> (Vec<Palette>, Vec<String>) {
    let mut files: Vec<PathBuf> = std::fs::read_dir(palettes_dir())
        .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.is_file()).collect())
        .unwrap_or_default();
    files.sort();
    let (mut ok, mut err) = (vec![builtin_ral()], Vec::new());
    for f in files {
        if f.file_name().is_some_and(|n| n.to_string_lossy().starts_with("LEGGIMI")) {
            continue;
        }
        match load_palette(&f) {
            Ok(p) => ok.push(p),
            Err(e) => err.push(format!("{}: {e}", f.display())),
        }
    }
    (ok, err)
}

// ------------------------------------------------------------------------------------------
// Pannello
// ------------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Tab {
    #[default]
    Rgb,
    Cmyk,
    Palette,
}

/// Stato del pannello aperto (nella memoria di egui, per campione).
#[derive(Clone)]
struct PickerState {
    original: [f32; 3],
    original_code: Option<String>,
    tab: Tab,
    hsva: Hsva,
    /// Ultimo colore scritto dal pannello (per accorgersi di modifiche fatte altrove).
    last: [f32; 3],
    hex: String,
    cmyk: [f32; 4],
    query: String,
}

/// Palette caricate (una volta, ricaricate a richiesta).
#[derive(Clone, Default)]
struct Palettes {
    list: Arc<Vec<Palette>>,
    errors: Arc<Vec<String>>,
    loaded: bool,
}

fn palettes(ctx: &egui::Context, reload: bool) -> Palettes {
    let id = egui::Id::NULL.with("forge-palette-colori");
    let cur: Palettes = ctx.data(|d| d.get_temp(id)).unwrap_or_default();
    if cur.loaded && !reload {
        return cur;
    }
    let (list, errors) = load_palettes();
    let p = Palettes {
        list: Arc::new(list),
        errors: Arc::new(errors),
        loaded: true,
    };
    ctx.data_mut(|d| d.insert_temp(id, p.clone()));
    p
}

/// Campione colore con il pannello RGB / CMYK / Palette. `code` = codice di palette associato
/// (mostrato accanto al campione). Vero se il colore o il codice sono cambiati.
/// Quadrato saturazione (orizzontale) / valore (verticale) della tinta corrente. Disegnato con
/// triangoli colorati ai vertici, non con una texture: il selettore di egui 0.37 usa una texture
/// creata al volo che nel programma non compariva (restavano solo il cerchietto e il triangolo).
fn sv_square(ui: &mut egui::Ui, hsva: &mut Hsva) -> bool {
    let size = egui::vec2(ui.available_width().max(120.0), 150.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
    let mut changed = false;
    if let Some(p) = resp.interact_pointer_pos() {
        hsva.s = egui::remap_clamp(p.x, rect.left()..=rect.right(), 0.0..=1.0);
        hsva.v = egui::remap_clamp(p.y, rect.bottom()..=rect.top(), 0.0..=1.0);
        changed = true;
    }
    if ui.is_rect_visible(rect) {
        const N: u32 = 16;
        let mut mesh = egui::Mesh::default();
        for yi in 0..=N {
            let v = yi as f32 / N as f32;
            for xi in 0..=N {
                let s = xi as f32 / N as f32;
                let pos = egui::pos2(egui::lerp(rect.left()..=rect.right(), s), egui::lerp(rect.bottom()..=rect.top(), v));
                mesh.colored_vertex(pos, egui::Color32::from(Hsva::new(hsva.h, s, v, 1.0)));
                if xi < N && yi < N {
                    let i = yi * (N + 1) + xi;
                    mesh.add_triangle(i, i + 1, i + N + 1);
                    mesh.add_triangle(i + 1, i + N + 2, i + N + 1);
                }
            }
        }
        ui.painter().add(egui::Shape::mesh(mesh));
        ui.painter()
            .rect_stroke(rect, 0.0, ui.visuals().widgets.noninteractive.bg_stroke, egui::StrokeKind::Inside);
        let c = egui::pos2(
            egui::lerp(rect.left()..=rect.right(), hsva.s),
            egui::lerp(rect.bottom()..=rect.top(), hsva.v),
        );
        let ring = if hsva.v > 0.6 && hsva.s < 0.5 {
            egui::Color32::BLACK
        } else {
            egui::Color32::WHITE
        };
        ui.painter().circle(
            c,
            6.0,
            egui::Color32::from(Hsva::new(hsva.h, hsva.s, hsva.v, 1.0)),
            egui::Stroke::new(1.5, ring),
        );
    }
    changed
}

/// Barra della tinta (0–360°), disegnata come il quadrato senza texture.
fn hue_bar(ui: &mut egui::Ui, hsva: &mut Hsva) -> bool {
    let size = egui::vec2(ui.available_width().max(120.0), 16.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
    let mut changed = false;
    if let Some(p) = resp.interact_pointer_pos() {
        hsva.h = egui::remap_clamp(p.x, rect.left()..=rect.right(), 0.0..=1.0);
        changed = true;
    }
    if ui.is_rect_visible(rect) {
        const N: u32 = 36;
        let mut mesh = egui::Mesh::default();
        for xi in 0..=N {
            let h = xi as f32 / N as f32;
            let x = egui::lerp(rect.left()..=rect.right(), h);
            let col = egui::Color32::from(Hsva::new(h, 1.0, 1.0, 1.0));
            mesh.colored_vertex(egui::pos2(x, rect.top()), col);
            mesh.colored_vertex(egui::pos2(x, rect.bottom()), col);
            if xi < N {
                let i = xi * 2;
                mesh.add_triangle(i, i + 1, i + 2);
                mesh.add_triangle(i + 1, i + 3, i + 2);
            }
        }
        ui.painter().add(egui::Shape::mesh(mesh));
        ui.painter()
            .rect_stroke(rect, 0.0, ui.visuals().widgets.noninteractive.bg_stroke, egui::StrokeKind::Inside);
        let x = egui::lerp(rect.left()..=rect.right(), hsva.h);
        let marker = egui::Rect::from_x_y_ranges(x - 3.0..=x + 3.0, rect.top() - 2.0..=rect.bottom() + 2.0);
        ui.painter()
            .rect_stroke(marker, 2.0, egui::Stroke::new(2.0, egui::Color32::WHITE), egui::StrokeKind::Middle);
    }
    changed
}

/// Pulsante colore (lineare) per luci, ambiente, effetti e stili: apre lo stesso selettore dei
/// materiali (RGB, CMYK, palette) al posto di quello di egui. Il codice della palette scelta non
/// si conserva: resta solo il colore. Vero se il colore è cambiato.
pub fn rgb_edit(ui: &mut egui::Ui, color: &mut [f32; 3]) -> bool {
    rgb_edit_hint(ui, color, "")
}

/// Come [`rgb_edit`], con un suggerimento in più sul campione.
pub fn rgb_edit_hint(ui: &mut egui::Ui, color: &mut [f32; 3], hint: &str) -> bool {
    let salt = ui.next_auto_id();
    let mut code = None;
    color_field_hint(ui, salt, color, &mut code, hint)
}

pub fn color_field(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    color: &mut [f32; 3],
    code: &mut Option<String>,
) -> bool {
    color_field_hint(ui, id_salt, color, code, "")
}

fn color_field_hint(
    ui: &mut egui::Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    color: &mut [f32; 3],
    code: &mut Option<String>,
    hint: &str,
) -> bool {
    let id = ui.make_persistent_id(("selettore-colore", id_salt));
    let mut changed = false;
    let shown = egui::Color32::from_rgb(to_srgb8(*color)[0], to_srgb8(*color)[1], to_srgb8(*color)[2]);
    let resp = ui.horizontal(|ui| {
        let (rect, resp) = ui.allocate_exact_size(
            egui::vec2(ui.spacing().interact_size.x, ui.spacing().interact_size.y),
            egui::Sense::click(),
        );
        let vis = ui.style().interact(&resp);
        ui.painter().rect_filled(rect, 3.0, shown);
        ui.painter().rect_stroke(rect, 3.0, vis.bg_stroke, egui::StrokeKind::Inside);
        if let Some(c) = code.as_deref() {
            let (short, _) = split_code(c);
            ui.label(egui::RichText::new(short).small().strong()).on_hover_text(c);
        }
        resp
    });
    let resp = if hint.is_empty() {
        resp.inner.on_hover_text(tr("Clic: selettore colore (RGB, CMYK, palette RAL)"))
    } else {
        resp.inner
            .on_hover_text(format!("{hint}\n{}", tr("Clic: selettore colore (RGB, CMYK, palette RAL)")))
    };
    let mut state: Option<PickerState> = ui.ctx().data(|d| d.get_temp(id));
    if resp.clicked() && state.is_none() {
        let s8 = to_srgb8(*color);
        state = Some(PickerState {
            original: *color,
            original_code: code.clone(),
            tab: if code.is_some() { Tab::Palette } else { Tab::Rgb },
            hsva: Hsva::from_rgb(*color),
            last: *color,
            hex: to_hex(s8),
            cmyk: rgb_to_cmyk(s8),
            query: String::new(),
        });
    }
    let Some(mut st) = state else { return false };
    // Colore cambiato altrove mentre il pannello è aperto: si riparte da lì.
    if st.last != *color {
        st.hsva = Hsva::from_rgb(*color);
        st.last = *color;
        st.hex = to_hex(to_srgb8(*color));
        st.cmyk = rgb_to_cmyk(to_srgb8(*color));
    }
    let mut close: Option<bool> = None; // Some(true) = applica, Some(false) = annulla
    let area = egui::Area::new(id.with("pannello"))
        .order(egui::Order::Foreground)
        .fixed_pos(resp.rect.left_bottom() + egui::vec2(0.0, 4.0))
        .constrain(true);
    area.show(ui.ctx(), |ui| {
        egui::Frame::popup(ui.style()).show(ui, |ui| {
            ui.set_width(300.0);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut st.tab, Tab::Rgb, "RGB");
                ui.selectable_value(&mut st.tab, Tab::Cmyk, "CMYK");
                ui.selectable_value(&mut st.tab, Tab::Palette, tr("Palette (RAL)"));
            });
            ui.separator();
            // Modifica a mano (RGB, CMYK): il colore torna personalizzato, senza codice.
            let mut manual: Option<[f32; 3]> = None;
            match st.tab {
                Tab::Rgb => {
                    let mut moved = sv_square(ui, &mut st.hsva);
                    moved |= hue_bar(ui, &mut st.hsva);
                    if moved {
                        manual = Some(st.hsva.to_rgb());
                    }
                    let mut s8 = to_srgb8(manual.unwrap_or(*color));
                    let mut edited = false;
                    ui.horizontal(|ui| {
                        for (k, l) in ["R", "G", "B"].iter().enumerate() {
                            edited |= ui.add(egui::DragValue::new(&mut s8[k]).prefix(format!("{l} "))).changed();
                        }
                    });
                    if edited {
                        manual = Some(from_srgb8(s8));
                    }
                    ui.horizontal(|ui| {
                        ui.label("HEX");
                        let r = ui.add(egui::TextEdit::singleline(&mut st.hex).desired_width(90.0));
                        if r.changed() {
                            if let Some(h) = parse_hex(&st.hex) {
                                manual = Some(from_srgb8(h));
                            }
                        }
                        if parse_hex(&st.hex).is_none() {
                            ui.colored_label(crate::ui::style::WARN, tr("codice non valido"));
                        }
                    });
                }
                Tab::Cmyk => {
                    let mut edited = false;
                    egui::Grid::new(id.with("cmyk")).num_columns(2).show(ui, |ui| {
                        for (k, l) in [tr("Ciano"), tr("Magenta"), tr("Giallo"), tr("Nero (K)")].iter().enumerate() {
                            ui.label(*l);
                            edited |= ui.add(egui::Slider::new(&mut st.cmyk[k], 0.0..=100.0).suffix(" %").max_decimals(1)).changed();
                            ui.end_row();
                        }
                    });
                    if edited {
                        manual = Some(from_srgb8(cmyk_to_rgb(st.cmyk)));
                    }
                    ui.label(
                        egui::RichText::new(tr("Conversione matematica semplice in RGB, senza profilo di stampa: per inserire un colore dato in CMYK, non per prevedere la stampa.")).small().weak(),
                    );
                }
                Tab::Palette => {
                    let mut reload = false;
                    let pal = palettes(ui.ctx(), false);
                    ui.horizontal(|ui| {
                        ui.add(egui::TextEdit::singleline(&mut st.query).hint_text(tr("Cerca codice o nome")).desired_width(170.0));
                        if ui.small_button(tr("Importa…")).on_hover_text(tr("Aggiunge un file di palette (.ase, .json, .csv) alla cartella Colori della libreria")).clicked() {
                            if let Some(f) = rfd::FileDialog::new().add_filter(tr("Palette"), &["ase", "json", "csv"]).pick_file() {
                                let dir = palettes_dir();
                                let _ = std::fs::create_dir_all(&dir);
                                if let Some(name) = f.file_name() {
                                    let _ = std::fs::copy(&f, dir.join(name));
                                }
                                reload = true;
                            }
                        }
                        if ui.small_button(tr("Ricarica")).clicked() {
                            reload = true;
                        }
                    });
                    let pal = if reload { palettes(ui.ctx(), true) } else { pal };
                    for e in pal.errors.iter() {
                        ui.colored_label(crate::ui::style::WARN, e);
                    }
                    let q = st.query.trim().to_lowercase();
                    egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
                        for p in pal.list.iter() {
                            ui.label(egui::RichText::new(&p.name).strong());
                            if !p.note.is_empty() {
                                ui.label(
                                    egui::RichText::new(tr(
                                        "Valori RGB indicativi, non ufficiali RAL (media di fonti pubbliche): per i colori esatti usa i campioni RAL o importa la libreria digitale ufficiale RAL (.ase) con «Importa…».",
                                    ))
                                    .small()
                                    .weak(),
                                );
                            }
                            for s in p.swatches.iter().filter(|s| {
                                q.is_empty()
                                    || s.code.to_lowercase().contains(&q)
                                    || s.name.to_lowercase().contains(&q)
                                    || s.name_en.to_lowercase().contains(&q)
                            }) {
                                // Il codice salvato ha il nome italiano (stabile in ogni lingua);
                                // l'elenco mostra il nome nella lingua dell'interfaccia.
                                let full = if s.name.is_empty() { s.code.clone() } else { format!("{} {}", s.code, s.name) };
                                let shown = if s.local_name().is_empty() { s.code.clone() } else { format!("{} {}", s.code, s.local_name()) };
                                let selected = code.as_deref() == Some(full.as_str());
                                let r = ui.horizontal(|ui| {
                                    let c8 = to_srgb8(s.color);
                                    egui::widgets::color_picker::show_color(ui, egui::Color32::from_rgb(c8[0], c8[1], c8[2]), egui::vec2(28.0, 16.0));
                                    ui.selectable_label(selected, &shown)
                                });
                                if r.inner.clicked() {
                                    *color = s.color;
                                    *code = Some(full);
                                    st.hsva = Hsva::from_rgb(s.color);
                                    st.hex = to_hex(to_srgb8(s.color));
                                    st.cmyk = rgb_to_cmyk(to_srgb8(s.color));
                                    changed = true;
                                }
                            }
                        }
                    });
                }
            }
            if let Some(c) = manual {
                *color = c;
                *code = None;
                st.hex = to_hex(to_srgb8(c));
                if st.tab != Tab::Cmyk {
                    st.cmyk = rgb_to_cmyk(to_srgb8(c));
                }
                if st.tab != Tab::Rgb {
                    st.hsva = Hsva::from_rgb(c);
                }
                changed = true;
            }
            ui.separator();
            ui.horizontal(|ui| {
                let (o8, n8) = (to_srgb8(st.original), to_srgb8(*color));
                egui::widgets::color_picker::show_color(ui, egui::Color32::from_rgb(o8[0], o8[1], o8[2]), egui::vec2(40.0, 22.0)).on_hover_text(tr("Prima"));
                egui::widgets::color_picker::show_color(ui, egui::Color32::from_rgb(n8[0], n8[1], n8[2]), egui::vec2(40.0, 22.0)).on_hover_text(tr("Adesso"));
                ui.label(egui::RichText::new(code.clone().unwrap_or_else(|| to_hex(n8))).small());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(tr("Applica")).clicked() {
                        close = Some(true);
                    }
                    if ui.button(tr("Annulla")).clicked() {
                        close = Some(false);
                    }
                });
            });
        });
    });
    if ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
        close = Some(false);
    }
    st.last = *color;
    match close {
        Some(false) => {
            changed |= *color != st.original || *code != st.original_code;
            *color = st.original;
            *code = st.original_code.clone();
            ui.ctx().data_mut(|d| d.remove::<PickerState>(id));
        }
        Some(true) => ui.ctx().data_mut(|d| d.remove::<PickerState>(id)),
        None => ui.ctx().data_mut(|d| {
            d.insert_temp(id, st);
        }),
    }
    changed
}

/// Colore di un materiale (`field`: `base_color`, `volume_color`, `emission`) con il suo codice
/// di palette in [`forge_scene::Material::color_codes`].
pub fn material_color(ui: &mut egui::Ui, m: &mut forge_scene::Material, field: &'static str) -> bool {
    let mut code = m.color_codes.get(field).cloned();
    let id = (m.id.0.clone(), field);
    let color = match field {
        "volume_color" => &mut m.volume_color,
        "emission" => &mut m.emission,
        _ => &mut m.base_color,
    };
    let changed = color_field(ui, id, color, &mut code);
    match code {
        Some(c) => {
            m.color_codes.insert(field.to_string(), c);
        }
        None => {
            m.color_codes.remove(field);
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_parse_and_format() {
        assert_eq!(parse_hex("#383E42"), Some([0x38, 0x3E, 0x42]));
        assert_eq!(parse_hex(" 383e42 "), Some([0x38, 0x3E, 0x42]));
        assert_eq!(parse_hex("#fff"), Some([255, 255, 255]));
        assert_eq!(parse_hex("#12345"), None);
        assert_eq!(parse_hex("#GG0000"), None);
        assert_eq!(to_hex([0x38, 0x3E, 0x42]), "#383E42");
    }

    /// RGB → CMYK → RGB torna uguale (a meno dell'arrotondamento); nero e bianco ai bordi.
    #[test]
    fn cmyk_roundtrip() {
        assert_eq!(rgb_to_cmyk([0, 0, 0]), [0.0, 0.0, 0.0, 100.0]);
        assert_eq!(rgb_to_cmyk([255, 255, 255]), [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(cmyk_to_rgb([100.0, 0.0, 0.0, 0.0]), [0, 255, 255]);
        for c in [[56u8, 62, 66], [200, 30, 10], [12, 200, 140], [255, 128, 0]] {
            let back = cmyk_to_rgb(rgb_to_cmyk(c));
            for k in 0..3 {
                assert!((back[k] as i32 - c[k] as i32).abs() <= 1, "{c:?} → {back:?}");
            }
        }
        // sRGB ↔ lineare.
        assert_eq!(to_srgb8(from_srgb8([56, 62, 66])), [56, 62, 66]);
    }

    /// File .ase con un colore RGB, uno Lab e un gruppo: letti con codice e nome.
    #[test]
    fn ase_palette_is_read() {
        fn entry(name: &str, model: &[u8; 4], vals: &[f32]) -> Vec<u8> {
            let mut b = Vec::new();
            let n: Vec<u16> = name.encode_utf16().chain([0]).collect();
            b.extend((n.len() as u16).to_be_bytes());
            for c in n {
                b.extend(c.to_be_bytes());
            }
            b.extend(model);
            for v in vals {
                b.extend(v.to_be_bytes());
            }
            b.extend(2u16.to_be_bytes());
            let mut blk = vec![0x00, 0x01];
            blk.extend((b.len() as u32).to_be_bytes());
            blk.extend(b);
            blk
        }
        let mut f = b"ASEF".to_vec();
        f.extend([0, 1, 0, 0]);
        f.extend(4u32.to_be_bytes());
        f.extend([0xC0, 0x01, 0, 0, 0, 0]); // inizio gruppo (vuoto)
        f.extend(entry(
            "RAL 7016 Anthracite grey",
            b"RGB ",
            &[56.0 / 255.0, 62.0 / 255.0, 66.0 / 255.0],
        ));
        f.extend(entry("Bianco", b"LAB ", &[1.0, 0.0, 0.0]));
        f.extend([0xC0, 0x02, 0, 0, 0, 0]); // fine gruppo
        let s = parse_ase(&f).unwrap();
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].code, "RAL 7016");
        assert_eq!(s[0].name, "Anthracite grey");
        assert_eq!(to_srgb8(s[0].color), [56, 62, 66]);
        // Lab L = 100 → bianco.
        assert!(to_srgb8(s[1].color).iter().all(|&v| v >= 253), "{:?}", to_srgb8(s[1].color));
        assert!(parse_ase(b"nope").is_err());
    }

    /// La palette RAL compresa: i codici RAL Classic con nome italiano e inglese, senza doppioni,
    /// e qualche valore noto vicino ai campioni (grigio antracite, bianco puro, nero intenso).
    #[test]
    fn builtin_ral_palette() {
        let p = builtin_ral();
        assert!(p.swatches.len() >= 210, "{}", p.swatches.len());
        assert!(p.note.contains("indicativi"));
        let mut codes: Vec<&str> = p.swatches.iter().map(|s| s.code.as_str()).collect();
        codes.sort();
        let n = codes.len();
        codes.dedup();
        assert_eq!(codes.len(), n, "codici doppi");
        assert!(p
            .swatches
            .iter()
            .all(|s| s.code.starts_with("RAL ") && !s.name.is_empty() && !s.name_en.is_empty()));
        let get = |c: &str| to_srgb8(p.swatches.iter().find(|s| s.code == c).unwrap().color);
        let near = |a: [u8; 3], b: [u8; 3]| a.iter().zip(b).all(|(x, y)| (*x as i32 - y as i32).abs() <= 12);
        assert!(near(get("RAL 7016"), [56, 62, 66]), "{:?}", get("RAL 7016"));
        assert!(get("RAL 9010").iter().all(|&v| v > 225), "{:?}", get("RAL 9010"));
        assert!(near(get("RAL 9005"), [10, 10, 10]), "{:?}", get("RAL 9005"));
        assert_eq!(p.swatches.iter().find(|s| s.code == "RAL 7016").unwrap().name, "Grigio antracite");
    }

    #[test]
    fn json_and_csv_palettes() {
        let j = parse_json(r##"[{"code":"RAL 7016","name":"Grigio antracite","hex":"#383E42"},{"code":"X 1","rgb":[255,0,0]}]"##).unwrap();
        assert_eq!(j.len(), 2);
        assert_eq!(to_srgb8(j[1].color), [255, 0, 0]);
        let c = parse_csv("codice;nome;colore\nRAL 9010;Bianco puro;#F1ECE1\nRAL 9005,Nero intenso,10,10,10\n");
        assert_eq!(c.len(), 2);
        assert_eq!(c[1].code, "RAL 9005");
        assert_eq!(to_srgb8(c[1].color), [10, 10, 10]);
    }

    /// Il codice si toglie quando il colore cambia a mano: lo fa [`color_field`] (qui si prova la
    /// regola sul materiale: un codice salvato resta finché c'è, e il progetto lo rilegge).
    #[test]
    fn material_code_roundtrip() {
        let mut m = forge_scene::Material::default();
        m.color_codes.insert("base_color".into(), "RAL 7016 Grigio antracite".into());
        let s = serde_json::to_string(&m).unwrap();
        let back: forge_scene::Material = serde_json::from_str(&s).unwrap();
        assert_eq!(
            back.color_codes.get("base_color").map(String::as_str),
            Some("RAL 7016 Grigio antracite")
        );
        // Senza codici il campo non si scrive (i progetti di prima restano uguali).
        let plain = serde_json::to_string(&forge_scene::Material::default()).unwrap();
        assert!(!plain.contains("color_codes"));
        assert_eq!(
            split_code("RAL 7016 Grigio antracite"),
            ("RAL 7016".into(), "Grigio antracite".into())
        );
    }
}
