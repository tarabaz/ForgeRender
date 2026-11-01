//! Elenco degli strumenti MCP di ForgeRender (nome, descrizione, schema degli argomenti).
//! Lo usano sia il ponte `forgerender-mcp` (per rispondere a `tools/list` anche quando la
//! finestra non è ancora collegata) sia ForgeRender, che li esegue.

use serde_json::{json, Value};

/// Lati in cui mettere una luce, rispetto all'inquadratura attuale.
pub const SIDES: [&str; 11] = [
    "destra",
    "sinistra",
    "alto",
    "davanti",
    "dietro",
    "davanti_destra",
    "davanti_sinistra",
    "dietro_destra",
    "dietro_sinistra",
    "controluce",
    "basso",
];

/// Definizioni degli strumenti (nome, descrizione, schema JSON degli argomenti).
pub fn list() -> Value {
    let targets = json!({
        "description": "Componenti: nome o id (anche parte del nome), una lista di nomi, oppure \"prodotto\" (predefinito: tutto tranne pavimento/fondale), \"selezione\", \"tutto\".",
        "anyOf": [{"type": "string"}, {"type": "array", "items": {"type": "string"}}]
    });
    let light_ref =
        json!({"description": "Luce: indice (da 0) o nome, come in scene_info.", "anyOf": [{"type": "integer"}, {"type": "string"}]});
    let color = json!({"description": "Colore \"#rrggbb\" oppure [r, g, b] da 0 a 1.", "anyOf": [{"type": "string"}, {"type": "array", "items": {"type": "number"}}]});
    let vec3 = json!({"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3});
    let sides = json!({"type": "string", "enum": SIDES, "description": "Lato rispetto all'inquadratura attuale."});
    let tools = vec![
        tool("inspect_components", "Immagini veloci dei pezzi del prodotto (dalla viewport, non un render) per capire che cosa sono, forma e funzione, prima di scegliere i materiali. Riquadri numerati: a sinistra il pezzo da solo, a destra l'assieme con il pezzo in arancione. I pezzi identici (viti, ruote) compaiono una volta sola con l'elenco delle copie. Per ogni riquadro: nome, id da usare in apply_material, copie, misure in mm, materiali e colori attuali. Non modifica la scena. Con molti pezzi chiedi le pagine successive.",
            json!({
                "targets": targets.clone(),
                "page": {"type": "integer", "description": "Pagina (da 1)."},
                "per_page": {"type": "integer", "description": "Pezzi per immagine (1–12, predefinito 12)."},
                "view": {"type": "string", "enum": ["tre_quarti", "alto", "basso", "fronte", "retro", "lato"], "description": "Da dove si guardano i pezzi (predefinito tre_quarti)."}
            }), &[]),
        tool("scene_info", "Stato della scena: prodotto (centro e misure in metri), componenti con materiale, luci (indice, tipo, posizione, colore, intensità), camera, ambiente, impostazioni e stato del render. Chiamalo prima di modificare.",
            json!({"max_components": {"type": "integer", "description": "Massimo di componenti elencati (predefinito 80)."}}), &[]),
        tool("new_scene", "Nuova scena: \"studio\" (studio fotografico dimostrativo, sostituito dal primo modello importato) o \"vuota\". Le modifiche non salvate si perdono.",
            json!({"kind": {"type": "string", "enum": ["studio", "vuota"]}}), &["kind"]),
        tool("open_project", "Apre un progetto .frproject.", json!({"path": {"type": "string", "description": "Percorso assoluto del file."}}), &["path"]),
        tool("save_project", "Salva il progetto. Senza path salva sul file già aperto.",
            json!({"path": {"type": "string", "description": "Percorso assoluto .frproject."}, "overwrite": {"type": "boolean", "description": "Sovrascrive un file esistente diverso da quello aperto (solo se l'utente lo chiede)."}}), &[]),
        tool("import_model", "Importa un modello 3D (glTF/GLB, OBJ, STL, STEP, Blender .blend, SolidWorks su Windows). Il primo import sostituisce la scena dimostrativa.",
            json!({
                "path": {"type": "string", "description": "Percorso assoluto del file."},
                "unit": {"type": "string", "enum": ["mm", "cm", "m", "in"], "description": "Unità di STL/OBJ (predefinito mm)."},
                "up_axis": {"type": "string", "enum": ["auto", "y", "z"]},
                "drop_to_floor": {"type": "boolean", "description": "Appoggia a terra dopo l'import (predefinito sì)."}
            }), &["path"]),
        tool("prepare_studio", "Prepara la scena prodotto in un colpo: appoggia il prodotto sulla base, mette il fondale, applica l'aspetto (luci, ambiente, colore del fondale) e la qualità, inquadra. Annullabile con undo_preset.",
            json!({
                "look": {"type": "string", "enum": ["studio", "chiaro", "scuro", "scenografico"], "description": "studio = fondale in tela inclinato + 4 luci da studio; chiaro = fondale bianco e luce morbida (e-commerce); scuro = fondale nero lucido, tre punti luce; scenografico = luce calda di taglio e controluce fredda."},
                "quality": {"type": "string", "enum": ["bozza", "bilanciata", "alta"]}
            }), &[]),
        tool("fit_scene_to_product", "Dopo aver spostato o scalato il prodotto: rifà fondale e luci sulle sue misure (tiene materiale del fondale e colori delle luci) e inquadra.", json!({}), &[]),
        tool("undo_preset", "Annulla l'ultimo prepare_studio / fit_scene_to_product (luci, ambiente, qualità, fondale e posizione come prima).", json!({}), &[]),
        tool("add_object", "Aggiunge un oggetto vicino al prodotto: pavimento, fondale curvo, fondale fotografico (tela inclinata), parete o forme semplici.",
            json!({"kind": {"type": "string", "enum": ["pavimento", "fondale", "fondale_fotografico", "parete", "cubo", "sfera", "cilindro", "tazza", "prisma", "diamante", "pendente"]}}), &["kind"]),
        tool("transform_objects", "Sposta, ruota o scala componenti (predefinito: il prodotto). Annullabile con Ctrl+Z nella finestra.",
            json!({
                "targets": targets,
                "move_m": {"description": "Spostamento [x, y, z] in metri (x destra del mondo, y alto, z verso la vista frontale).", "type": "array", "items": {"type": "number"}},
                "rotate_deg": {"description": "Rotazione aggiunta [x, y, z] in gradi (y = giro sul piano).", "type": "array", "items": {"type": "number"}},
                "scale": {"type": "number", "description": "Fattore di scala uniforme (2 = doppio)."},
                "drop_to_floor": {"type": "boolean", "description": "Appoggia dopo la trasformazione: scende fino a toccare l'oggetto subito sotto (pezzo, fondale o pavimento), quota 0 se sotto non c'è niente."}
            }), &[]),
        tool("set_visibility", "Mostra o nasconde componenti.", json!({"targets": targets, "visible": {"type": "boolean"}}), &["targets", "visible"]),
        tool("add_light", "Aggiunge una luce puntata sul prodotto. Esempi: \"luce a destra\" → side destra; \"controluce\" → side controluce. Intensità: usa strength (1 = normale, 2 = doppia) se non conosci le unità.",
            json!({
                "type": {"type": "string", "enum": ["area", "punto", "spot", "direzionale"], "description": "area = softbox (predefinita), punto = lampadina, spot = faretto a cono, direzionale = sole."},
                "side": sides,
                "position": vec3.clone(),
                "target": vec3.clone(),
                "strength": {"type": "number", "description": "Moltiplicatore dell'intensità tipica (predefinito 1)."},
                "intensity": {"type": "number", "description": "Valore assoluto: radianza per area (tipico 2–10), W/sr per punto/spot, W/m² per direzionale."},
                "kelvin": {"type": "number", "description": "Temperatura colore (3200 calda, 5500 neutra, 7500 fredda)."},
                "color": color.clone(),
                "size_m": {"type": "number", "description": "Lato della softbox (area) in metri."},
                "cone_deg": {"type": "number", "description": "Apertura dello spot in gradi."},
                "range_m": {"type": "number", "description": "Punto/spot: distanza massima raggiunta dalla luce (0 = illimitata)."},
                "range_softness": {"type": "number", "description": "Sfumatura del bordo del raggio, 0–1."},
                "name": {"type": "string"},
                "shadows": {"type": "boolean"}
            }), &[]),
        tool("edit_light", "Modifica una luce esistente: intensità, colore, posizione (anche per lato), dimensioni, cono, raggio d'azione, accesa/spenta.",
            json!({
                "light": light_ref.clone(),
                "strength": {"type": "number", "description": "Moltiplica l'intensità attuale (0.5 = metà)."},
                "intensity": {"type": "number"},
                "kelvin": {"type": "number"},
                "color": color.clone(),
                "side": sides,
                "position": vec3.clone(),
                "target": vec3,
                "aim_at_product": {"type": "boolean"},
                "size_m": {"description": "Area: lato o [larghezza, altezza] in metri.", "anyOf": [{"type": "number"}, {"type": "array", "items": {"type": "number"}}]},
                "cone_deg": {"type": "number"},
                "cone_softness_pct": {"type": "number"},
                "range_m": {"type": "number"},
                "range_softness": {"type": "number"},
                "beam_density": {"type": "number", "description": "Spot: fascio di luce visibile nell'aria (foschia nel cono, 1/m): 0 spento, 0.3 leggero, 1 marcato, 3 denso."},
                "beam_length_m": {"type": "number", "description": "Spot: lunghezza del fascio visibile in metri."},
                "enabled": {"type": "boolean"},
                "shadows": {"type": "boolean"},
                "name": {"type": "string"}
            }), &["light"]),
        tool("remove_light", "Elimina una luce (o tutte con light = \"tutte\").", json!({"light": light_ref}), &["light"]),
        tool("set_all_lights_color", "Colore di tutte le luci da una temperatura in Kelvin.", json!({"kelvin": {"type": "number"}}), &["kelvin"]),
        tool("list_materials", "Cerca materiali: quelli pronti del progetto (~120: metalli, plastiche, vetri, legni…) e la libreria su disco (texture PBR scaricate).",
            json!({
                "search": {"type": "string", "description": "Parole da cercare nel nome o nella categoria (italiano per i pronti, inglese per la libreria scaricata)."},
                "source": {"type": "string", "enum": ["tutti", "progetto", "libreria"]},
                "limit": {"type": "integer"}
            }), &[]),
        tool("apply_material", "Applica un materiale esistente (id o nome, anche parziale) ai componenti (predefinito: tutto il prodotto).",
            json!({"material": {"type": "string"}, "targets": targets.clone()}), &["material"]),
        tool("create_material", "Crea (o aggiorna, stesso nome) un materiale con parametri PBR e lo applica. Esempi: metallo satinato = metallic 1, roughness 0.35; plastica lucida = metallic 0, roughness 0.15; vetro = transmission 1, roughness 0.",
            json!({
                "name": {"type": "string"},
                "base": {"type": "string", "description": "Materiale di partenza da copiare (texture comprese)."},
                "color": color.clone(),
                "metallic": {"type": "number"},
                "roughness": {"type": "number"},
                "transmission": {"type": "number"},
                "ior": {"type": "number"},
                "abbe": {"type": "number", "description": "Dispersione dei trasparenti: numero di Abbe (crown 64, flint 36, diamante 55 con ior 2.42; 0 = nessuna). Visibile solo con il render spettrale."},
                "specular": {"type": "number"},
                "emission_color": color,
                "emission_strength": {"type": "number"},
                "targets": targets
            }), &["name"]),
        tool("set_camera", "Inquadratura del render: vista predefinita o angoli, focale, ortografica, zoom, profondità di campo.",
            json!({
                "view": {"type": "string", "enum": ["frontale", "retro", "sinistra", "destra", "alto", "tre_quarti", "isometrica"]},
                "yaw_deg": {"type": "number", "description": "Giro attorno al prodotto (0 = frontale, 90 = da destra)."},
                "pitch_deg": {"type": "number", "description": "Elevazione (0 = altezza prodotto, 90 = dall'alto)."},
                "focal_mm": {"type": "number", "description": "Focale equivalente full frame (50 normale, 85–100 ritratto prodotto)."},
                "orthographic": {"type": "boolean"},
                "frame": {"type": "string", "enum": ["prodotto", "tutto", "nessuno"], "description": "Inquadra dopo aver girato (predefinito prodotto)."},
                "zoom": {"type": "number", "description": "Fattore di distanza dopo l'inquadratura (>1 più lontano, <1 più vicino)."},
                "depth_of_field": {"type": "boolean"},
                "f_stop": {"type": "number"}
            }), &[]),
        tool("set_environment", "Ambiente (luce e sfondo intorno alla scena): uniforme, gradiente, HDRI o nero.",
            json!({
                "type": {"type": "string", "enum": ["uniforme", "gradiente", "hdri", "nero"]},
                "color": color,
                "intensity": {"type": "number"},
                "hdri_path": {"type": "string"},
                "rotation_deg": {"type": "number"},
                "visible": {"type": "boolean", "description": "Ambiente visibile come sfondo nell'immagine."}
            }), &[]),
        tool("set_render_settings", "Formato e qualità del render.",
            json!({
                "format": {"type": "string", "enum": ["16:9", "1:1", "4:5", "9:16"]},
                "width": {"type": "integer"},
                "height": {"type": "integer"},
                "samples": {"type": "integer", "description": "Campioni per pixel (64 bozza, 256 buona, 1024 finale)."},
                "quality": {"type": "string", "enum": ["bozza", "bilanciata", "alta"]},
                "device": {"type": "string", "enum": ["cpu", "gpu"]},
                "color": {"type": "string", "enum": ["rgb", "spettrale"], "description": "Trasporto della luce: rgb (predefinito, veloce) o spettrale (lunghezze d'onda: dispersione nei vetri con numero di Abbe, colori più fedeli; più lento, servono più campioni)."},
                "denoise": {"type": "boolean", "description": "Riduzione del rumore sull'immagine finale (consigliata sempre)."},
                "denoise_preview": {"type": "boolean", "description": "Ripulisce anche l'anteprima durante il render (a 8, 16, 32… campioni), non solo l'immagine finale."},
                "exposure_ev": {"type": "number"}
            }), &[]),
        tool("render", "Avvia il render, salva il PNG alla fine e restituisce l'anteprima da guardare. Aspetta al massimo max_wait_s: se il render è più lungo risponde \"in corso\" e la finestra lo finisce e lo salva da sola; poi chiama render_status.",
            json!({
                "samples": {"type": "integer", "description": "Campioni per pixel (predefinito quelli impostati; per una prova veloce 32–64)."},
                "output": {"type": "string", "description": "Percorso assoluto del PNG (predefinito: cartella renders)."},
                "overwrite": {"type": "boolean"},
                "max_wait_s": {"type": "number", "description": "Attesa massima della risposta (predefinito 45, massimo 600)."},
                "preview_px": {"type": "integer", "description": "Lato massimo dell'anteprima restituita (predefinito 768, 0 = nessuna)."}
            }), &[]),
        tool("render_status", "Stato dell'ultimo render avviato con render: avanzamento, oppure a render finito file salvato e anteprima. Può aspettare un po' prima di rispondere.",
            json!({
                "max_wait_s": {"type": "number", "description": "Attesa massima della fine (predefinito 45, 0 = risposta subito)."},
                "preview_px": {"type": "integer", "description": "Lato massimo dell'anteprima (predefinito 768)."}
            }), &[]),
        tool("stop_render", "Ferma il render in corso (l'immagine resta).", json!({}), &[]),
        tool("save_image", "Salva l'ultima immagine renderizzata. Il formato segue l'estensione: .png (predefinito), .jpg (leggero, per web), .tif (16 bit, per il fotoritocco), .exr (valori lineari, per il compositing).",
            json!({"path": {"type": "string"}, "overwrite": {"type": "boolean"}}), &["path"]),
    ];
    Value::Array(tools)
}

fn tool(name: &str, description: &str, props: Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": {"type": "object", "properties": props, "required": required},
    })
}
