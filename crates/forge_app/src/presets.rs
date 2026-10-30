//! Preset della modalità Semplice: aspetto della foto prodotto, qualità e "Prepara scena".
//!
//! I preset scrivono negli stessi dati che usa il renderer (luci e ambiente della scena,
//! fondale, `RenderSettings`): la modalità Semplice e quella Avanzata sono due viste sugli
//! stessi valori, quindi passare dall'una all'altra non perde niente.
//!
//! Applicare un preset salva prima i valori che sta per cambiare ([`PresetBackup`]):
//! "Annulla preset" li ripristina, e solo quelli (le altre modifiche fatte nel frattempo
//! restano). I valori scelti sono descritti in `docs/INTERFACCIA_SEMPLICE.md`.

use crate::i18n::{tk, tr};
use forge_math::{Aabb, Mat4, Vec3};
use forge_render_core::{DenoiseMode, DenoiseQuality, RenderSettings, ToneMapping};
use forge_scene::{AddedKind, AddedRecord, ComponentId, Environment, Light, Material, MaterialId, Scene, UserTransform};

/// Aspetto della foto prodotto.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Look {
    /// Catalogo: fondale bianco, luce morbida e avvolgente.
    Light,
    /// Fondale scuro lucido, tre punti luce: il prodotto stacca dal nero.
    Dark,
    /// Luce calda di taglio e controluce fredda, ambiente scuro.
    Dramatic,
    /// Fondale fotografico in tela inclinato e set da studio con quattro luci.
    Studio,
}

impl Look {
    pub const ALL: [Look; 4] = [Look::Studio, Look::Light, Look::Dark, Look::Dramatic];

    pub fn label(self) -> &'static str {
        match self {
            Look::Light => tr("Sfondo chiaro"),
            Look::Dark => tr("Sfondo scuro"),
            Look::Dramatic => tr("Studio scenografico"),
            Look::Studio => tr("Studio fotografico"),
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Look::Light => tr("Fondale bianco e luce morbida: foto da catalogo o e-commerce"),
            Look::Dark => tr("Fondale scuro lucido e tre punti luce: il prodotto stacca dal nero"),
            Look::Dramatic => tr("Luce calda di taglio e controluce fredda: foto d'effetto"),
            Look::Studio => tr("Fondale in tela inclinato e quattro luci da studio: come in sala posa"),
        }
    }

    /// Valori impostati dal preset.
    pub fn spec(self) -> LookSpec {
        match self {
            Look::Studio => LookSpec {
                lights: "studio-fotografico",
                environment: Environment::Uniform {
                    color: [1.0, 1.0, 1.0],
                    strength: 0.12,
                },
                environment_visible: true,
                backdrop: BackdropStyle::Sweep,
                backdrop_color: [0.80, 0.80, 0.79],
                backdrop_roughness: 0.95,
                tone_mapping: ToneMapping::AcesFilmic,
                exposure_ev: 0.0,
            },
            Look::Light => LookSpec {
                lights: "studio",
                environment: Environment::Uniform {
                    color: [1.0, 1.0, 1.0],
                    strength: 0.9,
                },
                environment_visible: true,
                backdrop: BackdropStyle::Curved,
                backdrop_color: [0.85, 0.85, 0.85],
                backdrop_roughness: 0.65,
                tone_mapping: ToneMapping::AcesFilmic,
                exposure_ev: 0.0,
            },
            Look::Dark => LookSpec {
                lights: "tre-punti",
                environment: Environment::Uniform {
                    color: [1.0, 1.0, 1.0],
                    strength: 0.0,
                },
                environment_visible: true,
                backdrop: BackdropStyle::Curved,
                backdrop_color: [0.03, 0.03, 0.03],
                backdrop_roughness: 0.35,
                tone_mapping: ToneMapping::AcesFilmic,
                exposure_ev: 0.0,
            },
            Look::Dramatic => LookSpec {
                lights: "cinematografica",
                environment: Environment::Gradient {
                    zenith: [0.02, 0.025, 0.035],
                    horizon: [0.05, 0.05, 0.06],
                    ground: [0.01, 0.01, 0.01],
                    strength: 0.5,
                },
                environment_visible: true,
                backdrop: BackdropStyle::Curved,
                backdrop_color: [0.05, 0.045, 0.04],
                backdrop_roughness: 0.3,
                tone_mapping: ToneMapping::AcesFilmic,
                exposure_ev: 0.3,
            },
        }
    }
}

/// Tipo di fondale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackdropStyle {
    /// Fondale curvo con parete verticale (Aggiungi → Fondale da studio).
    Curved,
    /// Fondale fotografico in tela, inclinato all'indietro (Aggiungi → Fondale fotografico).
    Sweep,
}

