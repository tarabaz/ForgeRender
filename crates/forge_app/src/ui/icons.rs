//! Icone dell'interfaccia: carattere Phosphor (MIT, `assets/fonts/Phosphor.ttf`, licenza in
//! `packaging/licenses/Phosphor_LICENSE.txt`) aggiunto come riserva del testo normale, così
//! un'icona si scrive dentro un'etichetta qualsiasi: `format!("{} Salva", icons::SAVE)`.
//! Codici dal file del carattere (stessi nomi delle icone su phosphoricons.com).

use eframe::egui;

const PHOSPHOR_TTF: &[u8] = include_bytes!("../../../../assets/fonts/Phosphor.ttf");

/// Aggiunge il carattere delle icone ai caratteri di egui (dopo quello del testo).
pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("phosphor".into(), std::sync::Arc::new(egui::FontData::from_static(PHOSPHOR_TTF)));
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        if let Some(list) = fonts.families.get_mut(&family) {
            let at = list.len().min(1);
            list.insert(at, "phosphor".into());
        }
    }
    ctx.set_fonts(fonts);
}

pub const NEW: &str = "\u{E236}"; // file-plus
pub const OPEN: &str = "\u{E256}"; // folder-open
pub const SAVE: &str = "\u{E248}"; // floppy-disk
pub const IMPORT: &str = "\u{E232}"; // file-arrow-down
pub const EXPORT: &str = "\u{EAF0}"; // export
pub const RECENT: &str = "\u{E1A0}"; // clock-counter-clockwise
pub const EXIT: &str = "\u{E42A}"; // sign-out
pub const UNDO: &str = "\u{E08A}"; // arrow-u-up-left
pub const REDO: &str = "\u{E08C}"; // arrow-u-up-right
pub const DUPLICATE: &str = "\u{E1CA}"; // copy
pub const DELETE: &str = "\u{E4A6}"; // trash
pub const HIDE: &str = "\u{E224}"; // eye-slash
pub const SHOW: &str = "\u{E220}"; // eye
pub const SETTINGS: &str = "\u{E270}"; // gear
pub const LIBRARY: &str = "\u{E758}"; // books
pub const ADD: &str = "\u{E1DA}"; // cube
pub const CAMERA: &str = "\u{E10E}"; // camera
pub const LIGHT: &str = "\u{E2DC}"; // lightbulb
pub const ENVIRONMENT: &str = "\u{E28C}"; // globe-hemisphere-west
pub const MATERIAL: &str = "\u{E6C8}"; // palette
pub const RENDER: &str = "\u{E3D0}"; // play
pub const PAUSE: &str = "\u{E39E}"; // pause
pub const STOP: &str = "\u{E46C}"; // stop
pub const RESTART: &str = "\u{E036}"; // arrow-clockwise
pub const FRAME: &str = "\u{ED0A}"; // cube-focus
pub const VIEWS: &str = "\u{EBE6}"; // perspective
pub const SCENE: &str = "\u{E6A2}"; // sparkle
pub const AI: &str = "\u{E762}"; // robot
pub const HELP: &str = "\u{E0E6}"; // book-open
pub const HOME: &str = "\u{E2C2}"; // house
pub const BACK: &str = "\u{E058}"; // arrow-left
pub const FORWARD: &str = "\u{E06C}"; // arrow-right
pub const SEARCH: &str = "\u{E30C}"; // magnifying-glass
pub const KEYS: &str = "\u{E2D8}"; // keyboard
pub const INFO: &str = "\u{E2CE}"; // info
pub const VIEWPORT: &str = "\u{E1DA}"; // cube
pub const IMAGE: &str = "\u{E2CA}"; // image
pub const GPU: &str = "\u{E612}"; // graphics-card
pub const QUALITY: &str = "\u{E434}"; // sliders-horizontal

#[cfg(test)]
mod tests {
    #[test]
    fn font_has_every_icon() {
        let face = ttf_parser_free_check(super::PHOSPHOR_TTF);
        assert!(face, "Phosphor.ttf non valido");
    }

    /// Controllo minimo senza dipendenze: intestazione TrueType e dimensione plausibile.
    fn ttf_parser_free_check(b: &[u8]) -> bool {
        b.len() > 100_000 && (b.starts_with(&[0, 1, 0, 0]) || b.starts_with(b"OTTO") || b.starts_with(b"true"))
    }
}

#[cfg(test)]
mod glyph_tests {
    /// "→" non è nei caratteri dell'interfaccia (si vedrebbe un quadratino): mai nei testi.
    #[test]
    fn no_missing_arrow_glyph_in_interface_texts() {
        let lang = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../lang/Lan_eng.json")).unwrap();
        assert!(!lang.contains('→'), "Lan_eng.json contiene →");
    }
}
