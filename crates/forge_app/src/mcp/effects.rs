//! Strumenti MCP degli effetti: foschia d'ambiente, fumo volumetrico, ghiaccio secco, polvere in
//! sospensione, lucciole e adesivi (decal). Aggiungono, modificano ed eliminano come i pulsanti
//! della scheda Effetti, con le stesse funzioni e gli stessi aggiornamenti della scena.
//!
//! I parametri si leggono e si scrivono con i nomi dei campi salvati nel progetto: `list_effects`
//! restituisce tutti i valori di ogni effetto, `edit_effect` cambia solo quelli indicati e
//! riporta i valori fuori intervallo nei limiti, come fa il pannello.

use super::ops::{self, Args};
use crate::app::ForgeApp;
use crate::i18n::{tf, tr};
use forge_math::Vec3;
use forge_scene::{FogVolume, Haze, HazeKind, Scene, SmokeShape, FOG_PRESETS, SMOKE_PRESETS};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Map, Value};

/// Un effetto della scena.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effect {
    Haze,
    Dust,
    Fireflies,
    Smoke(usize),
    Fog(usize),
    Decal(usize),
}

/// Tipi di foschia con il loro nome per l'assistente.
const HAZE_KINDS: [(&str, HazeKind); 5] = [
    ("nebbia_bassa", HazeKind::Ambient),
    ("nuvole_di_gas", HazeKind::GasClouds),
    ("fumo_tossico", HazeKind::ToxicFloor),
    ("strati", HazeKind::Layers),
    ("riccioli", HazeKind::Billows),
];

/// Nome di un preset come lo usa l'assistente: minuscole, parole unite da `_`.
pub fn preset_key(name: &str) -> String {
    let mut out = String::new();
    for c in name.to_lowercase().chars() {
        if c.is_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('_') && !out.is_empty() {
            out.push('_');
        }
    }
    out.trim_end_matches('_').to_string()
}

impl Effect {
    pub fn id(self) -> String {
        match self {
            Effect::Haze => "foschia".into(),
            Effect::Dust => "polvere".into(),
            Effect::Fireflies => "lucciole".into(),
            Effect::Smoke(i) => format!("fumo:{i}"),
            Effect::Fog(i) => format!("ghiaccio_secco:{i}"),
            Effect::Decal(i) => format!("adesivo:{i}"),
        }
    }

    fn kind(self) -> &'static str {
        match self {
            Effect::Haze => "foschia",
            Effect::Dust => "polvere",
            Effect::Fireflies => "lucciole",
            Effect::Smoke(_) => "fumo",
            Effect::Fog(_) => "ghiaccio_secco",
            Effect::Decal(_) => "adesivo",
        }
    }
}

/// Effetti presenti nella scena, nell'ordine dell'elenco.
pub fn present(scene: &Scene) -> Vec<Effect> {
    let mut v = Vec::new();
    if scene.haze.enabled {
        v.push(Effect::Haze);
    }
    if scene.haze_dust.enabled {
        v.push(Effect::Dust);
    }
    if scene.fireflies.enabled {
        v.push(Effect::Fireflies);
    }
    v.extend((0..scene.smokes.len()).map(Effect::Smoke));
    v.extend((0..scene.fogs.len()).map(Effect::Fog));
    v.extend((0..scene.decals.len()).map(Effect::Decal));
    v
}

fn name_of(scene: &Scene, e: Effect) -> String {
    match e {
        Effect::Haze => {
            let k = HAZE_KINDS.iter().find(|(_, k)| *k == scene.haze.kind).map_or("", |(n, _)| n);
            format!("{} ({k})", tr("Foschia"))
        }
        Effect::Dust => tr("Polvere in sospensione").into(),
        Effect::Fireflies => tr("Lucciole").into(),
        Effect::Smoke(i) => scene.smokes[i].name.clone(),
        Effect::Fog(i) => scene.fogs[i].name.clone(),
        Effect::Decal(i) => scene.decals[i].name.clone(),
    }
}

fn enabled_of(scene: &Scene, e: Effect) -> bool {
    match e {
        Effect::Haze => scene.haze.enabled,
        Effect::Dust => scene.haze_dust.enabled,
        Effect::Fireflies => scene.fireflies.enabled,
        Effect::Smoke(i) => scene.smokes[i].enabled,
        Effect::Fog(i) => scene.fogs[i].enabled,
        Effect::Decal(i) => scene.decals[i].enabled,
    }
}