/// Cosa imposta un [`Look`].
#[derive(Clone, Debug, PartialEq)]
pub struct LookSpec {
    /// Preset di luci di `forge_scene::light::preset`, dimensionato sul prodotto.
    pub lights: &'static str,
    pub environment: Environment,
    pub environment_visible: bool,
    /// Fondale che "Prepara scena" mette (o sostituisce, se ce n'è uno dell'altro tipo).
    pub backdrop: BackdropStyle,
    pub backdrop_color: [f32; 3],
    pub backdrop_roughness: f32,
    pub tone_mapping: ToneMapping,
    pub exposure_ev: f32,
}

/// Qualità del render.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quality {
    Draft,
    Balanced,
    High,
}

impl Quality {
    pub const ALL: [Quality; 3] = [Quality::Draft, Quality::Balanced, Quality::High];

    pub fn label(self) -> &'static str {
        match self {
            Quality::Draft => tr("Bozza"),
            Quality::Balanced => tr("Bilanciata"),
            Quality::High => tr("Alta"),
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Quality::Draft => tr("Veloce, per provare inquadratura e materiali"),
            Quality::Balanced => tr("Buona per la maggior parte delle foto"),
            Quality::High => tr("Per la foto finale: più lenta, vetri e riflessi più puliti"),
        }
    }

    /// (campioni per pixel, rimbalzi massimi). Il denoiser resta sull'immagine finale.
    pub fn values(self) -> (u32, u32) {
        match self {
            Quality::Draft => (64, 8),
            Quality::Balanced => (256, 12),
            Quality::High => (1024, 16),
        }
    }

    pub fn apply(self, s: &mut RenderSettings) {
        let (spp, bounces) = self.values();
        s.target_samples = spp;
        s.max_bounces = bounces;
        s.denoise.mode = DenoiseMode::Progressive;
    }

    /// Qualità corrispondente alle impostazioni (se coincidono con un preset).
    pub fn matching(s: &RenderSettings) -> Option<Quality> {
        Quality::ALL.into_iter().find(|q| {
            let (spp, b) = q.values();
            s.target_samples == spp && s.max_bounces == b && s.denoise.mode != DenoiseMode::Off
        })
    }
}

/// Formati d'immagine proposti nella modalità Semplice.
pub const FORMATS: [(&str, &str, u32, u32); 4] = [
    (tk("Orizzontale"), tk("16:9, 1920 × 1080"), 1920, 1080),
    (tk("Quadrato"), tk("1:1, 1080 × 1080 (Instagram)"), 1080, 1080),
    (tk("Verticale"), tk("4:5, 1080 × 1350 (Instagram)"), 1080, 1350),
    (tk("Storie"), tk("9:16, 1080 × 1920"), 1080, 1920),
];

/// Id del materiale del fondale creato dai preset.
pub const BACKDROP_MATERIAL: &str = "preset-fondale";

/// Oggetti di scenografia (non fanno parte del prodotto).
fn is_scenery(kind: &AddedKind) -> bool {
    matches!(
        kind,
        AddedKind::Floor { .. } | AddedKind::Backdrop { .. } | AddedKind::StudioSweep { .. } | AddedKind::Wall { .. }
    )
}

fn scenery_ids(scene: &Scene) -> Vec<ComponentId> {
    scene.added.iter().filter(|r| is_scenery(&r.kind)).map(|r| r.id.clone()).collect()
}

/// Il nodo appartiene alla scenografia (pavimento, fondale, parete aggiunti)?
pub fn is_scenery_node(scene: &Scene, mut i: usize) -> bool {
    while let Some(p) = scene.nodes[i].parent {
        i = p;
    }
    scenery_ids(scene).contains(&scene.nodes[i].id)
}

/// Radici del prodotto: nodi di primo livello che non sono scenografia (pavimento, fondale, parete).
pub fn product_roots(scene: &Scene) -> Vec<usize> {
    let scenery = scenery_ids(scene);
    scene
        .roots
        .iter()
        .copied()
        .filter(|&r| !scenery.contains(&scene.nodes[r].id) && !scene.nodes[r].removed)
        .collect()
}

/// Ingombro del prodotto visibile (senza pavimento, fondale e pareti).
pub fn product_bounds(scene: &Scene) -> Aabb {
    let scenery = scenery_ids(scene);
    let mut b = Aabb::EMPTY;
    for (i, n) in scene.nodes.iter().enumerate() {
        let Some(m) = n.mesh else { continue };
        // Si sale fino alla radice per sapere se il nodo è scenografia.
        let mut r = i;
        while let Some(p) = scene.nodes[r].parent {
            r = p;
        }
        if scenery.contains(&scene.nodes[r].id) || !scene.is_visible(i) {
            continue;
        }
        b.grow(&scene.meshes[m].bounds().transformed(&scene.world_transform(i)));
    }
    b
}

