//! Strumenti MCP: definizioni (schema degli argomenti) ed esecuzione sulla scena della
//! finestra. Ogni strumento riusa le funzioni dell'interfaccia (preset, import, render…),
//! quindi fa esattamente quello che farebbe l'utente con i pulsanti.

use super::ops::{self, Args, FoundMaterial};
use super::rpc::ToolOutput;
use crate::app::{AddKind, ForgeApp};
use crate::presets::{self, Look, Quality};
use forge_math::Vec3;
use forge_render_core::{BackendKind, DenoiseMode, SessionState};
use forge_scene::{Environment, Light, LightKind, Material};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Esito di uno strumento: subito, oppure da completare nei fotogrammi successivi
/// (import, apertura di un progetto, render).
pub enum Step {
    Done(ToolOutput),
    Wait(Waiter),
}

/// Controllo ripetuto a ogni fotogramma: `Some` quando l'operazione è finita.
pub type Poll = Box<dyn FnMut(&mut ForgeApp) -> Option<ToolOutput>>;

pub struct Waiter {
    pub deadline: Instant,
    pub poll: Poll,
}

type R = Result<Step, String>;

fn done(v: Value) -> R {
    Ok(Step::Done(ToolOutput::ok(v)))
}

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
    let sides = json!({"type": "string", "enum": ops::SIDES, "description": "Lato rispetto all'inquadratura attuale."});
    let tools = vec![
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
            json!({"kind": {"type": "string", "enum": ["pavimento", "fondale", "fondale_fotografico", "parete", "cubo", "sfera", "cilindro", "tazza"]}}), &["kind"]),
        tool("transform_objects", "Sposta, ruota o scala componenti (predefinito: il prodotto). Annullabile con Ctrl+Z nella finestra.",
            json!({
                "targets": targets,
                "move_m": {"description": "Spostamento [x, y, z] in metri (x destra del mondo, y alto, z verso la vista frontale).", "type": "array", "items": {"type": "number"}},
                "rotate_deg": {"description": "Rotazione aggiunta [x, y, z] in gradi (y = giro sul piano).", "type": "array", "items": {"type": "number"}},
                "scale": {"type": "number", "description": "Fattore di scala uniforme (2 = doppio)."},
                "drop_to_floor": {"type": "boolean", "description": "Appoggia a quota 0 dopo la trasformazione."}
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
                "denoise": {"type": "boolean"},
                "exposure_ev": {"type": "number"}
            }), &[]),
        tool("render", "Avvia il render, aspetta la fine, salva il PNG e restituisce l'anteprima da guardare.",
            json!({
                "samples": {"type": "integer", "description": "Campioni per pixel (predefinito quelli impostati; per una prova veloce 32–64)."},
                "output": {"type": "string", "description": "Percorso assoluto del PNG (predefinito: cartella renders)."},
                "overwrite": {"type": "boolean"},
                "wait": {"type": "boolean", "description": "Aspetta la fine (predefinito sì)."},
                "timeout_s": {"type": "number", "description": "Attesa massima (predefinito 900)."},
                "preview_px": {"type": "integer", "description": "Lato massimo dell'anteprima restituita (predefinito 768, 0 = nessuna)."}
            }), &[]),
        tool("stop_render", "Ferma il render in corso (l'immagine resta).", json!({}), &[]),
        tool("save_image", "Salva come PNG l'ultima immagine renderizzata.",
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

/// Esegue uno strumento. Errori e avvisi che l'interfaccia mostrerebbe in finestra tornano
/// nella risposta (così non restano finestre aperte davanti all'utente).
pub fn call(app: &mut ForgeApp, name: &str, args: &Value) -> Step {
    app.error = None;
    app.info = None;
    match dispatch(app, name, args) {
        Ok(Step::Done(out)) => Step::Done(with_messages(app, out)),
        Ok(Step::Wait(mut w)) => {
            let mut poll = std::mem::replace(&mut w.poll, Box::new(|_| None));
            w.poll = Box::new(move |app| poll(app).map(|o| with_messages(app, o)));
            Step::Wait(w)
        }
        Err(e) => Step::Done(ToolOutput::error(e)),
    }
}

fn with_messages(app: &mut ForgeApp, mut out: ToolOutput) -> ToolOutput {
    if let Some(e) = app.error.take() {
        out.is_error = true;
        out.text = format!("{e}\n\n{}", out.text);
    }
    if let Some(i) = app.info.take() {
        out.text = format!("{}\n\nAvviso di ForgeRender: {i}", out.text);
    }
    out
}

fn dispatch(app: &mut ForgeApp, name: &str, v: &Value) -> R {
    let a = Args(v);
    match name {
        "scene_info" => scene_info(app, &a),
        "new_scene" => new_scene(app, &a),
        "open_project" => open_project(app, &a),
        "save_project" => save_project(app, &a),
        "import_model" => import_model(app, &a),
        "prepare_studio" => prepare_studio(app, &a),
        "fit_scene_to_product" => {
            if presets::product_bounds(&app.scene).is_empty() {
                return Err("nella scena non c'è un prodotto: importa prima un modello".into());
            }
            app.simple_fit_scene();
            done(json!({"esito": app.simple_msg.clone()}))
        }
        "undo_preset" => {
            if app.preset_backup.is_none() {
                return Err("nessun preset da annullare".into());
            }
            app.simple_undo_preset();
            done(json!({"esito": app.simple_msg.clone()}))
        }
        "add_object" => add_object(app, &a),
        "transform_objects" => transform_objects(app, &a),
        "set_visibility" => set_visibility(app, &a),
        "add_light" => add_light(app, &a),
        "edit_light" => edit_light(app, &a),
        "remove_light" => remove_light(app, &a),
        "set_all_lights_color" => {
            let k = a.f32("kelvin")?.ok_or("manca \"kelvin\"")?;
            presets::set_lights_kelvin(&mut app.scene, k.clamp(1000.0, 40000.0));
            app.shading_changed();
            done(json!({"luci": app.scene.lights.len(), "kelvin": k}))
        }
        "list_materials" => list_materials(app, &a),
        "apply_material" => apply_material(app, &a),
        "create_material" => create_material(app, &a),
        "set_camera" => set_camera(app, &a),
        "set_environment" => set_environment(app, &a),
        "set_render_settings" => set_render_settings(app, &a),
        "render" => render(app, &a),
        "stop_render" => {
            app.stop_render();
            done(json!({"esito": "render fermato"}))
        }
        "save_image" => {
            let path = output_path(&a.req_str("path")?, "png", a.bool("overwrite")?.unwrap_or(false), None)?;
            let s = save_frame(app, &path)?;
            done(json!({"file": path.display().to_string(), "campioni": s}))
        }
        other => Err(format!("strumento sconosciuto: {other}")),
    }
}

// --- Scena e progetto ---

fn scene_info(app: &mut ForgeApp, a: &Args) -> R {
    let max = a.u32("max_components")?.unwrap_or(80) as usize;
    let mut v = ops::scene_json(&app.scene, max);
    let s = &app.settings;
    let p = app.session.progress();
    v["progetto"] = json!(app.meta.path.as_ref().map(|p| p.display().to_string()));
    v["modifiche_non_salvate"] = json!(app.unsaved);
    v["selezione"] = json!(app.selection.iter().map(|&i| app.scene.nodes[i].name.clone()).collect::<Vec<_>>());
    v["aspetto"] = json!(presets::matching_look(&app.scene, &app.settings)
        .map(|l| l.label())
        .unwrap_or("personalizzato"));
    v["render"] = json!({
        "larghezza": s.width,
        "altezza": s.height,
        "campioni": s.target_samples,
        "dispositivo": if s.backend == BackendKind::Gpu { "gpu" } else { "cpu" },
        "denoiser": s.denoise.mode != DenoiseMode::Off,
        "esposizione_ev": s.exposure_ev,
        "ambiente_visibile": s.environment_visible,
        "stato": state_name(&p.state),
        "campioni_fatti": p.samples,
    });
    done(v)
}

fn state_name(s: &SessionState) -> String {
    match s {
        SessionState::Idle => "fermo".into(),
        SessionState::Rendering => "in corso".into(),
        SessionState::Paused => "in pausa".into(),
        SessionState::Completed => "completato".into(),
        SessionState::Cancelled => "interrotto".into(),
        SessionState::Failed(e) => format!("errore: {e}"),
    }
}

fn new_scene(app: &mut ForgeApp, a: &Args) -> R {
    match a.req_str("kind")?.to_lowercase().as_str() {
        "studio" => {
            let settings = app.prefs.new_project_settings();
            app.replace_scene(
                presets::studio_demo_scene(),
                forge_project::ProjectMeta {
                    test_scene: true,
                    ..Default::default()
                },
                Some(settings),
            );
            app.simple_look = Look::Studio;
        }
        "vuota" | "empty" => app.new_empty_scene(),
        k => return Err(format!("tipo di scena \"{k}\": usa studio o vuota")),
    }
    done(json!({"esito": "nuova scena", "luci": app.scene.lights.len(), "componenti": app.scene.nodes.len()}))
}

fn existing_file(p: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(p);
    if !path.is_absolute() {
        return Err(format!("serve un percorso assoluto: {p}"));
    }
    if !path.is_file() {
        return Err(format!("file non trovato: {p}"));
    }
    Ok(path)
}

/// Attende la fine dell'operazione in background (import, apertura), poi `finish`.
fn wait_pending(secs: u64, finish: impl Fn(&mut ForgeApp) -> ToolOutput + 'static) -> R {
    Ok(Step::Wait(Waiter {
        deadline: Instant::now() + Duration::from_secs(secs),
        poll: Box::new(move |app| app.pending.is_none().then(|| finish(app))),
    }))
}

fn product_summary(app: &ForgeApp) -> Value {
    let b = presets::product_bounds(&app.scene);
    if b.is_empty() {
        return Value::Null;
    }
    let r = |v: Vec3| v.to_array().map(|x| (x * 1000.0).round() / 1000.0);
    json!({"centro_m": r(b.center()), "dimensioni_m": r(b.max - b.min), "componenti": app.scene.nodes.iter().filter(|n| !n.removed).count()})
}

fn open_project(app: &mut ForgeApp, a: &Args) -> R {
    let path = existing_file(&a.req_str("path")?)?;
    app.open_project(&path);
    let p = path.display().to_string();
    wait_pending(600, move |app| {
        if app.error.is_some() {
            return ToolOutput::error(format!("apertura di {p} fallita"));
        }
        ToolOutput::ok(json!({"aperto": p, "prodotto": product_summary(app), "luci": app.scene.lights.len()}))
    })
}

fn save_project(app: &mut ForgeApp, a: &Args) -> R {
    let path = match a.str("path")? {
        Some(p) if !p.is_empty() => {
            let same = app.meta.path.as_ref().is_some_and(|m| m == Path::new(&p));
            output_path(&p, forge_project::EXTENSION, a.bool("overwrite")?.unwrap_or(false) || same, None)?
        }
        _ => app.meta.path.clone().ok_or("il progetto non ha ancora un file: indica \"path\"")?,
    };
    forge_project::save(&path, &app.scene, &app.meta, &app.settings).map_err(|e| format!("salvataggio fallito: {e:#}"))?;
    app.meta.path = Some(path.clone());
    app.unsaved = false;
    done(json!({"salvato": path.display().to_string()}))
}

fn import_model(app: &mut ForgeApp, a: &Args) -> R {
    let path = existing_file(&a.req_str("path")?)?;
    if let Some(u) = a.str("unit")? {
        use forge_scene::units::LengthUnit as U;
        app.import_opts.unit = match u.as_str() {
            "mm" => U::Millimeter,
            "cm" => U::Centimeter,
            "m" => U::Meter,
            "in" | "pollici" => U::Inch,
            _ => return Err(format!("unità \"{u}\": mm, cm, m o in")),
        };
    }
    if let Some(u) = a.str("up_axis")? {
        use forge_import::UpAxis as U;
        app.import_opts.up = match u.to_lowercase().as_str() {
            "auto" => U::Auto,
            "y" => U::Y,
            "z" => U::Z,
            _ => return Err(format!("asse \"{u}\": auto, y o z")),
        };
    }
    app.import_opts.drop_to_floor = a.bool("drop_to_floor")?.unwrap_or(true);
    if app.pending.is_some() {
        return Err("c'è già un'operazione in corso: riprova tra poco".into());
    }
    app.import_path(&path);
    let p = path.display().to_string();
    wait_pending(900, move |app| {
        if app.error.is_some() {
            return ToolOutput::error(format!("importazione di {p} fallita"));
        }
        ToolOutput::ok(json!({"importato": p, "prodotto": product_summary(app)}))
    })
}

fn look_of(s: &str) -> Result<Look, String> {
    Ok(match s.to_lowercase().as_str() {
        "studio" | "studio_fotografico" | "photo_studio" => Look::Studio,
        "chiaro" | "light" | "bianco" => Look::Light,
        "scuro" | "dark" | "nero" => Look::Dark,
        "scenografico" | "dramatic" | "drammatico" => Look::Dramatic,
        _ => return Err(format!("aspetto \"{s}\": studio, chiaro, scuro o scenografico")),
    })
}

fn quality_of(s: &str) -> Result<Quality, String> {
    Ok(match s.to_lowercase().as_str() {
        "bozza" | "draft" => Quality::Draft,
        "bilanciata" | "balanced" | "media" => Quality::Balanced,
        "alta" | "high" | "finale" => Quality::High,
        _ => return Err(format!("qualità \"{s}\": bozza, bilanciata o alta")),
    })
}

fn prepare_studio(app: &mut ForgeApp, a: &Args) -> R {
    if let Some(l) = a.str("look")? {
        app.simple_look = look_of(&l)?;
    }
    if let Some(q) = a.str("quality")? {
        app.simple_quality = quality_of(&q)?;
    }
    if presets::product_bounds(&app.scene).is_empty() {
        // Senza prodotto: solo aspetto e qualità (luci, ambiente, colori).
        app.simple_apply_look(app.simple_look);
        app.simple_apply_quality(app.simple_quality);
        return done(json!({"esito": "nessun prodotto nella scena: applicati solo aspetto e qualità", "aspetto": app.simple_look.label()}));
    }
    app.simple_prepare_scene();
    done(json!({
        "esito": app.simple_msg.clone(),
        "luci": app.scene.lights.iter().enumerate().map(|(i, l)| ops::light_json(i, l)).collect::<Vec<_>>(),
        "prodotto": product_summary(app),
    }))
}

fn add_object(app: &mut ForgeApp, a: &Args) -> R {
    let k = a.req_str("kind")?.to_lowercase();
    let kind = match k.as_str() {
        "pavimento" | "floor" => AddKind::Floor,
        "fondale" | "backdrop" | "fondale_curvo" => AddKind::Backdrop,
        "fondale_fotografico" | "tela" | "photo_backdrop" | "sweep" => AddKind::StudioSweep,
        "parete" | "wall" => AddKind::Wall,
        "cubo" | "cube" | "box" => AddKind::Box,
        "sfera" | "sphere" => AddKind::Sphere,
        "cilindro" | "cylinder" => AddKind::Cylinder,
        "tazza" | "cup" => AddKind::Cup,
        _ => return Err(format!("oggetto \"{k}\" sconosciuto")),
    };
    app.add_object(kind);
    let name = app.selection.iter().next().map(|&i| app.scene.nodes[i].name.clone());
    done(json!({"aggiunto": name}))
}

fn transform_objects(app: &mut ForgeApp, a: &Args) -> R {
    let selection: Vec<usize> = app.selection.iter().copied().collect();
    let nodes = ops::resolve_targets(&app.scene, a.0.get("targets"), &selection)?;
    let mv = a.vec3("move_m")?;
    let rot = a.vec3("rotate_deg")?;
    let scale = a.f32("scale")?;
    if scale.is_some_and(|s| s <= 0.0) {
        return Err("\"scale\" deve essere maggiore di 0".into());
    }
    app.push_undo(&nodes);
    for &i in &nodes {
        if let Some(d) = mv {
            app.scene.translate_world(i, d);
        }
        if let Some(r) = rot {
            app.scene.nodes[i].user.rotation_deg += r;
        }
        if let Some(s) = scale {
            app.scene.nodes[i].user.scale *= s;
        }
    }
    if a.bool("drop_to_floor")?.unwrap_or(false) {
        for &i in &nodes {
            app.scene.drop_to_floor(i, 0.0);
        }
    }
    app.geometry_changed();
    done(
        json!({"componenti": nodes.iter().map(|&i| app.scene.nodes[i].name.clone()).collect::<Vec<_>>(), "prodotto": product_summary(app)}),
    )
}

fn set_visibility(app: &mut ForgeApp, a: &Args) -> R {
    let selection: Vec<usize> = app.selection.iter().copied().collect();
    let nodes = ops::resolve_targets(&app.scene, a.0.get("targets"), &selection)?;
    let visible = a.bool("visible")?.ok_or("manca \"visible\"")?;
    app.push_undo(&nodes);
    for &i in &nodes {
        app.scene.nodes[i].visible = visible;
    }
    app.geometry_changed();
    done(json!({"componenti": nodes.len(), "visibili": visible}))
}

// --- Luci ---

/// Centro e raggio del prodotto (o della scena se manca).
fn product_anchor(app: &ForgeApp) -> (Vec3, f32) {
    let b = presets::product_bounds(&app.scene);
    if b.is_empty() {
        app.anchor()
    } else {
        (b.center(), b.radius().max(0.05))
    }
}

fn light_color(a: &Args) -> Result<Option<[f32; 3]>, String> {
    if let Some(c) = a.color("color")? {
        return Ok(Some(c));
    }
    Ok(a.f32("kelvin")?.map(forge_scene::light::kelvin_to_rgb))
}

fn add_light(app: &mut ForgeApp, a: &Args) -> R {
    let (c, r) = product_anchor(app);
    let kind = a.str("type")?.unwrap_or_else(|| "area".into()).to_lowercase();
    let side = a.str("side")?.unwrap_or_else(|| "destra".into());
    let cam = app.scene.render_camera().clone();
    let pos = match a.vec3("position")? {
        Some(p) => p,
        None => ops::light_position(&cam, c, r, &side)?,
    };
    let target = a.vec3("target")?.unwrap_or(c);
    let strength = a.f32("strength")?.unwrap_or(1.0).max(0.0);
    let name = a.str("name")?.unwrap_or_else(|| format!("Luce {side}"));
    let mut l = match kind.as_str() {
        "area" | "softbox" => {
            let s = a.f32("size_m")?.unwrap_or((1.2 * r).max(0.2)).max(0.01);
            Light::area(&name, pos, target, s, s, 4.0 * strength)
        }
        "punto" | "point" | "puntiforme" => Light::point(&name, pos, 20.0 * r * r * strength),
        "spot" | "faretto" => {
            let cone = a.f32("cone_deg")?.unwrap_or(40.0).clamp(1.0, 170.0);
            Light {
                kind: LightKind::Spot {
                    inner_angle_deg: cone * 0.6,
                    outer_angle_deg: cone,
                },
                target,
                ..Light::point(&name, pos, 40.0 * r * r * strength)
            }
        }
        "direzionale" | "directional" | "sole" | "sun" => Light {
            kind: LightKind::Directional,
            target,
            ..Light::point(&name, pos, 3.0 * strength)
        },
        _ => return Err(format!("tipo di luce \"{kind}\": area, punto, spot o direzionale")),
    };
    if let Some(i) = a.f32("intensity")? {
        l.intensity = i.max(0.0);
    }
    if let Some(col) = light_color(a)? {
        l.color = col;
    }
    if let Some(s) = a.bool("shadows")? {
        l.cast_shadows = s;
    }
    if l.has_range() {
        if let Some(rg) = a.f32("range_m")? {
            l.range = rg.max(0.0);
        }
        if let Some(s) = a.f32("range_softness")? {
            l.range_softness = s.clamp(0.0, 1.0);
        }
    }
    app.scene.lights.push(l);
    let i = app.scene.lights.len() - 1;
    app.selected_light = Some(i);
    app.shading_changed();
    done(ops::light_json(i, &app.scene.lights[i]))
}

fn edit_light(app: &mut ForgeApp, a: &Args) -> R {
    let i = ops::find_light(&app.scene, a.0.get("light").ok_or("manca \"light\"")?)?;
    let (c, r) = product_anchor(app);
    let cam = app.scene.render_camera().clone();
    let side_pos = match a.str("side")? {
        Some(s) => Some(ops::light_position(&cam, c, r, &s)?),
        None => None,
    };
    let col = light_color(a)?;
    let l = &mut app.scene.lights[i];
    if let Some(n) = a.str("name")? {
        l.name = n;
    }
    if let Some(v) = a.f32("intensity")? {
        l.intensity = v.max(0.0);
    }
    if let Some(s) = a.f32("strength")? {
        l.intensity *= s.max(0.0);
    }
    if let Some(col) = col {
        l.color = col;
    }
    if let Some(p) = side_pos {
        l.position = p;
        l.target = c;
    }
    if let Some(p) = a.vec3("position")? {
        l.position = p;
    }
    if let Some(t) = a.vec3("target")? {
        l.target = t;
    }
    if a.bool("aim_at_product")?.unwrap_or(false) {
        l.target = c;
    }
    if let Some(e) = a.bool("enabled")? {
        l.enabled = e;
    }
    if let Some(s) = a.bool("shadows")? {
        l.cast_shadows = s;
    }
    if let LightKind::Area { width, height } = &mut l.kind {
        match a.0.get("size_m") {
            Some(Value::Number(n)) => {
                let s = n.as_f64().unwrap_or(1.0).max(0.01) as f32;
                (*width, *height) = (s, s);
            }
            Some(Value::Array(v)) if v.len() == 2 => {
                *width = v[0].as_f64().unwrap_or(*width as f64).max(0.01) as f32;
                *height = v[1].as_f64().unwrap_or(*height as f64).max(0.01) as f32;
            }
            _ => {}
        }
    }
    if let LightKind::Spot {
        inner_angle_deg,
        outer_angle_deg,
    } = &mut l.kind
    {
        let soft = if *outer_angle_deg > 0.0 {
            1.0 - *inner_angle_deg / *outer_angle_deg
        } else {
            0.4
        };
        let soft = a.f32("cone_softness_pct")?.map(|p| (p / 100.0).clamp(0.0, 1.0)).unwrap_or(soft);
        if let Some(cone) = a.f32("cone_deg")? {
            *outer_angle_deg = cone.clamp(1.0, 170.0);
        }
        *inner_angle_deg = *outer_angle_deg * (1.0 - soft);
    }
    if l.has_range() {
        if let Some(rg) = a.f32("range_m")? {
            l.range = rg.max(0.0);
        }
        if let Some(s) = a.f32("range_softness")? {
            l.range_softness = s.clamp(0.0, 1.0);
        }
    }
    app.selected_light = Some(i);
    app.shading_changed();
    done(ops::light_json(i, &app.scene.lights[i]))
}

fn remove_light(app: &mut ForgeApp, a: &Args) -> R {
    let v = a.0.get("light").ok_or("manca \"light\"")?;
    if v.as_str()
        .is_some_and(|s| matches!(s.trim().to_lowercase().as_str(), "tutte" | "all"))
    {
        let n = app.scene.lights.len();
        app.scene.lights.clear();
        app.selected_light = None;
        app.shading_changed();
        return done(json!({"eliminate": n}));
    }
    let i = ops::find_light(&app.scene, v)?;
    let l = app.scene.lights.remove(i);
    app.selected_light = None;
    app.shading_changed();
    done(json!({"eliminata": l.name, "luci_rimaste": app.scene.lights.len()}))
}

// --- Materiali ---

fn list_materials(app: &mut ForgeApp, a: &Args) -> R {
    let q = a.str("search")?.unwrap_or_default().to_lowercase();
    let words: Vec<&str> = q.split_whitespace().collect();
    let hit = |t: String| words.iter().all(|w| t.to_lowercase().contains(w));
    let source = a.str("source")?.unwrap_or_else(|| "tutti".into());
    let limit = a.u32("limit")?.unwrap_or(40).clamp(1, 500) as usize;
    let mut out = Vec::new();
    if source != "libreria" {
        for m in app
            .scene
            .materials
            .materials
            .iter()
            .filter(|m| hit(format!("{} {} {}", m.name, m.category, m.id.0)))
        {
            out.push(json!({
                "id": m.id.0, "nome": m.name, "categoria": m.category, "origine": "progetto",
                "metallic": m.metallic, "roughness": m.roughness, "transmission": m.transmission,
                "texture": m.base_color_texture.is_some(),
            }));
        }
    }
    if source != "progetto" {
        for m in app
            .library
            .materials()
            .iter()
            .filter(|m| hit(format!("{} {} {}", m.material.name, m.categories.join(" "), m.tags.join(" "))))
        {
            out.push(json!({
                "id": m.material.id.0, "nome": m.material.name, "categoria": m.categories.join(", "),
                "origine": format!("libreria ({})", m.source), "texture": true,
            }));
        }
    }
    let total = out.len();
    out.truncate(limit);
    done(json!({"trovati": total, "materiali": out}))
}

fn assign(app: &mut ForgeApp, id: &forge_scene::MaterialId, targets: Option<&Value>) -> Result<Vec<String>, String> {
    let selection: Vec<usize> = app.selection.iter().copied().collect();
    let roots = ops::resolve_targets(&app.scene, targets, &selection)?;
    let nodes = ops::mesh_nodes_under(&app.scene, &roots);
    if nodes.is_empty() {
        return Err("nessun componente con geometria a cui applicare il materiale".into());
    }
    for &i in &nodes {
        app.scene.assignments.insert(app.scene.nodes[i].id.clone(), id.clone());
    }
    app.selected_material = id.clone();
    app.appearance_changed();
    Ok(nodes.iter().take(30).map(|&i| app.scene.nodes[i].name.clone()).collect())
}

fn apply_material(app: &mut ForgeApp, a: &Args) -> R {
    let q = a.req_str("material")?;
    let found = ops::find_material(&app.scene, app.library.materials(), &q)?;
    let id = match found {
        FoundMaterial::Project(id) => id,
        FoundMaterial::Library(m) => {
            let id = m.id.clone();
            if app.scene.materials.get(&id).is_none() {
                app.scene.materials.upsert(*m);
            }
            id
        }
    };
    let names = assign(app, &id, a.0.get("targets"))?;
    let m = app.scene.materials.get(&id).expect("materiale");
    done(json!({"materiale": m.name, "id": id.0, "componenti": names}))
}

fn create_material(app: &mut ForgeApp, a: &Args) -> R {
    let name = a.req_str("name")?;
    let mut m: Material = match a.str("base")? {
        Some(b) => match ops::find_material(&app.scene, app.library.materials(), &b)? {
            FoundMaterial::Project(id) => app.scene.materials.get(&id).cloned().expect("materiale"),
            FoundMaterial::Library(m) => *m,
        },
        None => Material::default(),
    };
    m.id = ops::material_id_for(&name);
    m.name = name;
    m.category = "IA".into();
    if let Some(c) = a.color("color")? {
        m.base_color = c;
    }
    if let Some(v) = a.f32("metallic")? {
        m.metallic = v.clamp(0.0, 1.0);
    }
    if let Some(v) = a.f32("roughness")? {
        m.roughness = v.clamp(0.0, 1.0);
    }
    if let Some(v) = a.f32("transmission")? {
        m.transmission = v.clamp(0.0, 1.0);
    }
    if let Some(v) = a.f32("ior")? {
        m.ior = v.clamp(1.0, 3.0);
    }
    if let Some(v) = a.f32("specular")? {
        m.specular = v.clamp(0.0, 1.0);
    }
    if let Some(c) = a.color("emission_color")? {
        m.emission = c;
        if m.emission_strength <= 0.0 {
            m.emission_strength = 1.0;
        }
    }
    if let Some(v) = a.f32("emission_strength")? {
        m.emission_strength = v.max(0.0);
    }
    let id = m.id.clone();
    let summary = json!({
        "id": id.0, "nome": m.name, "colore": ops::to_hex(m.base_color),
        "metallic": m.metallic, "roughness": m.roughness, "transmission": m.transmission,
    });
    app.scene.materials.upsert(m);
    let names = if a.has("targets") || !presets::product_bounds(&app.scene).is_empty() {
        assign(app, &id, a.0.get("targets"))?
    } else {
        app.appearance_changed();
        Vec::new()
    };
    done(json!({"materiale": summary, "applicato_a": names}))
}

// --- Camera, ambiente, impostazioni ---

fn set_camera(app: &mut ForgeApp, a: &Args) -> R {
    use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};
    let lim = FRAC_PI_2 - 0.02;
    let mut turned = false;
    if let Some(v) = a.str("view")? {
        let (yaw, pitch, ortho) = match v.to_lowercase().as_str() {
            "frontale" | "front" => (0.0, 0.12, None),
            "retro" | "back" => (std::f32::consts::PI, 0.12, None),
            "destra" | "right" => (FRAC_PI_2, 0.12, None),
            "sinistra" | "left" => (-FRAC_PI_2, 0.12, None),
            "alto" | "top" => (0.0, lim, None),
            "tre_quarti" | "three_quarter" => (FRAC_PI_4, 0.35, None),
            "isometrica" | "iso" => (FRAC_PI_4, crate::ui::navigator::ISO_PITCH, Some(true)),
            _ => return Err(format!("vista \"{v}\" sconosciuta")),
        };
        app.orbit.yaw = yaw;
        app.orbit.pitch = pitch;
        if let Some(o) = ortho {
            app.orbit.lens.ortho = o;
        }
        turned = true;
    }
    if let Some(y) = a.f32("yaw_deg")? {
        app.orbit.yaw = y.to_radians();
        turned = true;
    }
    if let Some(p) = a.f32("pitch_deg")? {
        app.orbit.pitch = p.to_radians().clamp(-lim, lim);
        turned = true;
    }
    if let Some(mm) = a.f32("focal_mm")? {
        let mut c = app.orbit.camera();
        c.set_focal_mm(mm.clamp(8.0, 600.0));
        app.orbit.vfov_deg = c.vfov_deg;
        turned = true;
    }
    if let Some(o) = a.bool("orthographic")? {
        app.orbit.lens.ortho = o;
    }
    let frame = a
        .str("frame")?
        .unwrap_or_else(|| if turned { "prodotto".into() } else { "nessuno".into() });
    match frame.to_lowercase().as_str() {
        "prodotto" | "product" => app.frame_product(),
        "tutto" | "all" => {
            let b = app.scene.bounds();
            app.orbit.frame(&b);
        }
        "nessuno" | "none" => {}
        f => return Err(format!("inquadratura \"{f}\": prodotto, tutto o nessuno")),
    }
    if let Some(z) = a.f32("zoom")? {
        app.orbit.zoom(z.clamp(0.05, 20.0));
    }
    if let Some(d) = a.bool("depth_of_field")? {
        app.orbit.lens.dof = d;
        app.orbit.lens.focus_distance = app.orbit.distance.max(0.01);
    }
    if let Some(f) = a.f32("f_stop")? {
        app.orbit.lens.f_stop = f.clamp(0.7, 64.0);
    }
    app.commit_view();
    let c = app.scene.render_camera();
    done(json!({
        "posizione_m": c.position.to_array(),
        "bersaglio_m": c.target.to_array(),
        "focale_mm": c.focal_mm(),
        "ortografica": c.lens.ortho,
        "profondita_di_campo": c.lens.dof,
    }))
}

