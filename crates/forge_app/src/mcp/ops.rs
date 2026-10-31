//! Operazioni sulla scena usate dagli strumenti MCP, senza finestra: argomenti, posizione
//! delle luci rispetto alla camera, ricerca di luci, componenti e materiali, stato della scena.

use forge_library::store::LibraryMaterial;
use forge_math::Vec3;
use forge_scene::{Camera, Environment, Light, LightKind, Material, MaterialId, Scene};
use serde_json::{json, Value};

/// Argomenti di uno strumento (oggetto JSON), con messaggi d'errore leggibili.
pub struct Args<'a>(pub &'a Value);

impl Args<'_> {
    pub fn has(&self, k: &str) -> bool {
        self.0.get(k).is_some_and(|v| !v.is_null())
    }

    pub fn str(&self, k: &str) -> Result<Option<String>, String> {
        match self.0.get(k) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(s)) => Ok(Some(s.trim().to_string())),
            Some(v) => Ok(Some(v.to_string())),
        }
    }

    pub fn req_str(&self, k: &str) -> Result<String, String> {
        self.str(k)?
            .filter(|s| !s.is_empty())
            .ok_or_else(|| format!("manca l'argomento \"{k}\""))
    }

    pub fn f32(&self, k: &str) -> Result<Option<f32>, String> {
        match self.0.get(k) {
            None | Some(Value::Null) => Ok(None),
            Some(v) => v
                .as_f64()
                .or_else(|| v.as_str().and_then(|s| s.trim().replace(',', ".").parse().ok()))
                .filter(|x| x.is_finite())
                .map(|x| Some(x as f32))
                .ok_or_else(|| format!("\"{k}\" deve essere un numero")),
        }
    }

    pub fn u32(&self, k: &str) -> Result<Option<u32>, String> {
        Ok(self.f32(k)?.map(|x| x.round().max(0.0) as u32))
    }

    pub fn bool(&self, k: &str) -> Result<Option<bool>, String> {
        match self.0.get(k) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Bool(b)) => Ok(Some(*b)),
            Some(Value::String(s)) if matches!(s.as_str(), "true" | "si" | "sì" | "yes") => Ok(Some(true)),
            Some(Value::String(s)) if matches!(s.as_str(), "false" | "no") => Ok(Some(false)),
            Some(_) => Err(format!("\"{k}\" deve essere true o false")),
        }
    }

    pub fn vec3(&self, k: &str) -> Result<Option<Vec3>, String> {
        match self.0.get(k) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Array(a)) if a.len() == 3 => {
                let mut v = [0.0f32; 3];
                for (i, x) in a.iter().enumerate() {
                    v[i] = x
                        .as_f64()
                        .filter(|x| x.is_finite())
                        .ok_or_else(|| format!("\"{k}\": servono 3 numeri"))? as f32;
                }
                Ok(Some(Vec3::from_array(v)))
            }
            Some(_) => Err(format!("\"{k}\" deve essere [x, y, z] in metri")),
        }
    }

    /// Colore lineare da "#rrggbb" (sRGB, come nei programmi di grafica) o da [r, g, b] 0–1.
    pub fn color(&self, k: &str) -> Result<Option<[f32; 3]>, String> {
        match self.0.get(k) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(s)) => parse_hex(s).map(Some).ok_or_else(|| format!("\"{k}\": colore come \"#rrggbb\"")),
            Some(Value::Array(a)) if a.len() == 3 => {
                let mut c = [0.0f32; 3];
                for (i, x) in a.iter().enumerate() {
                    c[i] = x.as_f64().ok_or_else(|| format!("\"{k}\": servono 3 numeri 0–1"))?.clamp(0.0, 1.0) as f32;
                }
                Ok(Some(c))
            }
            Some(_) => Err(format!("\"{k}\": colore come \"#rrggbb\" o [r, g, b] da 0 a 1")),
        }
    }
}

/// "#rrggbb" sRGB → colore lineare.
pub fn parse_hex(s: &str) -> Option<[f32; 3]> {
    let h = s.trim().trim_start_matches('#');
    if h.len() != 6 || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let ch = |i: usize| {
        u8::from_str_radix(&h[i..i + 2], 16)
            .ok()
            .map(|v| forge_scene::material::srgb_to_linear(v as f32 / 255.0))
    };
    Some([ch(0)?, ch(2)?, ch(4)?])
}