/// Tutti i valori di un effetto, con i nomi dei campi salvati nel progetto.
fn values_of(scene: &Scene, e: Effect) -> Value {
    let v = match e {
        Effect::Haze => serde_json::to_value(&scene.haze),
        Effect::Dust => serde_json::to_value(&scene.haze_dust),
        Effect::Fireflies => serde_json::to_value(&scene.fireflies),
        Effect::Smoke(i) => serde_json::to_value(&scene.smokes[i]),
        Effect::Fog(i) => serde_json::to_value(&scene.fogs[i]),
        Effect::Decal(i) => serde_json::to_value(&scene.decals[i]),
    };
    v.unwrap_or(Value::Null)
}

/// Riga breve per `scene_info`.
pub fn summary(scene: &Scene) -> Value {
    Value::Array(
        present(scene)
            .into_iter()
            .map(|e| json!({"id": e.id(), "tipo": e.kind(), "nome": name_of(scene, e), "acceso": enabled_of(scene, e)}))
            .collect(),
    )
}

fn effect_json(scene: &Scene, e: Effect) -> Value {
    json!({
        "id": e.id(),
        "tipo": e.kind(),
        "nome": name_of(scene, e),
        "acceso": enabled_of(scene, e),
        "valori": values_of(scene, e),
    })
}

/// Elenco completo: ogni effetto con tutti i valori, e cosa si può aggiungere.
pub fn list(app: &ForgeApp) -> Result<Value, String> {
    let s = &app.scene;
    Ok(json!({
        "unita": "metri; y verso l'alto; colori [r, g, b] da 0 a 1 (lineari)",
        "effetti": present(s).into_iter().map(|e| effect_json(s, e)).collect::<Vec<_>>(),
        "da_aggiungere": {
            "foschia": HAZE_KINDS.map(|(n, _)| n),
            "fumo": SMOKE_PRESETS.iter().map(|p| preset_key(p.name)).collect::<Vec<_>>(),
            "ghiaccio_secco": FOG_PRESETS.iter().map(|p| preset_key(p.name)).collect::<Vec<_>>(),
            "polvere": [],
            "lucciole": ["bosco_incantato"],
            "adesivo": "image_path (PNG, JPG…) sui componenti targets",
        },
    }))
}

/// Trova l'effetto indicato: id di `list_effects` ("fumo:0", "foschia"…), tipo se ce n'è uno
/// solo, oppure nome (anche parte del nome).
pub fn find(scene: &Scene, v: &Value) -> Result<Effect, String> {
    let all = present(scene);
    let s = match v {
        Value::String(s) => s.trim().to_lowercase(),
        _ => return Err("\"effect\": scrivi l'id di list_effects (es. \"fumo:0\", \"foschia\") o il nome".into()),
    };
    if let Some(e) = all.iter().find(|e| e.id() == s) {
        return Ok(*e);
    }
    let kind = match s.as_str() {
        "haze" | "nebbia" => "foschia",
        "dust" => "polvere",
        "fireflies" => "lucciole",
        "smoke" => "fumo",
        "ghiaccio" | "dry_ice" | "ghiaccio secco" => "ghiaccio_secco",
        "decal" | "etichetta" | "logo" => "adesivo",
        other => other,
    };
    let of_kind: Vec<Effect> = all.iter().copied().filter(|e| e.kind() == kind).collect();
    match of_kind.len() {
        1 => return Ok(of_kind[0]),
        n if n > 1 => {
            return Err(format!(
                "ci sono {n} effetti di tipo {kind}: indica l'id ({})",
                of_kind.iter().map(|e| e.id()).collect::<Vec<_>>().join(", ")
            ))
        }
        _ => {}
    }
    let exact: Vec<Effect> = all.iter().copied().filter(|e| name_of(scene, *e).to_lowercase() == s).collect();
    let found = if exact.is_empty() {
        all.iter()
            .copied()
            .filter(|e| name_of(scene, *e).to_lowercase().contains(&s))
            .collect()
    } else {
        exact
    };
    match found.len() {
        1 => Ok(found[0]),
        0 => Err(format!(
            "effetto \"{s}\" non trovato; nella scena: {}",
            if all.is_empty() {
                "nessuno".to_string()
            } else {
                all.iter().map(|e| e.id()).collect::<Vec<_>>().join(", ")
            }
        )),
        _ => Err(format!(
            "\"{s}\" indica più effetti: {}",
            found.iter().map(|e| e.id()).collect::<Vec<_>>().join(", ")
        )),
    }
}

/// Aggiorna la scena dopo una modifica, come fanno i pannelli degli effetti.
fn effect_changed(app: &mut ForgeApp, e: Effect) {
    match e {
        Effect::Dust => crate::ui::atmosphere::dust_changed(app),
        Effect::Decal(_) => {
            app.scene.sync_effects();
            app.geometry_changed();
            app.appearance_changed();
        }
        _ => app.shading_changed(),
    }
}