/// Fondale dimensionato e posizionato dietro un prodotto con questo ingombro (stessa regola
/// dei menu Aggiungi → Fondale da studio / Fondale fotografico).
pub fn backdrop_for(b: &Aabb, style: BackdropStyle) -> (AddedKind, Mat4) {
    if style == BackdropStyle::Sweep {
        return sweep_for(b);
    }
    let (c, ground, e, back) = if b.is_empty() {
        (Vec3::ZERO, 0.0, 1.0, -0.5)
    } else {
        let s = b.max - b.min;
        ((b.min + b.max) * 0.5, b.min.y, s.x.max(s.z).max(0.5), b.min.z)
    };
    // Largo e alto abbastanza da riempire anche le viste ruotate (tre quarti, isometrica) e quelle
    // dall'alto: con 3 volte il prodotto i bordi del fondale entravano nell'immagine. Il raccordo
    // resta quello di un fondale alto 2,5 volte il prodotto.
    let height = (3.5 * e).max(3.5);
    let radius = 0.25 * (2.5 * e).max(2.5);
    // La parete del fondale sta a z = -raggio rispetto all'origine della mesh.
    let wall = back - 0.3 * e + radius;
    // Il piano si estende davanti alla scena quanto basta a uscire dall'inquadratura.
    let depth = (c.z - wall) + 2.0 * e;
    (
        AddedKind::Backdrop {
            width: (12.0 * e).max(10.0),
            depth,
            height,
            radius,
        },
        Mat4::from_translation(Vec3::new(c.x, ground, wall)),
    )
}

/// Inclinazione all'indietro del telo del fondale fotografico (gradi dalla verticale).
pub const SWEEP_TILT_DEG: f32 = 10.0;

/// Fondale fotografico in tela per un prodotto con questo ingombro: raccordo ampio (il telo
/// "cade" morbido) e telo inclinato all'indietro, largo abbastanza per uscire dall'inquadratura.
fn sweep_for(b: &Aabb) -> (AddedKind, Mat4) {
    let (c, ground, e, back) = if b.is_empty() {
        (Vec3::ZERO, 0.0, 1.0, -0.5)
    } else {
        let s = b.max - b.min;
        ((b.min + b.max) * 0.5, b.min.y, s.x.max(s.z).max(0.5), b.min.z)
    };
    // Largo e alto abbastanza da riempire anche le viste ruotate a 45° e quelle dall'alto (a 7 volte
    // il prodotto se ne vedeva il bordo). Il raccordo resta quello di un telo alto 2,5 volte.
    let height = (3.5 * e).max(3.5);
    let radius = 0.3 * (2.5 * e).max(2.5);
    // Il piede del raccordo sta dietro il prodotto; il telo poi sale e arretra.
    let foot = back - 0.35 * e;
    let depth = (c.z - foot) + 2.5 * e;
    (
        AddedKind::StudioSweep {
            width: (12.0 * e).max(10.0),
            depth,
            height,
            radius,
            tilt_deg: SWEEP_TILT_DEG,
        },
        Mat4::from_translation(Vec3::new(c.x, ground, foot)),
    )
}

fn style_of(kind: &AddedKind) -> Option<BackdropStyle> {
    match kind {
        AddedKind::Backdrop { .. } => Some(BackdropStyle::Curved),
        AddedKind::StudioSweep { .. } => Some(BackdropStyle::Sweep),
        _ => None,
    }
}

/// Fondale presente nella scena (il primo, curvo o fotografico).
pub fn find_backdrop(scene: &Scene) -> Option<ComponentId> {
    scene.added.iter().find(|r| style_of(&r.kind).is_some()).map(|r| r.id.clone())
}

/// Valori che un preset sta per cambiare, per "Annulla preset".
#[derive(Clone, Debug)]
pub struct PresetBackup {
    lights: Vec<Light>,
    environment: Environment,
    environment_visible: bool,
    tone_mapping: ToneMapping,
    exposure_ev: f32,
    target_samples: u32,
    max_bounces: u32,
    denoise_mode: DenoiseMode,
    /// Fondale aggiunto dal preset (da togliere annullando).
    added_backdrop: Option<ComponentId>,
    /// Fondale dell'altro tipo tolto dal preset (da rimettere annullando), con il suo materiale.
    removed_backdrop: Option<(AddedRecord, Option<MaterialId>)>,
    /// Assegnazione del fondale prima del preset.
    backdrop_assignment: Option<(ComponentId, Option<MaterialId>)>,
    /// Materiale del fondale prima del preset (se esisteva).
    backdrop_material: Option<Material>,
    /// Trasformazioni utente delle radici del prodotto prima di "Appoggia".
    transforms: Vec<(ComponentId, UserTransform)>,
}