/// Colore lineare → "#rrggbb" sRGB (per le risposte).
pub fn to_hex(c: [f32; 3]) -> String {
    let b = c.map(|v| (forge_scene::material::linear_to_srgb(v).clamp(0.0, 1.0) * 255.0).round() as u8);
    format!("#{:02x}{:02x}{:02x}", b[0], b[1], b[2])
}

pub use forge_mcp::tools::SIDES;

/// Direzione (dal centro del prodotto) di un lato, con la base della camera: "destra" è la
/// destra dell'inquadratura, "davanti" verso la camera. Accetta anche i nomi inglesi.
pub fn side_direction(cam: &Camera, side: &str) -> Option<Vec3> {
    let (r, _, f) = cam.basis();
    // Componenti orizzontali: le luci non seguono l'inclinazione della camera.
    let flat = |v: Vec3| Vec3::new(v.x, 0.0, v.z).normalize_or(Vec3::X);
    let right = flat(r);
    let front = -flat(f);
    let up = Vec3::Y;
    let s = side.trim().to_lowercase().replace([' ', '-'], "_");
    let d = match s.as_str() {
        "destra" | "right" => right + front * 0.5 + up * 0.6,
        "sinistra" | "left" => -right + front * 0.5 + up * 0.6,
        "alto" | "sopra" | "top" | "above" => up + front * 0.2,
        "davanti" | "fronte" | "front" => front + up * 0.4,
        "dietro" | "back" | "behind" => -front + up * 0.6,
        "controluce" | "rim" | "backlight" => -front * 0.8 + up * 0.9 + right * 0.3,
        "davanti_destra" | "front_right" => right + front + up * 0.6,
        "davanti_sinistra" | "front_left" => -right + front + up * 0.6,
        "dietro_destra" | "back_right" => right - front + up * 0.6,
        "dietro_sinistra" | "back_left" => -right - front + up * 0.6,
        "basso" | "sotto" | "bottom" | "below" => front * 0.6 - up * 0.3,
        _ => return None,
    };
    Some(d.normalize())
}

/// Posizione di una luce sul lato `side` del prodotto (centro `c`, raggio `radius`).
pub fn light_position(cam: &Camera, c: Vec3, radius: f32, side: &str) -> Result<Vec3, String> {
    let d = side_direction(cam, side).ok_or_else(|| format!("lato \"{side}\" sconosciuto: usa uno di {}", SIDES.join(", ")))?;
    Ok(c + d * (2.5 * radius).max(0.3))
}

/// Indice della luce: numero (da 0) o nome (uguale, poi contenuto, senza maiuscole).
pub fn find_light(scene: &Scene, v: &Value) -> Result<usize, String> {
    if let Some(i) = v.as_u64() {
        return (i < scene.lights.len() as u64)
            .then_some(i as usize)
            .ok_or_else(|| format!("luce {i} inesistente: ci sono {} luci (da 0)", scene.lights.len()));
    }
    let q = v
        .as_str()
        .ok_or("indica la luce con il numero (da 0) o il nome")?
        .trim()
        .to_lowercase();
    if let Ok(i) = q.parse::<usize>() {
        return find_light(scene, &json!(i));
    }
    let names: Vec<String> = scene.lights.iter().map(|l| l.name.to_lowercase()).collect();
    names
        .iter()
        .position(|n| *n == q)
        .or_else(|| names.iter().position(|n| n.contains(&q)))
        .ok_or_else(|| {
            format!(
                "nessuna luce \"{q}\"; luci: {}",
                scene.lights.iter().map(|l| l.name.as_str()).collect::<Vec<_>>().join(", ")
            )
        })
}