/// Copia in `target` i campi di `values` (nomi come in `list_effects`). Campi sconosciuti e
/// valori del tipo sbagliato sono errori; la struttura non cambia se qualcosa non va.
fn merge<T: Serialize + DeserializeOwned>(target: &mut T, values: &Map<String, Value>, keep: &[&str]) -> Result<(), String> {
    let mut cur = serde_json::to_value(&*target).map_err(|e| e.to_string())?;
    let obj = cur.as_object_mut().ok_or("valori non modificabili")?;
    for (k, v) in values {
        if keep.contains(&k.as_str()) {
            return Err(format!("\"{k}\" non si cambia (è il numero dell'effetto nella scena)"));
        }
        if !obj.contains_key(k) {
            let mut names: Vec<&String> = obj.keys().filter(|n| !keep.contains(&n.as_str())).collect();
            names.sort();
            return Err(format!(
                "campo \"{k}\" sconosciuto; campi: {}",
                names.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
            ));
        }
        obj.insert(k.clone(), v.clone());
    }
    *target = serde_json::from_value(cur).map_err(|e| format!("valori non validi: {e}"))?;
    Ok(())
}

/// Applica `values`, `enabled` e `name` all'effetto.
fn apply_edit(app: &mut ForgeApp, e: Effect, a: &Args) -> Result<(), String> {
    let empty = Map::new();
    let values = match a.0.get("values") {
        None | Some(Value::Null) => &empty,
        Some(Value::Object(m)) => m,
        Some(_) => return Err("\"values\" dev'essere un oggetto {campo: valore}".into()),
    };
    let enabled = a.bool("enabled")?;
    let name = a.str("name")?;
    let s = &mut app.scene;
    match e {
        Effect::Haze => {
            merge(&mut s.haze, values, &[])?;
            if let Some(on) = enabled {
                s.haze.enabled = on;
            }
        }
        Effect::Dust => {
            merge(&mut s.haze_dust, values, &[])?;
            if let Some(on) = enabled {
                s.haze_dust.enabled = on;
            }
        }
        Effect::Fireflies => {
            merge(&mut s.fireflies, values, &[])?;
            if let Some(on) = enabled {
                s.fireflies.enabled = on;
            }
        }
        Effect::Smoke(i) => {
            let mut x = s.smokes[i].clone();
            merge(&mut x, values, &["n"])?;
            x = x.sanitized();
            if let Some(on) = enabled {
                x.enabled = on;
            }
            if let Some(n) = name {
                x.name = n;
            }
            s.smokes[i] = x;
        }
        Effect::Fog(i) => {
            let mut x = s.fogs[i].clone();
            merge(&mut x, values, &["id"])?;
            let id = x.id;
            x = x.sanitized();
            x.id = id;
            if let Some(on) = enabled {
                x.enabled = on;
            }
            if let Some(n) = name {
                x.name = n;
            }
            s.fogs[i] = x;
        }
        Effect::Decal(i) => {
            let mut x = s.decals[i].clone();
            merge(&mut x, values, &["n"])?;
            if let Some(on) = enabled {
                x.enabled = on;
            }
            if let Some(n) = name {
                x.name = n;
            }
            s.decals[i] = x;
        }
    }
    effect_changed(app, e);
    Ok(())
}

pub fn edit(app: &mut ForgeApp, a: &Args) -> Result<Value, String> {
    let e = find(&app.scene, a.0.get("effect").ok_or("manca \"effect\"")?)?;
    apply_edit(app, e, a)?;
    Ok(effect_json(&app.scene, e))
}

/// Punto di partenza: `position`, altrimenti come i pulsanti (sopra la selezione o al centro).
fn start_point(a: &Args, default: Vec3) -> Result<Vec3, String> {
    Ok(a.vec3("position")?.unwrap_or(default))
}

fn pick<'a, T>(items: &'a [T], key: impl Fn(&T) -> &str, wanted: Option<String>, what: &str) -> Result<&'a T, String> {
    let Some(w) = wanted else { return Ok(&items[0]) };
    let w = preset_key(&w);
    items
        .iter()
        .find(|p| preset_key(key(p)) == w)
        .or_else(|| items.iter().find(|p| preset_key(key(p)).contains(&w)))
        .ok_or_else(|| {
            format!(
                "{what} \"{w}\" sconosciuto: {}",
                items.iter().map(|p| preset_key(key(p))).collect::<Vec<_>>().join(", ")
            )
        })
}