fn set_environment(app: &mut ForgeApp, a: &Args) -> R {
    let color = a.color("color")?;
    let intensity = a.f32("intensity")?.map(|v| v.max(0.0));
    if let Some(t) = a.str("type")? {
        let cur_strength = match &app.scene.environment {
            Environment::Uniform { strength, .. } | Environment::Gradient { strength, .. } | Environment::Hdri { strength, .. } => {
                *strength
            }
        };
        app.scene.environment = match t.to_lowercase().as_str() {
            "uniforme" | "uniform" => Environment::Uniform {
                color: color.unwrap_or([1.0; 3]),
                strength: intensity.unwrap_or(cur_strength),
            },
            "nero" | "black" => Environment::Uniform {
                color: [0.0; 3],
                strength: 0.0,
            },
            "gradiente" | "gradient" => {
                let mut e = Environment::default();
                if let (Environment::Gradient { strength, .. }, Some(i)) = (&mut e, intensity) {
                    *strength = i;
                }
                e
            }
            "hdri" => {
                let path = existing_file(&a.req_str("hdri_path")?)?;
                Environment::Hdri {
                    path,
                    strength: intensity.unwrap_or(1.0),
                    rotation_deg: a.f32("rotation_deg")?.unwrap_or(0.0),
                }
            }
            _ => return Err(format!("ambiente \"{t}\": uniforme, gradiente, hdri o nero")),
        };
    } else {
        match &mut app.scene.environment {
            Environment::Uniform { color: c, strength } => {
                if let Some(col) = color {
                    *c = col;
                }
                if let Some(i) = intensity {
                    *strength = i;
                }
            }
            Environment::Gradient { strength, .. } => {
                if let Some(i) = intensity {
                    *strength = i;
                }
            }
            Environment::Hdri {
                strength, rotation_deg, ..
            } => {
                if let Some(i) = intensity {
                    *strength = i;
                }
                if let Some(r) = a.f32("rotation_deg")? {
                    *rotation_deg = r;
                }
            }
        }
    }
    if let Some(v) = a.bool("visible")? {
        app.settings.environment_visible = v;
    }
    app.shading_changed();
    let v = ops::scene_json(&app.scene, 0);
    done(json!({"ambiente": v["ambiente"], "visibile": app.settings.environment_visible}))
}