/// Componenti indicati da una lista di nomi/id oppure dalle parole "prodotto", "tutto",
/// "selezione". Nomi: uguali o contenuti, senza maiuscole.
pub fn resolve_targets(scene: &Scene, v: Option<&Value>, selection: &[usize]) -> Result<Vec<usize>, String> {
    let items: Vec<String> = match v {
        None | Some(Value::Null) => vec!["prodotto".into()],
        Some(Value::String(s)) => vec![s.clone()],
        Some(Value::Array(a)) => a
            .iter()
            .map(|x| x.as_str().map(str::to_string).unwrap_or_else(|| x.to_string()))
            .collect(),
        Some(_) => return Err("\"targets\": un nome o una lista di nomi di componenti".into()),
    };
    let mut out = Vec::new();
    for item in items {
        let q = item.trim().to_lowercase();
        let found: Vec<usize> = match q.as_str() {
            "prodotto" | "product" => crate::presets::product_roots(scene),
            "tutto" | "all" | "tutti" => (0..scene.nodes.len())
                .filter(|&i| scene.nodes[i].parent.is_none() && !scene.nodes[i].removed)
                .collect(),
            "selezione" | "selection" | "selezionati" => selection.to_vec(),
            _ => {
                let live = |i: &usize| !scene.nodes[*i].removed;
                let exact: Vec<usize> = (0..scene.nodes.len())
                    .filter(live)
                    .filter(|&i| scene.nodes[i].id.0.to_lowercase() == q || scene.nodes[i].name.to_lowercase() == q)
                    .collect();
                if exact.is_empty() {
                    (0..scene.nodes.len())
                        .filter(live)
                        .filter(|&i| scene.nodes[i].name.to_lowercase().contains(&q))
                        .collect()
                } else {
                    exact
                }
            }
        };
        if found.is_empty() {
            return Err(format!("nessun componente \"{item}\" (usa scene_info per i nomi)"));
        }
        out.extend(found);
    }
    out.sort_unstable();
    out.dedup();
    Ok(out)
}

/// Nodi con una mesh dentro i sottoalberi indicati (radici comprese).
pub fn mesh_nodes_under(scene: &Scene, roots: &[usize]) -> Vec<usize> {
    let mut out = Vec::new();
    let mut stack = roots.to_vec();
    while let Some(i) = stack.pop() {
        if scene.nodes[i].mesh.is_some() {
            out.push(i);
        }
        stack.extend(&scene.nodes[i].children);
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// Dove sta un materiale trovato.
pub enum FoundMaterial {
    Project(MaterialId),
    Library(Box<Material>),
}

/// Cerca un materiale per id o nome: prima nel progetto (compresi i ~120 pronti), poi nella
/// libreria su disco. Uguale prima, poi contenuto (tutte le parole), senza maiuscole.
pub fn find_material(scene: &Scene, library: &[LibraryMaterial], query: &str) -> Result<FoundMaterial, String> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return Err("indica il materiale (id o nome)".into());
    }
    let proj = &scene.materials.materials;
    if let Some(m) = proj.iter().find(|m| m.id.0.to_lowercase() == q || m.name.to_lowercase() == q) {
        return Ok(FoundMaterial::Project(m.id.clone()));
    }
    if let Some(m) = library
        .iter()
        .find(|m| m.material.id.0.to_lowercase() == q || m.material.name.to_lowercase() == q)
    {
        return Ok(FoundMaterial::Library(Box::new(m.material.clone())));
    }
    let words: Vec<&str> = q.split_whitespace().collect();
    let hit = |name: &str, cat: &str| {
        let t = format!("{} {}", name.to_lowercase(), cat.to_lowercase());
        words.iter().all(|w| t.contains(w))
    };
    if let Some(m) = proj.iter().find(|m| hit(&m.name, &m.category)) {
        return Ok(FoundMaterial::Project(m.id.clone()));
    }
    if let Some(m) = library.iter().find(|m| hit(&m.material.name, &m.categories.join(" "))) {
        return Ok(FoundMaterial::Library(Box::new(m.material.clone())));
    }
    Err(format!(
        "nessun materiale \"{query}\": cercalo con list_materials, oppure crealo con create_material"
    ))
}

/// Id per un materiale creato dall'assistente: "ia-<nome>".
pub fn material_id_for(name: &str) -> MaterialId {
    let slug: String = name
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    MaterialId::new(format!("ia-{}", if slug.is_empty() { "materiale".into() } else { slug }))
}

fn r3(v: Vec3) -> [f32; 3] {
    v.to_array().map(|x| (x * 1000.0).round() / 1000.0)
}

