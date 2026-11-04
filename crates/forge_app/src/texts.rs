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
    // forge_scene::LaserSource
    tk("Rosso (650 nm)"),
    tk("Verde (532 nm)"),
    tk("Blu (450 nm)"),
    tk("Viola UV (405 nm)"),
    tk("Bianco da laboratorio"),
    // forge_scene::light::PRESETS, FOG_PRESETS, FOG_QUALITY
    tk("Studio neutro"),
    tk("Studio fotografico (4 luci)"),
    tk("Tre punti luce"),
    tk("Softbox"),
    tk("Illuminazione industriale"),
    tk("Cinematografica"),
    tk("Misurino che trabocca"),
    tk("Emettitore dentro il contenitore, 1 cm sotto il bordo: cupola a cavolfiore, colata lungo la parete, tappeto sul tavolo."),
    tk("Beuta: nuvola che sale"),
    tk("Emettitore all'imboccatura di un collo stretto: la nebbia esce a sbuffi e sale in una nuvola, qualche ricciolo ricade."),
    tk("Cocktail: colata e riccioli"),
    tk("Emettitore nel bicchiere poco sotto il bordo: poca nebbia che trabocca a filo, scende lungo lo stelo e si arriccia."),
    tk("Calderone / pozione"),
    tk("Emettitore nel calderone o in una ciotola grande: nebbia densa che ribolle sopra il bordo e scivola a terra."),
    tk("Tappeto basso da palco"),
    tk("Emettitore a 20–40 cm da terra (la bocca della macchina): tappeto basso e compatto che copre il pavimento."),
    tk("Molto alta"),
    tk("Ultra"),
    // forge_scene::SMOKE_PRESETS
    tk("Fumo caldo simulato"),
    tk("Simulazione fisica: fumo caldo che esce dal fondo del riquadro, sale, si allarga e si arriccia da solo"),
    tk("Gas pesante simulato"),
    tk("Simulazione fisica di un gas freddo e pesante che esce, scende e si stende sul piano, scorrendo attorno agli oggetti"),
    tk("Nube sospesa"),
    tk("Un ammasso di fumo morbido a mezz'aria, con sbuffi e bordi sfilacciati"),
    tk("Incenso"),
    tk("Un filo sottile che sale liscio e poi si rompe in riccioli: il fondo del riquadro va sulla punta del bastoncino"),
    tk("Vapore da tazza"),
    tk("Simulazione fisica: vapore bianco e leggero che sale piano dalla tazza in filamenti che si arricciano e svaniscono: il fondo del riquadro va sulla superficie del caffè, la sorgente è larga come la tazza"),
    tk("Pennacchio da tubo"),
    tk("Fumo che esce da un tubo, un ugello o un camino, sale allargandosi e si apre in sbuffi: il fondo del riquadro va sulla bocca"),
    tk("Fumo denso industriale"),
    tk("Fumo scuro e pesante che si gonfia in grossi sbuffi: ciminiere, incendi, scene drammatiche"),
    tk("Banco di fumo"),
    tk("Fumo che riempie tutto il riquadro con i bordi sfilacciati: stanza piena di fumo, palco, strato sotto il soffitto"),
];