fn set_render_settings(app: &mut ForgeApp, a: &Args) -> R {
    if let Some(q) = a.str("quality")? {
        let q = quality_of(&q)?;
        app.simple_apply_quality(q);
    }
    if let Some(f) = a.str("format")? {
        let (w, h) = match f.as_str() {
            "16:9" | "orizzontale" => (1920, 1080),
            "1:1" | "quadrato" => (1080, 1080),
            "4:5" | "verticale" => (1080, 1350),
            "9:16" | "storie" => (1080, 1920),
            _ => return Err(format!("formato \"{f}\": 16:9, 1:1, 4:5 o 9:16")),
        };
        (app.settings.width, app.settings.height) = (w, h);
    }
    if let Some(w) = a.u32("width")? {
        app.settings.width = w.clamp(16, 16384);
    }
    if let Some(h) = a.u32("height")? {
        app.settings.height = h.clamp(16, 16384);
    }
    if let Some(s) = a.u32("samples")? {
        app.settings.target_samples = s.min(1 << 20);
    }
    if let Some(d) = a.str("device")? {
        app.settings.backend = match d.to_lowercase().as_str() {
            "gpu" => {
                if app.gpu_ready() == Some(false) {
                    return Err("nessuna GPU utilizzabile: resta la CPU".into());
                }
                BackendKind::Gpu
            }
            "cpu" => BackendKind::Cpu,
            _ => return Err(format!("dispositivo \"{d}\": cpu o gpu")),
        };
    }
    if let Some(d) = a.bool("denoise")? {
        app.settings.denoise.mode = if d { DenoiseMode::Final } else { DenoiseMode::Off };
    }
    if let Some(e) = a.f32("exposure_ev")? {
        app.settings.exposure_ev = e.clamp(-10.0, 10.0);
    }
    app.unsaved = true;
    let s = &app.settings;
    done(json!({
        "larghezza": s.width, "altezza": s.height, "campioni": s.target_samples,
        "dispositivo": if s.backend == BackendKind::Gpu { "gpu" } else { "cpu" },
        "denoiser": s.denoise.mode != DenoiseMode::Off, "esposizione_ev": s.exposure_ev,
    }))
}