pub fn add(app: &mut ForgeApp, a: &Args) -> Result<Value, String> {
    let kind = preset_key(&a.req_str("type")?);
    let preset = a.str("preset")?;
    let e = match kind.as_str() {
        "foschia" | "haze" | "nebbia" => {
            if app.scene.haze.enabled {
                return Err("la foschia c'è già (una per scena): cambiala con edit_effect \"foschia\"".into());
            }
            let w = preset.unwrap_or_else(|| "nebbia_bassa".into());
            let (_, k) = pick(&HAZE_KINDS, |(n, _)| n, Some(w), "tipo di foschia")?;
            app.scene.haze = Haze::of_kind(*k);
            Effect::Haze
        }
        "fumo" | "smoke" => {
            let p = pick(&SMOKE_PRESETS, |p| p.name, preset, "preset del fumo")?;
            let base = start_point(a, crate::ui::smoke::base_point(app))?;
            let mut s = (p.make)();
            let n = app.scene.smokes.iter().map(|s| s.n).max().unwrap_or(0) + 1;
            s.n = n;
            s.seed = n;
            s.name = tr(p.name).to_string();
            let lift = if s.shape == SmokeShape::Cloud { s.size.y * 0.25 } else { 0.0 };
            s.center = base + Vec3::new(0.0, s.size.y * 0.5 + lift, 0.0);
            app.scene.smokes.push(s);
            app.smoke_preview.just_added = Some(n);
            app.selected_smoke = Some(app.scene.smokes.len() - 1);
            app.selected_fog = None;
            app.selected_light = None;
            app.selection.clear();
            Effect::Smoke(app.scene.smokes.len() - 1)
        }
        "ghiaccio_secco" | "ghiaccio" | "dry_ice" => {
            let above = {
                let mut b = forge_math::Aabb::EMPTY;
                for &i in &app.selection {
                    b.grow(&app.scene.bounds_of(i));
                }
                (!b.is_empty()).then(|| Vec3::new(b.center().x, b.max.y + 0.03, b.center().z))
            };
            let emitter = start_point(a, above.unwrap_or(Vec3::new(0.0, 0.3, 0.0)))?;
            let n = app.scene.fogs.len() + 1;
            let mut f = FogVolume {
                name: if n == 1 {
                    tr("Ghiaccio secco").into()
                } else {
                    tf("Ghiaccio secco {n}", &[format!("{}", n)])
                },
                seed: n as u32,
                emitter,
                ..Default::default()
            };
            if preset.is_some() {
                pick(&FOG_PRESETS, |p| p.name, preset, "preset del ghiaccio secco")?.apply(&mut f);
            }
            app.scene.fogs.push(f);
            app.selected_fog = Some(app.scene.fogs.len() - 1);
            app.selected_light = None;
            Effect::Fog(app.scene.fogs.len() - 1)
        }
        "polvere" | "dust" | "polvere_in_sospensione" => {
            if app.scene.haze_dust.enabled {
                return Err("la polvere c'è già (una per scena): cambiala con edit_effect \"polvere\"".into());
            }
            app.scene.haze_dust.enabled = true;
            app.scene.haze_dust.region = Some(app.scene.default_dust_region());
            app.selected_dust = true;
            Effect::Dust
        }
        "lucciole" | "fireflies" | "bosco_incantato" => {
            if app.scene.fireflies.enabled {
                return Err("le lucciole ci sono già: cambiale con edit_effect \"lucciole\"".into());
            }
            let (center, size) = crate::ui::fireflies::framed_volume(app);
            let center = a.vec3("position")?.unwrap_or(center);
            app.scene.fireflies = forge_scene::Fireflies::enchanted_forest(center, size);
            Effect::Fireflies
        }
        "adesivo" | "decal" | "etichetta" | "logo" => {
            let image = std::path::PathBuf::from(a.req_str("image_path")?);
            if !image.is_file() {
                return Err(format!("immagine non trovata: {}", image.display()));
            }
            let selection: Vec<usize> = app.selection.iter().copied().collect();
            let among = ops::mesh_nodes_under(&app.scene, &ops::resolve_targets(&app.scene, a.0.get("targets"), &selection)?);
            if among.is_empty() {
                return Err("nessun pezzo con geometria tra i componenti indicati".into());
            }
            let i = crate::ui::atmosphere::add_decal_on(app, &among, image).map_err(|e| e.to_string())?;
            Effect::Decal(i)
        }
        other => {
            return Err(format!(
                "tipo di effetto \"{other}\": foschia, fumo, ghiaccio_secco, polvere, lucciole o adesivo"
            ))
        }
    };
    if a.0.get("values").is_some() || a.has("enabled") || a.has("name") {
        apply_edit(app, e, a)?;
    } else {
        effect_changed(app, e);
    }
    Ok(effect_json(&app.scene, e))
}