pub fn light_json(i: usize, l: &Light) -> Value {
    let mut v = json!({
        "indice": i,
        "nome": l.name,
        "tipo": match l.kind {
            LightKind::Point => "punto",
            LightKind::Spot { .. } => "spot",
            LightKind::Directional => "direzionale",
            LightKind::Area { .. } => "area",
            LightKind::Portal { .. } => "portale",
        },
        "posizione_m": r3(l.position),
        "bersaglio_m": r3(l.target),
        "colore": to_hex(l.color),
        "intensita": l.intensity,
        "accesa": l.enabled,
        "ombre": l.cast_shadows,
    });
    match l.kind {
        LightKind::Area { width, height } | LightKind::Portal { width, height } => {
            v["dimensioni_m"] = json!([width, height]);
        }
        LightKind::Spot {
            outer_angle_deg,
            inner_angle_deg,
        } => {
            v["cono_gradi"] = json!(outer_angle_deg);
            v["sfumatura_cono_pct"] = json!(if outer_angle_deg > 0.0 {
                (1.0 - inner_angle_deg / outer_angle_deg) * 100.0
            } else {
                0.0
            });
            if l.beam_density > 0.0 {
                v["fascio_densita"] = json!(l.beam_density);
                v["fascio_lunghezza_m"] = json!(beam_len(l));
            }
        }
        _ => {}
    }
    if l.has_range() && l.range > 0.0 {
        v["raggio_azione_m"] = json!(l.range);
        v["sfumatura_raggio"] = json!(l.range_softness);
    }
    v
}

/// Stato della scena per l'assistente: prodotto, componenti, luci, camera, ambiente.
pub fn scene_json(scene: &Scene, max_components: usize) -> Value {
    let b = crate::presets::product_bounds(scene);
    let product = if b.is_empty() {
        Value::Null
    } else {
        json!({
            "centro_m": r3(b.center()),
            "dimensioni_m": r3(b.max - b.min),
            "base_y_m": (b.min.y * 1000.0).round() / 1000.0,
        })
    };
    let live: Vec<usize> = (0..scene.nodes.len()).filter(|&i| !scene.nodes[i].removed).collect();
    let components: Vec<Value> = live
        .iter()
        .take(max_components)
        .map(|&i| {
            let n = &scene.nodes[i];
            let m = scene.resolve_material(i);
            json!({
                "nome": n.name,
                "id": n.id.0,
                "padre": n.parent.map(|p| scene.nodes[p].name.clone()),
                "mesh": n.mesh.is_some(),
                "visibile": scene.is_visible(i),
                "scenografia": crate::presets::is_scenery_node(scene, i),
                "materiale": scene.materials.get(m).map(|x| x.name.clone()).unwrap_or_else(|| m.0.clone()),
            })
        })
        .collect();
    let cam = scene.render_camera();
    let env = match &scene.environment {
        Environment::Uniform { color, strength } => json!({"tipo": "uniforme", "colore": to_hex(*color), "intensita": strength}),
        Environment::Gradient { strength, .. } => json!({"tipo": "gradiente", "intensita": strength}),
        Environment::Hdri {
            path,
            strength,
            rotation_deg,
        } => {
            json!({"tipo": "hdri", "file": path.display().to_string(), "intensita": strength, "rotazione_gradi": rotation_deg})
        }
    };
    json!({
        "unita": "metri; y verso l'alto",
        "prodotto": product,
        "componenti_totali": live.len(),
        "componenti": components,
        "luci": scene.lights.iter().enumerate().map(|(i, l)| light_json(i, l)).collect::<Vec<_>>(),
        "camera_render": {
            "posizione_m": r3(cam.position),
            "bersaglio_m": r3(cam.target),
            "focale_mm": (cam.focal_mm() * 10.0).round() / 10.0,
            "ortografica": cam.lens.ortho,
        },
        "ambiente": env,
    })
}