impl PresetBackup {
    pub fn capture(scene: &Scene, s: &RenderSettings) -> Self {
        Self {
            lights: scene.lights.clone(),
            environment: scene.environment.clone(),
            environment_visible: s.environment_visible,
            tone_mapping: s.tone_mapping,
            exposure_ev: s.exposure_ev,
            target_samples: s.target_samples,
            max_bounces: s.max_bounces,
            denoise_mode: s.denoise.mode,
            added_backdrop: None,
            removed_backdrop: None,
            backdrop_assignment: find_backdrop(scene).map(|id| {
                let m = scene.assignments.get(&id).cloned();
                (id, m)
            }),
            backdrop_material: scene.materials.get(&MaterialId::new(BACKDROP_MATERIAL)).cloned(),
            transforms: product_roots(scene)
                .into_iter()
                .map(|r| (scene.nodes[r].id.clone(), scene.nodes[r].user))
                .collect(),
        }
    }

    /// Ripristina i valori salvati (solo quelli toccati dai preset).
    pub fn restore(&self, scene: &mut Scene, s: &mut RenderSettings) {
        scene.lights = self.lights.clone();
        scene.environment = self.environment.clone();
        s.environment_visible = self.environment_visible;
        s.tone_mapping = self.tone_mapping;
        s.exposure_ev = self.exposure_ev;
        s.target_samples = self.target_samples;
        s.max_bounces = self.max_bounces;
        s.denoise.mode = self.denoise_mode;
        if let Some(id) = &self.added_backdrop {
            if let Some(i) = scene.find(id) {
                scene.delete_added(i);
            }
        }
        if let Some((rec, m)) = &self.removed_backdrop {
            if scene.find(&rec.id).is_none() {
                scene.instantiate_added(rec);
                scene.added.push(rec.clone());
                if let Some(m) = m {
                    scene.assignments.insert(rec.id.clone(), m.clone());
                }
            }
        }
        if let Some((id, m)) = &self.backdrop_assignment {
            match m {
                Some(m) => {
                    scene.assignments.insert(id.clone(), m.clone());
                }
                None => {
                    scene.assignments.remove(id);
                }
            }
        }
        let mid = MaterialId::new(BACKDROP_MATERIAL);
        match &self.backdrop_material {
            Some(m) => scene.materials.upsert(m.clone()),
            None if !scene.assignments.values().any(|v| *v == mid) => {
                scene.materials.materials.retain(|m| m.id != mid);
            }
            None => {}
        }
        for (id, t) in &self.transforms {
            if let Some(i) = scene.find(id) {
                scene.nodes[i].user = *t;
            }
        }
    }
}

/// Applica un aspetto: luci dimensionate sul prodotto, ambiente, tone mapping, esposizione e
/// colore del fondale (se c'è). Non crea il fondale: lo fa [`prepare_scene`].
/// Valori di base della modalità Semplice, applicati sempre (nel pannello Semplice non si vedono;
/// in Avanzata sono nella scheda Render):
/// - arresto a soglia di rumore 0,05 con almeno 32 campioni;
/// - denoiser anche sull'anteprima durante il render, CUDA se la scheda lo permette (altrimenti
///   il denoiser usa da solo la CPU), qualità finale alta e anteprima veloce, prima anteprima a 8
///   campioni, intensità 90 %, dettagli preservati (albedo e normali).
pub fn apply_simple_base(s: &mut RenderSettings) {
    s.noise_threshold = 0.05;
    s.adaptive_min_samples = 32;
    let d = &mut s.denoise;
    d.mode = DenoiseMode::Progressive;
    d.use_cuda = true;
    d.quality = DenoiseQuality::High;
    d.preview_quality = DenoiseQuality::Fast;
    d.first_preview_samples = 8;
    d.strength = 0.9;
    d.use_guides = true;
}

pub fn apply_look(scene: &mut Scene, s: &mut RenderSettings, look: Look) {
    let spec = look.spec();
    let b = product_bounds(scene);
    let (center, radius) = if b.is_empty() {
        (Vec3::ZERO, 1.0)
    } else {
        (b.center(), b.radius().max(1e-3))
    };
    if let Some(lights) = forge_scene::light::preset(spec.lights, center, radius) {
        scene.lights = lights;
    }
    scene.environment = spec.environment.clone();
    s.environment_visible = spec.environment_visible;
    s.tone_mapping = spec.tone_mapping;
    s.exposure_ev = spec.exposure_ev;
    if let Some(id) = find_backdrop(scene) {
        let mid = MaterialId::new(BACKDROP_MATERIAL);
        scene.materials.upsert(Material {
            id: mid.clone(),
            name: "Fondale (preset)".into(),
            base_color: spec.backdrop_color,
            roughness: spec.backdrop_roughness,
            metallic: 0.0,
            category: "Scenografia".into(),
            ..Default::default()
        });
        scene.assignments.insert(id, mid);
    }
}

