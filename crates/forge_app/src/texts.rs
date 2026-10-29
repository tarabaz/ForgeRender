//! Testi dell'interfaccia che arrivano dalle altre librerie (`label()` dei tipi di luce,
//! tone mapping, stati del render…). Sono qui solo perché il test delle lingue li trovi:
//! la traduzione si fa dove vengono mostrati, con `tr(x.label())`.

use crate::i18n::tk;

#[allow(dead_code)] // letta dal test delle lingue, non dal programma
pub const EXTERNAL: &[&str] = &[
    // forge_scene::LightKind
    tk("Puntiforme"),
    tk("Spot"),
    tk("Direzionale"),
    tk("Area (rettangolare)"),
    tk("Portale (finestra)"),
    // forge_scene::AddedKind
    tk("Pavimento"),
    tk("Fondale da studio"),
    tk("Fondale fotografico"),
    tk("Parete"),
    tk("Cubo"),
    tk("Sfera"),
    tk("Cilindro"),
    tk("Tazza"),
    // forge_scene::LengthUnit
    tk("pollici"),
    // forge_import::UpAxis
    tk("Automatico"),
    tk("Y verso l'alto"),
    tk("Z verso l'alto"),
    // forge_render_core: ToneMapping, BackendKind, DenoiseMode, DenoiseQuality
    tk("Standard (clamp)"),
    tk("ACES filmico"),
    tk("CPU (tutti i core)"),
    tk("GPU (compute shader)"),
    tk("Disattivata"),
    tk("Solo immagine finale"),
    tk("Anteprima veloce + finale"),
    tk("Veloce"),
    tk("Bilanciata"),
    tk("Alta"),
    // forge_render_core::SessionState
    tk("In attesa"),
    tk("Rendering in corso"),
    tk("In pausa"),
    tk("Completato"),
    tk("Interrotto"),
];