// --- Render ---

/// Percorso d'uscita assoluto con l'estensione giusta; rifiuta di sovrascrivere senza permesso.
fn output_path(p: &str, ext: &str, overwrite: bool, default_dir: Option<&Path>) -> Result<PathBuf, String> {
    let mut path = PathBuf::from(p);
    if !path.is_absolute() {
        match default_dir {
            Some(d) => path = d.join(path),
            None => return Err(format!("serve un percorso assoluto: {p}")),
        }
    }
    if !path.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext)) {
        path.set_extension(ext);
    }
    if path.exists() && !overwrite {
        return Err(format!(
            "{} esiste già: per sostituirlo serve overwrite = true (chiedilo all'utente)",
            path.display()
        ));
    }
    Ok(path)
}

/// Cartella predefinita dei render: accanto al progetto, altrimenti accanto ai log
/// (portable: cartella del programma; installata: %LOCALAPPDATA%\ForgeRender).
fn renders_dir(app: &ForgeApp) -> PathBuf {
    if let Some(d) = app.meta.path.as_ref().and_then(|p| p.parent()) {
        return d.join("render");
    }
    forge_telemetry::default_log_dir()
        .and_then(|d| d.parent().map(|p| p.join("renders")))
        .unwrap_or_else(|| std::env::temp_dir().join("ForgeRender").join("renders"))
}