/// Porta il prodotto ad appoggiare sulla base (quota 0), tenendo le parti nella loro
/// posizione relativa. Restituisce lo spostamento verticale applicato.
pub fn drop_product_to_base(scene: &mut Scene) -> f32 {
    let b = product_bounds(scene);
    if b.is_empty() {
        return 0.0;
    }
    let dy = -b.min.y;
    if dy.abs() < 1e-6 {
        return 0.0;
    }
    for r in product_roots(scene) {
        scene.translate_world(r, Vec3::new(0.0, dy, 0.0));
    }
    dy
}

/// "Prepara scena prodotto": appoggia il prodotto sulla base, aggiunge il fondale se manca
/// (dimensionato sul prodotto), applica aspetto e qualità. L'inquadratura la fa l'app (camera).
/// Restituisce il backup per "Annulla preset".
pub fn prepare_scene(scene: &mut Scene, s: &mut RenderSettings, look: Look, quality: Quality) -> PresetBackup {
    let mut backup = PresetBackup::capture(scene, s);
    drop_product_to_base(scene);
    fit_backdrop(scene, look.spec().backdrop, &mut backup);
    apply_look(scene, s, look);
    quality.apply(s);
    backup
}

/// Mette un fondale del tipo `want` dimensionato e posizionato sul prodotto: quello presente
/// (di qualunque tipo) viene tolto e rifatto, mantenendo il suo materiale. Annotato nel backup.
fn fit_backdrop(scene: &mut Scene, want: BackdropStyle, backup: &mut PresetBackup) {
    let mut material = None;
    if let Some(r) = scene.added.iter().find(|r| style_of(&r.kind).is_some()).cloned() {
        let m = scene.assignments.get(&r.id).cloned();
        material = m.clone();
        if let Some(i) = scene.find(&r.id) {
            scene.delete_added(i);
        }
        backup.removed_backdrop = Some((r, m));
    }
    let (kind, placement) = backdrop_for(&product_bounds(scene), want);
    let i = scene.add_object(kind, placement);
    let id = scene.nodes[i].id.clone();
    if let Some(m) = material {
        scene.assignments.insert(id.clone(), m);
    }
    backup.added_backdrop = Some(id);
}

/// "Adatta la scena al prodotto": dopo aver spostato o scalato il prodotto, rifà fondale (dello
/// stesso tipo e con lo stesso materiale) e luci dell'aspetto attuale sulle sue nuove misure,
/// tenendo i colori delle luci. Senza aspetto riconosciuto le luci restano come sono.
pub fn fit_scene(scene: &mut Scene, s: &RenderSettings) -> PresetBackup {
    let mut backup = PresetBackup::capture(scene, s);
    // Le luci sono ancora dimensionate sulle vecchie misure: l'aspetto si riconosce dal resto.
    let look = matching_look(scene, s).or_else(|| look_ignoring_light_placement(scene, s));
    let style = scene
        .added
        .iter()
        .find_map(|r| style_of(&r.kind))
        .or(look.map(|l| l.spec().backdrop))
        .unwrap_or(BackdropStyle::Sweep);
    fit_backdrop(scene, style, &mut backup);
    if let Some(l) = look {
        let b = product_bounds(scene);
        if !b.is_empty() {
            let colors: Vec<[f32; 3]> = scene.lights.iter().map(|l| l.color).collect();
            if let Some(mut lights) = forge_scene::light::preset(l.spec().lights, b.center(), b.radius().max(1e-3)) {
                for (l, c) in lights.iter_mut().zip(colors) {
                    l.color = c;
                }
                scene.lights = lights;
            }
        }
    }
    backup
}

/// Aspetto riconosciuto da ambiente, visualizzazione e nomi delle luci, senza guardare dove
/// stanno le luci (servono dopo che il prodotto è stato spostato o scalato).
fn look_ignoring_light_placement(scene: &Scene, s: &RenderSettings) -> Option<Look> {
    Look::ALL.into_iter().find(|l| {
        let spec = l.spec();
        let names: Vec<String> = forge_scene::light::preset(spec.lights, Vec3::ZERO, 1.0)
            .unwrap_or_default()
            .into_iter()
            .map(|l| l.name)
            .collect();
        scene.lights.iter().map(|l| &l.name).eq(names.iter())
            && scene.environment == spec.environment
            && s.environment_visible == spec.environment_visible
            && s.tone_mapping == spec.tone_mapping
            && (s.exposure_ev - spec.exposure_ev).abs() < 1e-4
    })
}