pub fn remove(app: &mut ForgeApp, a: &Args) -> Result<Value, String> {
    let e = find(&app.scene, a.0.get("effect").ok_or("manca \"effect\"")?)?;
    let name = name_of(&app.scene, e);
    match e {
        Effect::Haze => {
            app.scene.haze.enabled = false;
            app.shading_changed();
        }
        Effect::Dust => {
            app.scene.haze_dust.enabled = false;
            app.selected_dust = false;
            crate::ui::atmosphere::dust_changed(app);
        }
        Effect::Fireflies => {
            app.scene.fireflies.enabled = false;
            app.shading_changed();
        }
        Effect::Smoke(i) => {
            app.scene.smokes.remove(i);
            app.selected_smoke = None;
            app.shading_changed();
        }
        Effect::Fog(i) => {
            let gone = app.scene.fogs.remove(i);
            app.scene.timeline.remove_element(&forge_scene::Target::Fog(gone.id));
            app.selected_fog = None;
            app.shading_changed();
        }
        Effect::Decal(i) => {
            app.scene.decals.remove(i);
            app.selection.clear();
            effect_changed(app, e);
        }
    }
    Ok(json!({"eliminato": name, "effetti": summary(&app.scene)}))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La descrizione di add_effect per l'assistente elenca proprio i preset del programma.
    #[test]
    fn add_effect_description_lists_the_real_presets() {
        let tools = forge_mcp::tools::list();
        let add = tools.as_array().unwrap().iter().find(|t| t["name"] == "add_effect").unwrap();
        let text = add["description"].as_str().unwrap();
        let keys = SMOKE_PRESETS
            .iter()
            .map(|p| preset_key(p.name))
            .chain(FOG_PRESETS.iter().map(|p| preset_key(p.name)))
            .chain(HAZE_KINDS.iter().map(|(n, _)| n.to_string()));
        for k in keys {
            assert!(text.contains(&k), "manca {k}");
        }
    }

    #[test]
    fn preset_keys_are_simple() {
        assert_eq!(preset_key("Beuta: nuvola che sale"), "beuta_nuvola_che_sale");
        assert_eq!(preset_key("Vapore da tazza"), "vapore_da_tazza");
        assert_eq!(preset_key("  Incenso  "), "incenso");
    }

    /// Modifica per nome di campo: cambia solo quello, rifiuta campi sconosciuti, valori del tipo
    /// sbagliato e il numero dell'effetto, lasciando la struttura com'era.
    #[test]
    fn merge_changes_only_known_fields() {
        let mut h = Haze::of_kind(HazeKind::Layers);
        let before = h.clone();
        let m = |v: Value| v.as_object().unwrap().clone();
        merge(&mut h, &m(json!({"density": 0.9, "color": [1.0, 0.5, 0.2]})), &[]).unwrap();
        assert_eq!((h.density, h.color, h.kind), (0.9, [1.0, 0.5, 0.2], HazeKind::Layers));
        let after = h.clone();
        assert!(merge(&mut h, &m(json!({"densita": 1.0})), &[]).unwrap_err().contains("density"));
        assert!(merge(&mut h, &m(json!({"density": "tanta"})), &[]).is_err());
        assert_eq!(h, after, "niente cambia se un valore non va");
        let mut f = FogVolume::default();
        assert!(merge(&mut f, &m(json!({"id": 7})), &["id"]).is_err());
        assert_ne!(before, after);
    }

    /// Gli id di `list_effects`, i tipi con un solo effetto e i nomi trovano l'effetto giusto.
    #[test]
    fn effects_are_found_by_id_kind_and_name() {
        let mut s = Scene {
            haze: Haze::of_kind(HazeKind::Ambient),
            ..Default::default()
        };
        s.fogs.push(FogVolume {
            name: "Calderone".into(),
            ..Default::default()
        });
        s.fogs.push(FogVolume {
            name: "Bicchiere".into(),
            ..Default::default()
        });
        let f = |q: &str| find(&s, &json!(q));
        assert_eq!(f("foschia"), Ok(Effect::Haze));
        assert_eq!(f("ghiaccio_secco:1"), Ok(Effect::Fog(1)));
        assert_eq!(f("calder"), Ok(Effect::Fog(0)));
        assert!(f("ghiaccio_secco").unwrap_err().contains("2 effetti"));
        assert!(f("lucciole").is_err(), "non ci sono");
        assert_eq!(present(&s).len(), 3);
        assert_eq!(summary(&s).as_array().unwrap().len(), 3);
    }
}