fn save_frame(app: &ForgeApp, path: &Path) -> Result<u32, String> {
    let frame = app.session.latest_frame().ok_or("nessuna immagine: avvia prima un render")?;
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| format!("cartella {} non creata: {e}", d.display()))?;
    }
    forge_image_io::save_frame_png(
        path,
        &frame,
        app.settings.tone_mapping,
        app.settings.exposure_ev,
        app.settings.denoise.strength,
    )
    .map_err(|e| format!("salvataggio di {} fallito: {e:#}", path.display()))?;
    Ok(frame.samples)
}

fn preview_png(path: &Path, px: u32) -> Option<Vec<u8>> {
    let img = image::open(path).ok()?.thumbnail(px, px);
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png).ok()?;
    Some(out.into_inner())
}

fn render(app: &mut ForgeApp, a: &Args) -> R {
    if let Some(s) = a.u32("samples")? {
        app.settings.target_samples = s.clamp(1, 1 << 20);
    }
    let wait = a.bool("wait")?.unwrap_or(true);
    if wait && app.settings.target_samples == 0 {
        return Err("i campioni sono illimitati: indica \"samples\" per sapere quando finire".into());
    }
    let dir = renders_dir(app);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let out = a.str("output")?.unwrap_or_else(|| format!("render_{stamp}.png"));
    let path = output_path(&out, "png", a.bool("overwrite")?.unwrap_or(false), Some(&dir))?;
    let preview = a.u32("preview_px")?.unwrap_or(768).min(2048);
    let timeout = a.f32("timeout_s")?.unwrap_or(900.0).clamp(5.0, 6.0 * 3600.0);
    let before = app.session.progress().job_id;
    app.start_render();
    if !wait {
        return done(json!({"esito": "render avviato", "campioni": app.settings.target_samples}));
    }
    let started = Instant::now();
    Ok(Step::Wait(Waiter {
        deadline: Instant::now() + Duration::from_secs_f32(timeout),
        poll: Box::new(move |app| {
            if app.error.is_some() {
                return Some(ToolOutput::error("render non avviato"));
            }
            let p = app.session.progress();
            if p.job_id == before || app.is_preparing() || app.is_baking_fog() {
                return None;
            }
            match &p.state {
                SessionState::Completed if !app.render_is_stale() => {}
                SessionState::Failed(e) => return Some(ToolOutput::error(format!("render fallito: {e}"))),
                SessionState::Cancelled => return Some(ToolOutput::error("render interrotto")),
                _ => return None,
            }
            Some(match save_frame(app, &path) {
                Ok(samples) => ToolOutput {
                    png: (preview > 0).then(|| preview_png(&path, preview)).flatten(),
                    ..ToolOutput::ok(json!({
                        "file": path.display().to_string(),
                        "campioni": samples,
                        "larghezza": p.width,
                        "altezza": p.height,
                        "tempo_render_s": (p.elapsed.as_secs_f32() * 10.0).round() / 10.0,
                        "tempo_totale_s": (started.elapsed().as_secs_f32() * 10.0).round() / 10.0,
                        "dispositivo": p.backend,
                    }))
                },
                Err(e) => ToolOutput::error(e),
            })
        }),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_tool_has_a_valid_schema_and_handler() {
        let tools = list();
        let tools = tools.as_array().unwrap();
        assert!(tools.len() >= 20);
        let mut names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        for t in tools {
            assert_eq!(t["inputSchema"]["type"], "object", "{}", t["name"]);
            assert!(t["description"].as_str().unwrap().len() > 20);
            for r in t["inputSchema"]["required"].as_array().unwrap() {
                assert!(
                    t["inputSchema"]["properties"].get(r.as_str().unwrap()).is_some(),
                    "{}: {r}",
                    t["name"]
                );
            }
        }
        let n = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), n, "nomi doppi");
        // Ogni nome ha un ramo in `dispatch` (il sorgente lo elenca tra virgolette).
        let src = include_str!("tools.rs");
        let body = &src[src.find("fn dispatch").unwrap()..src.find("// --- Scena e progetto ---").unwrap()];
        for name in names {
            assert!(body.contains(&format!("\"{name}\" =>")), "manca il gestore di {name}");
        }
    }

    #[test]
    fn output_paths_refuse_to_overwrite() {
        let dir = std::env::temp_dir().join(format!("forge-mcp-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = output_path("prova", "png", false, Some(&dir)).unwrap();
        assert_eq!(p, dir.join("prova.png"));
        std::fs::write(&p, b"x").unwrap();
        assert!(output_path("prova.png", "png", false, Some(&dir)).is_err());
        assert!(output_path("prova.png", "png", true, Some(&dir)).is_ok());
        assert!(output_path("relativo.png", "png", false, None).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn looks_and_qualities_parse() {
        assert_eq!(look_of("Studio").unwrap(), Look::Studio);
        assert_eq!(look_of("dark").unwrap(), Look::Dark);
        assert!(look_of("viola").is_err());
        assert_eq!(quality_of("alta").unwrap(), Quality::High);
    }
}