/// Aspetto corrispondente allo stato attuale (luci, ambiente e visualizzazione uguali al
/// preset), per mostrare "Personalizzato" quando l'utente ha cambiato qualcosa.
pub fn matching_look(scene: &Scene, s: &RenderSettings) -> Option<Look> {
    let b = product_bounds(scene);
    let (center, radius) = if b.is_empty() {
        (Vec3::ZERO, 1.0)
    } else {
        (b.center(), b.radius().max(1e-3))
    };
    Look::ALL.into_iter().find(|l| {
        let spec = l.spec();
        let lights = forge_scene::light::preset(spec.lights, center, radius).unwrap_or_default();
        // Il colore delle luci si può cambiare senza uscire dall'aspetto.
        scene.lights.len() == lights.len()
            && scene.lights.iter().zip(&lights).all(|(a, b)| {
                let mut a = a.clone();
                a.color = b.color;
                a == *b
            })
            && scene.environment == spec.environment
            && s.environment_visible == spec.environment_visible
            && s.tone_mapping == spec.tone_mapping
            && (s.exposure_ev - spec.exposure_ev).abs() < 1e-4
    })
}

/// Colore delle luci da una temperatura in kelvin (3200 calda, 5500 neutra, 7500 fredda).
/// I portali non emettono e restano come sono.
pub fn set_lights_kelvin(scene: &mut Scene, kelvin: f32) {
    let c = forge_scene::light::kelvin_to_rgb(kelvin);
    for l in &mut scene.lights {
        if !matches!(l.kind, forge_scene::LightKind::Portal { .. }) {
            l.color = c;
        }
    }
}