/// Lunghezza del fascio visibile di una spot (come la usa il render).
pub fn beam_len(l: &Light) -> f32 {
    forge_render_core::fog::beam_length(l)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn front_camera() -> Camera {
        Camera {
            position: Vec3::new(0.0, 0.5, 4.0),
            target: Vec3::new(0.0, 0.5, 0.0),
            up: Vec3::Y,
            ..Default::default()
        }
    }

    #[test]
    fn sides_follow_the_camera() {
        let cam = front_camera();
        let r = side_direction(&cam, "destra").unwrap();
        assert!(r.x > 0.5 && r.y > 0.0, "{r:?}");
        let l = side_direction(&cam, "left").unwrap();
        assert!(l.x < -0.5);
        assert!(side_direction(&cam, "davanti").unwrap().z > 0.5, "verso la camera");
        assert!(side_direction(&cam, "controluce").unwrap().z < 0.0, "dietro il prodotto");
        assert!(side_direction(&cam, "Alto").unwrap().y > 0.9);
        // Camera girata di 90° (guarda verso -X da +X): la destra diventa -Z.
        let side = Camera {
            position: Vec3::new(4.0, 0.5, 0.0),
            ..front_camera()
        };
        assert!(side_direction(&side, "destra").unwrap().z < -0.5);
        assert!(light_position(&cam, Vec3::ZERO, 1.0, "di lato").is_err());
        let p = light_position(&cam, Vec3::ZERO, 0.01, "destra").unwrap();
        assert!((p.length() - 0.3).abs() < 1e-4, "distanza minima 30 cm: {p:?}");
    }

    #[test]
    fn colors_and_arguments() {
        let c = parse_hex("#ff8000").unwrap();
        assert!((c[0] - 1.0).abs() < 1e-5 && c[2] == 0.0 && c[1] > 0.2 && c[1] < 0.25);
        assert_eq!(to_hex(c), "#ff8000");
        assert!(parse_hex("rosso").is_none());
        let v = json!({"n": "2,5", "v": [1, 2, 3], "c": [0.5, 0.5, 2.0], "b": "sì"});
        let a = Args(&v);
        assert_eq!(a.f32("n").unwrap(), Some(2.5));
        assert_eq!(a.vec3("v").unwrap(), Some(Vec3::new(1.0, 2.0, 3.0)));
        assert_eq!(a.color("c").unwrap(), Some([0.5, 0.5, 1.0]));
        assert_eq!(a.bool("b").unwrap(), Some(true));
        assert!(a.req_str("assente").is_err());
        assert!(Args(&json!({"v": [1, 2]})).vec3("v").is_err());
    }

    #[test]
    fn finds_lights_components_and_materials() {
        let s = Scene::test_scene();
        assert_eq!(find_light(&s, &json!(0)).unwrap(), 0);
        let name = s.lights[1].name.clone();
        assert_eq!(find_light(&s, &json!(name.to_uppercase())).unwrap(), 1);
        assert!(find_light(&s, &json!(99)).is_err());
        let t = resolve_targets(&s, Some(&json!(["sfera vetro"])), &[]).unwrap();
        assert_eq!(t.len(), 1);
        assert_eq!(s.nodes[t[0]].name, "Sfera vetro");
        assert!(!resolve_targets(&s, None, &[]).unwrap().is_empty(), "prodotto");
        assert!(resolve_targets(&s, Some(&json!("inesistente")), &[]).is_err());
        assert!(!mesh_nodes_under(&s, &resolve_targets(&s, None, &[]).unwrap()).is_empty());
        // Materiali pronti: per id, per nome, per parole.
        let first = &s.materials.materials[1];
        assert!(matches!(find_material(&s, &[], &first.id.0).unwrap(), FoundMaterial::Project(id) if id == first.id));
        assert!(matches!(find_material(&s, &[], "acciaio").unwrap(), FoundMaterial::Project(_)));
        assert!(find_material(&s, &[], "zzz nessuno").is_err());
        assert_eq!(material_id_for("Metallo satinato!").0, "ia-metallo-satinato");
    }

    #[test]
    fn scene_json_describes_product_lights_and_camera() {
        let s = crate::presets::studio_demo_scene();
        let v = scene_json(&s, 100);
        assert!(v["prodotto"]["dimensioni_m"].is_array());
        assert_eq!(v["luci"].as_array().unwrap().len(), s.lights.len());
        assert_eq!(v["luci"][0]["tipo"], "area");
        assert!(v["componenti"].as_array().unwrap().iter().any(|c| c["scenografia"] == true));
        assert!(v["camera_render"]["focale_mm"].as_f64().unwrap() > 10.0);
    }
}