/// Scena con cui si apre l'app: gli oggetti della scena di test (senza il suo pavimento) sul
/// fondale fotografico, con le luci da studio e la camera sul prodotto.
pub fn studio_demo_scene() -> Scene {
    let mut scene = Scene::test_scene();
    if let Some(i) = scene.find(&ComponentId("test/Scena/pavimento".into())) {
        scene.remove_subtree(i);
    }
    let mut s = RenderSettings::default();
    let _ = prepare_scene(&mut scene, &mut s, Look::Studio, Quality::Balanced);
    let b = product_bounds(&scene);
    let mut orbit = forge_scene::OrbitController::from_camera(&scene.camera);
    orbit.frame(&b);
    scene.camera = orbit.camera();
    scene
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_render_core::{Accumulator, RenderBackend, RenderScene};

    /// Scena di test senza pavimento né luci: un cubo sollevato da terra.
    fn product_scene() -> Scene {
        let mut s = Scene {
            lights: Vec::new(),
            ..Default::default()
        };
        let mesh = s.add_mesh(forge_scene::primitives::uv_sphere(0.2, 24, 12));
        s.add_node(
            None,
            ComponentId("prodotto".into()),
            "Prodotto",
            Mat4::from_translation(Vec3::new(0.3, 0.7, -0.1)),
            Some(mesh),
        );
        s.refresh_pivots();
        s
    }

    fn mean_luminance(scene: &Scene, s: &RenderSettings, cam: &forge_scene::Camera) -> f64 {
        let mut scene = scene.clone();
        scene.camera = cam.clone();
        let rs = RenderScene::compile(&scene);
        let st = RenderSettings {
            width: 48,
            height: 32,
            ..s.clone()
        };
        let mut be = forge_render_cpu::CpuBackend::new(2).unwrap();
        let mut acc = Accumulator::new(st.width, st.height);
        let mut pass = vec![[0.0; 3]; acc.pixel_count()];
        let cancel = std::sync::atomic::AtomicBool::new(false);
        for i in 0..8 {
            let n = be.render_pass(&rs, &st, i, 1, &mut pass, &cancel).unwrap();
            acc.add_samples(&pass, n);
        }
        let img = acc.resolve();
        img.iter().map(|p| (p[0] + p[1] + p[2]) as f64 / 3.0).sum::<f64>() / img.len() as f64
    }

    /// Modalità Semplice: i valori di base richiesti (adattivo, denoiser anche in anteprima,
    /// CUDA, qualità) e la qualità scelta resta riconosciuta.
    #[test]
    fn simple_base_values() {
        let mut s = RenderSettings::default();
        Quality::High.apply(&mut s);
        apply_simple_base(&mut s);
        assert_eq!((s.noise_threshold, s.adaptive_min_samples), (0.05, 32));
        let d = &s.denoise;
        assert_eq!(d.mode, DenoiseMode::Progressive);
        assert!(d.use_cuda && d.use_guides);
        assert_eq!(
            (d.quality, d.preview_quality, d.first_preview_samples),
            (DenoiseQuality::High, DenoiseQuality::Fast, 8)
        );
        assert!((d.strength - 0.9).abs() < 1e-6);
        assert_eq!(Quality::matching(&s), Some(Quality::High));
        let before = s.clone();
        apply_simple_base(&mut s);
        assert_eq!(s, before, "applicarli di nuovo non cambia niente");
    }

    #[test]
    fn prepare_scene_drops_adds_backdrop_and_configures() {
        let mut scene = product_scene();
        let mut s = RenderSettings::default();
        let before = product_bounds(&scene);
        assert!(before.min.y > 0.4);
        let backup = prepare_scene(&mut scene, &mut s, Look::Light, Quality::Balanced);
        // Appoggiato sulla base, senza spostamenti orizzontali.
        let after = product_bounds(&scene);
        assert!(after.min.y.abs() < 1e-4, "{after:?}");
        assert!((after.center().x - before.center().x).abs() < 1e-5);
        // Fondale aggiunto, con il materiale del preset; non conta nell'ingombro del prodotto.
        let bd = find_backdrop(&scene).expect("fondale");
        assert_eq!(scene.assignments.get(&bd), Some(&MaterialId::new(BACKDROP_MATERIAL)));
        assert_eq!(product_bounds(&scene), after);
        // Valori reali del renderer.
        assert_eq!(scene.lights.len(), 2, "preset studio");
        assert_eq!(Quality::matching(&s), Some(Quality::Balanced));
        assert_eq!((s.target_samples, s.max_bounces), (256, 12));
        assert_eq!(matching_look(&scene, &s), Some(Look::Light));
        // Ripetere non aggiunge un secondo fondale.
        let _ = prepare_scene(&mut scene, &mut s, Look::Dark, Quality::Draft);
        assert_eq!(
            scene.added.iter().filter(|r| matches!(r.kind, AddedKind::Backdrop { .. })).count(),
            1
        );
        assert_eq!(matching_look(&scene, &s), Some(Look::Dark));
        // Una modifica dell'utente: "Personalizzato".
        s.exposure_ev += 0.5;
        assert_eq!(matching_look(&scene, &s), None);
        let _ = backup;
    }

    #[test]
    fn undo_restores_only_what_the_preset_changed() {
        let mut scene = product_scene();
        scene.lights = vec![Light::area("Mia luce", Vec3::new(1.0, 2.0, 1.0), Vec3::ZERO, 0.5, 0.5, 7.0)];
        let mut s = RenderSettings {
            target_samples: 77,
            exposure_ev: -0.4,
            ..Default::default()
        };
        let lights0 = scene.lights.clone();
        let env0 = scene.environment.clone();
        let s0 = s.clone();
        let user0 = scene.nodes[0].user;
        let backup = prepare_scene(&mut scene, &mut s, Look::Dramatic, Quality::High);
        assert_ne!(scene.lights, lights0);
        // Modifica fatta dopo il preset, su un'impostazione che il preset non tocca: resta.
        s.width = 640;
        backup.restore(&mut scene, &mut s);
        assert_eq!(scene.lights, lights0);
        assert_eq!(scene.environment, env0);
        assert_eq!(s.target_samples, 77);
        assert_eq!(s.exposure_ev, s0.exposure_ev);
        assert_eq!(s.max_bounces, s0.max_bounces);
        assert_eq!(s.width, 640, "le altre modifiche non si perdono");
        assert!(find_backdrop(&scene).is_none(), "il fondale aggiunto dal preset si toglie");
        assert!(scene.materials.get(&MaterialId::new(BACKDROP_MATERIAL)).is_none());
        assert_eq!(scene.nodes[0].user, user0, "il prodotto torna dov'era");
    }

    /// I preset cambiano davvero l'immagine: con la stessa camera, sfondo chiaro molto più
    /// luminoso dello scuro (render CPU reale, piccolo).
    #[test]
    fn looks_change_the_rendered_image() {
        let mut scene = product_scene();
        let mut s = RenderSettings::default();
        let _ = prepare_scene(&mut scene, &mut s, Look::Light, Quality::Draft);
        let b = product_bounds(&scene);
        let cam = forge_scene::Camera {
            position: b.center() + Vec3::new(0.0, 0.4, 1.6),
            target: b.center(),
            up: Vec3::Y,
            vfov_deg: 40.0,
            ..Default::default()
        };
        let light = mean_luminance(&scene, &s, &cam);
        let _ = prepare_scene(&mut scene, &mut s, Look::Dark, Quality::Draft);
        let dark = mean_luminance(&scene, &s, &cam);
        eprintln!("luminanza media: chiaro {light:.4}, scuro {dark:.4}");
        assert!(light > 0.2, "sfondo chiaro: {light}");
        assert!(dark < light * 0.5, "sfondo scuro {dark} contro chiaro {light}");
    }

    /// "Studio fotografico": fondale in tela inclinato e 4 luci; passando a un aspetto con il
    /// fondale curvo il telo viene sostituito, e "Annulla preset" lo rimette.
    #[test]
    fn studio_look_uses_sweep_and_swaps_backdrop_undoably() {
        let mut scene = product_scene();
        let mut s = RenderSettings::default();
        let _ = prepare_scene(&mut scene, &mut s, Look::Studio, Quality::Balanced);
        let rec = backdrop_record_of(&scene);
        assert!(matches!(rec.kind, AddedKind::StudioSweep { .. }), "{:?}", rec.kind);
        assert_eq!(scene.lights.len(), 4);
        assert!(scene.lights.iter().all(|l| !l.visible_to_camera), "luci fuori dall'inquadratura");
        assert_eq!(matching_look(&scene, &s), Some(Look::Studio));
        // Colore delle luci: resta l'aspetto Studio, non "Personalizzato".
        set_lights_kelvin(&mut scene, 3200.0);
        assert_ne!(scene.lights[0].color, [1.0; 3]);
        assert_eq!(matching_look(&scene, &s), Some(Look::Studio));
        // Il prodotto sta davanti al telo.
        let b = product_bounds(&scene);
        assert!(rec.placement.w_axis.z < b.min.z, "telo dietro il prodotto");
        // Sfondo chiaro vuole il fondale curvo: il telo viene sostituito.
        let undo = prepare_scene(&mut scene, &mut s, Look::Light, Quality::Balanced);
        assert!(matches!(backdrop_record_of(&scene).kind, AddedKind::Backdrop { .. }));
        assert_eq!(scene.added.iter().filter(|r| style_of(&r.kind).is_some()).count(), 1);
        undo.restore(&mut scene, &mut s);
        assert!(matches!(backdrop_record_of(&scene).kind, AddedKind::StudioSweep { .. }));
        assert_eq!(scene.added.iter().filter(|r| style_of(&r.kind).is_some()).count(), 1);
        assert_eq!(
            scene.assignments.get(&backdrop_record_of(&scene).id),
            Some(&MaterialId::new(BACKDROP_MATERIAL))
        );
    }

    #[test]
    fn studio_demo_scene_is_ready() {
        let scene = studio_demo_scene();
        assert!(scene.find(&ComponentId("test/Scena/pavimento".into())).is_none());
        assert!(matches!(backdrop_record_of(&scene).kind, AddedKind::StudioSweep { .. }));
        let b = product_bounds(&scene);
        assert!(b.min.y.abs() < 1e-4, "oggetti appoggiati");
        // La camera guarda il prodotto.
        assert!((scene.camera.target - b.center()).length() < 1e-3);
    }

    /// Prodotto ingrandito e spostato dopo il preset: "Adatta" rifà fondale e luci sulle nuove
    /// misure, tiene materiale del fondale, aspetto e colori delle luci; si annulla.
    #[test]
    fn fit_scene_follows_the_product() {
        let mut scene = product_scene();
        let mut s = RenderSettings::default();
        let _ = prepare_scene(&mut scene, &mut s, Look::Studio, Quality::Balanced);
        set_lights_kelvin(&mut scene, 3200.0);
        let warm = scene.lights[0].color;
        let old = backdrop_record_of(&scene);
        // L'utente scala il prodotto di 3 volte e lo sposta di lato.
        scene.nodes[0].user.scale = Vec3::splat(3.0);
        scene.nodes[0].user.translation += Vec3::new(2.0, 0.0, 0.0);
        drop_product_to_base(&mut scene);
        let undo = fit_scene(&mut scene, &s);
        let new = backdrop_record_of(&scene);
        let (AddedKind::StudioSweep { width: w0, .. }, AddedKind::StudioSweep { width: w1, .. }) = (&old.kind, &new.kind) else {
            panic!("tipo di fondale cambiato");
        };
        assert!(w1 > w0, "fondale più grande: {w0} → {w1}");
        let b = product_bounds(&scene);
        assert!(
            (new.placement.w_axis.x - b.center().x).abs() < 1e-3,
            "fondale centrato sul prodotto"
        );
        assert_eq!(scene.assignments.get(&new.id), Some(&MaterialId::new(BACKDROP_MATERIAL)));
        assert_eq!(matching_look(&scene, &s), Some(Look::Studio), "luci rifatte sulle nuove misure");
        assert_eq!(scene.lights[0].color, warm, "colori tenuti");
        undo.restore(&mut scene, &mut s);
        assert_eq!(backdrop_record_of(&scene).kind, old.kind);
    }

    fn backdrop_record_of(scene: &Scene) -> AddedRecord {
        let id = find_backdrop(scene).expect("fondale");
        scene.added.iter().find(|r| r.id == id).cloned().unwrap()
    }

    #[test]
    fn backdrop_sizing_matches_add_menu_rule() {
        let b = Aabb {
            min: Vec3::new(-0.5, 0.0, -0.25),
            max: Vec3::new(0.5, 0.4, 0.25),
        };
        let (kind, m) = backdrop_for(&b, BackdropStyle::Curved);
        let AddedKind::Backdrop { width, height, .. } = kind else {
            panic!()
        };
        // Prodotto largo 1 m: fondale largo 12 m e alto 3,5 m (riempie anche le viste ruotate).
        assert_eq!(width, 12.0);
        assert_eq!(height, 3.5);
        assert!(m.w_axis.z < b.min.z + 0.6, "dietro il prodotto");
    }
}
