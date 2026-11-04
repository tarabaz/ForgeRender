//! Materiali PBR del renderer, indipendenti dal file CAD di origine.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Identificatore stabile di un materiale (stringa, salvata nel progetto).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MaterialId(pub String);

impl MaterialId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl std::fmt::Display for MaterialId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Materiale fisicamente plausibile in stile "metallic/roughness".
///
/// Modello implementato dal backend CPU (Milestone 1):
/// - `metallic` interpola tra dielettrico (diffuso + speculare GGX) e conduttore (GGX con F0 = base_color);
/// - `transmission` interpola verso un dielettrico trasparente liscio (vetro), con Fresnel esatto e `ior`;
/// - `emission` è radianza emessa (W·sr⁻¹·m⁻²) moltiplicata per `emission_strength`.
///
/// Texture opzionali (mappe PBR), con mappatura UV o box:
/// - colore base (sRGB), moltiplicato per `base_color`;
/// - roughness (canale R, lineare; oppure glossiness invertita), moltiplicata per `roughness`;
/// - metallicità (canale R), moltiplicata per `metallic`;
/// - normal map in tangent space (OpenGL, +Y; oppure DirectX, −Y);
/// - height (rilievo con profondità `bump_mm`, applicato come bump mapping);
/// - occlusione ambientale (scurisce il colore base secondo `ao_strength`);
/// - opacità (ritaglio: sotto il 50 % la superficie non esiste, anche per le ombre).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Material {
    pub id: MaterialId,
    pub name: String,
    /// Colore base lineare (non sRGB).
    pub base_color: [f32; 3],
    pub metallic: f32,
    pub roughness: f32,
    pub ior: f32,
    /// Dispersione dei trasparenti: numero di Abbe (vetro crown ~64, flint ~36, diamante ~55,
    /// più basso = più dispersione); 0 = nessuna. Si vede solo nel render spettrale.
    pub abbe: f32,
    /// Scala del riflesso speculare dei dielettrici (0.5 = F0 derivato da `ior`).
    pub specular: f32,
    pub transmission: f32,
    pub emission: [f32; 3],
    pub emission_strength: f32,
    pub base_color_texture: Option<PathBuf>,
    pub roughness_texture: Option<PathBuf>,
    pub normal_texture: Option<PathBuf>,
    pub normal_strength: f32,
    pub metallic_texture: Option<PathBuf>,
    pub height_texture: Option<PathBuf>,
    pub ao_texture: Option<PathBuf>,
    pub opacity_texture: Option<PathBuf>,
    /// Normal map in convenzione DirectX (verde invertito rispetto a OpenGL).
    pub normal_directx: bool,
    /// La mappa di rugosità è una mappa di lucidità (gloss): viene invertita.
    pub roughness_is_gloss: bool,
    /// Profondità del rilievo della mappa height, in millimetri.
    pub bump_mm: f32,
    /// Displacement vero (la geometria viene suddivisa e spostata lungo la normale
    /// secondo la mappa height): escursione totale in millimetri, 0 = spento.
    pub displacement_mm: f32,
    /// Livello della height che non sposta la superficie (0 = tutto in fuori, 0.5 = metà dentro e metà fuori).
    pub displacement_mid: f32,
    /// Intensità dell'occlusione ambientale (0 = ignorata, 1 = piena).
    pub ao_strength: f32,
    pub mapping: TextureMapping,
    /// Materiali trasparenti: colore che prende la luce dopo `volume_depth_cm` di percorso
    /// dentro l'oggetto (legge di Beer). Bianco = nessun assorbimento. A differenza del colore
    /// base (che tinge la superficie) dipende dallo spessore: un liquido è più scuro e saturo
    /// dove è profondo, più chiaro sui bordi.
    pub volume_color: [f32; 3],
    pub volume_depth_cm: f32,
    /// Priorità dei volumi trasparenti che si sovrappongono (liquido dentro un bicchiere):
    /// nella parte comune vale quello con priorità più alta. Il liquido si modella un filo
    /// più grande della cavità del vetro, con priorità maggiore del vetro.
    pub priority: i32,
    /// Categoria per la libreria. [`LIQUIDS_CATEGORY`] rende il materiale un liquido.
    pub category: String,
}

/// Categoria dei liquidi della libreria: un materiale di questa categoria è un liquido.
pub const LIQUIDS_CATEGORY: &str = "Liquidi";

impl Material {
    /// Liquido (categoria [`LIQUIDS_CATEGORY`]): dove si sovrappone al contenitore vince lui a
    /// parità di priorità, così basta modellarlo un filo più grande della cavità.
    pub fn is_liquid(&self) -> bool {
        self.category.eq_ignore_ascii_case(LIQUIDS_CATEGORY)
    }
}

/// Come si proiettano le texture sulla superficie.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Projection {
    /// Coordinate UV della mesh (OBJ con `vt`). Senza UV si ricade sulla proiezione box.
    Uv,
    /// Proiezione cubica sugli assi dell'oggetto (la texture lo segue quando lo si sposta o
    /// ruota), con le tre proiezioni sfumate secondo la normale:
    /// adatta ai modelli CAD, che di solito non hanno UV.
    #[default]
    Box,
}

/// A cosa è ancorata la proiezione Box: assi e origine da cui partono le coordinate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum TextureAnchor {
    /// Al singolo pezzo (0.15+): la texture lo segue quando lo si sposta o lo si ruota; pezzi
    /// accostati e orientati allo stesso modo restano continui, pezzi ruotati diversamente nel
    /// file hanno ognuno i propri assi.
    #[default]
    Component,
    /// All'assieme (la radice dell'albero: file importato o oggetto aggiunto): tutti i pezzi
    /// condividono la proiezione, continua anche tra pezzi ruotati diversamente; segue l'assieme,
    /// mentre su un pezzo spostato da solo la texture scorre.
    Assembly,
    /// Al mondo (come fino alla 0.14): la texture resta ferma e l'oggetto ci scorre sopra.
    World,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TextureMapping {
    pub projection: Projection,
    /// Box: lato di una ripetizione della texture in metri. UV: numero di ripetizioni.
    pub scale: f32,
    pub rotation_deg: f32,
    pub offset: [f32; 2],
    /// Solo per la proiezione Box: a cosa sono ancorate le coordinate.
    pub anchor: TextureAnchor,
    /// Solo per la proiezione Box: la ripetizione segue la dimensione dell'oggetto invece dei
    /// metri. `scale` è allora la frazione del lato più lungo del pezzo (o dell'assieme, con
    /// l'ancoraggio Assieme) coperta da una ripetizione: 1 = una ripetizione sull'oggetto, come
    /// le anteprime di BlenderKit; la stessa texture sta bene su una sfera da 5 cm e su un tavolo.
    pub fit_object: bool,
}

impl Default for TextureMapping {
    fn default() -> Self {
        Self {
            projection: Projection::Box,
            scale: 0.1,
            rotation_deg: 0.0,
            offset: [0.0; 2],
            anchor: TextureAnchor::Component,
            fit_object: false,
        }
    }
}

impl TextureMapping {
    /// Trasforma le coordinate di base (UV della mesh o proiezione box già in metri).
    pub fn apply(&self, uv: forge_math::Vec2, is_box: bool) -> forge_math::Vec2 {
        let s = if is_box { 1.0 / self.scale.max(1e-6) } else { self.scale };
        let (sn, cs) = self.rotation_deg.to_radians().sin_cos();
        let p = uv * s;
        forge_math::Vec2::new(p.x * cs - p.y * sn, p.x * sn + p.y * cs) + forge_math::Vec2::from_array(self.offset)
    }

    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

impl Default for Material {
    fn default() -> Self {
        Self {
            id: MaterialId::new("default"),
            name: "Predefinito".into(),
            base_color: [0.8, 0.8, 0.8],
            metallic: 0.0,
            roughness: 0.5,
            ior: 1.5,
            abbe: 0.0,
            specular: 0.5,
            transmission: 0.0,
            emission: [0.0; 3],
            emission_strength: 0.0,
            base_color_texture: None,
            roughness_texture: None,
            normal_texture: None,
            normal_strength: 1.0,
            metallic_texture: None,
            height_texture: None,
            ao_texture: None,
            opacity_texture: None,
            normal_directx: false,
            roughness_is_gloss: false,
            bump_mm: 1.0,
            displacement_mm: 0.0,
            displacement_mid: 0.5,
            ao_strength: 0.5,
            mapping: TextureMapping::default(),
            volume_color: [1.0; 3],
            volume_depth_cm: 10.0,
            priority: 0,
            category: "Generico".into(),
        }
    }
}

impl Material {
    /// Riporta i parametri negli intervalli validi.
    pub fn sanitized(&self) -> Material {
        let mut m = self.clone();
        let c01 = |v: f32| if v.is_finite() { v.clamp(0.0, 1.0) } else { 0.0 };
        m.base_color = m.base_color.map(c01);
        m.metallic = c01(m.metallic);
        m.roughness = c01(m.roughness);
        m.specular = c01(m.specular);
        m.transmission = c01(m.transmission);
        m.ior = if m.ior.is_finite() { m.ior.clamp(1.0, 3.0) } else { 1.5 };
        m.emission = m.emission.map(|v| if v.is_finite() { v.max(0.0) } else { 0.0 });
        m.emission_strength = if m.emission_strength.is_finite() {
            m.emission_strength.max(0.0)
        } else {
            0.0
        };
        m.normal_strength = if m.normal_strength.is_finite() {
            m.normal_strength.clamp(0.0, 4.0)
        } else {
            1.0
        };
        m.bump_mm = if m.bump_mm.is_finite() { m.bump_mm.clamp(0.0, 100.0) } else { 1.0 };
        m.ao_strength = c01(m.ao_strength);
        m.displacement_mm = if m.displacement_mm.is_finite() {
            m.displacement_mm.clamp(0.0, 1000.0)
        } else {
            0.0
        };
        m.displacement_mid = c01(m.displacement_mid);
        m.volume_color = m.volume_color.map(|v| if v.is_finite() { v.clamp(1e-4, 1.0) } else { 1.0 });
        m.volume_depth_cm = if m.volume_depth_cm.is_finite() {
            m.volume_depth_cm.clamp(0.01, 1e5)
        } else {
            10.0
        };
        m.priority = m.priority.clamp(-100, 100);
        m.abbe = if m.abbe.is_finite() && m.abbe > 0.0 {
            m.abbe.clamp(5.0, 200.0)
        } else {
            0.0
        };
        if !m.mapping.scale.is_finite() || m.mapping.scale <= 0.0 {
            m.mapping.scale = TextureMapping::default().scale;
        }
        m
    }

    /// Il materiale sposta davvero la geometria (mappa height + escursione > 0).
    pub fn has_displacement(&self) -> bool {
        self.height_texture.is_some() && self.displacement_mm.is_finite() && self.displacement_mm > 0.0
    }

    pub fn has_textures(&self) -> bool {
        self.texture_slots().iter().any(|t| t.is_some())
    }

    /// Tutti gli slot texture (per salvataggio, ricollegamento e copia).
    pub fn texture_slots(&self) -> [&Option<PathBuf>; 7] {
        [
            &self.base_color_texture,
            &self.roughness_texture,
            &self.normal_texture,
            &self.metallic_texture,
            &self.height_texture,
            &self.ao_texture,
            &self.opacity_texture,
        ]
    }

    pub fn texture_slots_mut(&mut self) -> [&mut Option<PathBuf>; 7] {
        [
            &mut self.base_color_texture,
            &mut self.roughness_texture,
            &mut self.normal_texture,
            &mut self.metallic_texture,
            &mut self.height_texture,
            &mut self.ao_texture,
            &mut self.opacity_texture,
        ]
    }

    pub fn is_emissive(&self) -> bool {
        self.emission_strength > 0.0 && self.emission.iter().any(|&c| c > 0.0)
    }
}

/// Converte un colore sRGB (0..1) in lineare.
pub fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

pub fn linear_to_srgb(c: f32) -> f32 {
    let c = c.max(0.0);
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// Raccolta dei materiali disponibili nel progetto.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MaterialLibrary {
    pub materials: Vec<Material>,
}

impl MaterialLibrary {
    pub fn with_presets() -> Self {
        let mut lib = Self { materials: presets() };
        lib.ensure_default();
        lib
    }

    pub fn ensure_default(&mut self) {
        if self.get(&MaterialId::new("default")).is_none() {
            self.materials.insert(0, Material::default());
        }
    }

    pub fn get(&self, id: &MaterialId) -> Option<&Material> {
        self.materials.iter().find(|m| &m.id == id)
    }

    pub fn get_mut(&mut self, id: &MaterialId) -> Option<&mut Material> {
        self.materials.iter_mut().find(|m| &m.id == id)
    }

    pub fn index_of(&self, id: &MaterialId) -> Option<usize> {
        self.materials.iter().position(|m| &m.id == id)
    }

    /// Aggiunge o sostituisce un materiale con lo stesso id.
    pub fn upsert(&mut self, m: Material) {
        if let Some(existing) = self.get_mut(&m.id) {
            *existing = m;
        } else {
            self.materials.push(m);
        }
    }

    /// Genera un id univoco a partire da una base.
    pub fn unique_id(&self, base: &str) -> MaterialId {
        let mut candidate = MaterialId::new(base);
        let mut n = 1;
        while self.get(&candidate).is_some() {
            n += 1;
            candidate = MaterialId::new(format!("{base}-{n}"));
        }
        candidate
    }
}

/// Libreria di materiali pronti all'uso, divisa per categorie.
///
/// I materiali sono dati, non codice: una cartella per categoria (`01 Metalli`, `02 Anodizzati`…)
/// e dentro una cartella per materiale con `materiale.json` e le mappe (lo stesso formato degli
/// zip di "Esporta zip…"), preparate da `forge_render_core/examples/libreria_serie.rs` secondo
/// `tools/libreria-serie/tabella.json`. Si leggono dalla cartella dei materiali compresi nel
/// programma ([`builtin_textures_dir`]) e da quella del pacchetto aggiuntivo
/// ([`extra_materials_dir`]), se c'è: un materiale aggiuntivo con lo stesso id sostituisce quello
/// compreso. Le cartelle che iniziano con `_` o `.` si saltano.
///
/// Se la cartella dei materiali manca (programma copiato senza `materiali`) restano pochi
/// materiali di base senza mappe, gli stessi id usati dalla scena di prova.
///
/// La lettura si fa una volta per processo: le chiamate successive copiano l'elenco.
pub fn presets() -> Vec<Material> {
    static CACHE: std::sync::OnceLock<Vec<Material>> = std::sync::OnceLock::new();
    CACHE
        .get_or_init(|| {
            let mut v = builtin_textures_dir().map(library_presets).unwrap_or_default();
            if let Some(extra) = extra_materials_dir() {
                for m in library_presets(extra) {
                    match v.iter_mut().find(|p| p.id == m.id) {
                        Some(p) => *p = m,
                        None => v.push(m),
                    }
                }
            }
            if v.is_empty() {
                tracing::warn!("cartella dei materiali non trovata: restano i materiali di base");
                v = fallback_presets();
            }
            v
        })
        .clone()
}

/// Materiali di una libreria a cartelle: `<radice>/<categoria>/<materiale>/materiale.json`,
/// categorie e materiali in ordine di nome della cartella.
pub fn library_presets(root: &std::path::Path) -> Vec<Material> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut cats: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir() && !p.file_name().is_some_and(|n| n.to_string_lossy().starts_with(['_', '.'])))
        .collect();
    cats.sort();
    let mut out: Vec<Material> = Vec::new();
    for c in cats {
        for m in data_presets(&c) {
            if !out.iter().any(|p| p.id == m.id) {
                out.push(m);
            }
        }
    }
    out
}

/// Materiali senza mappe se la cartella dei materiali non c'è.
fn fallback_presets() -> Vec<Material> {
    let mut v = Vec::new();
    let mut add = |id: &str, name: &str, cat: &str, f: &dyn Fn(&mut Material)| {
        let mut m = Material {
            id: MaterialId::new(id),
            name: name.into(),
            category: cat.into(),
            ..Default::default()
        };
        f(&mut m);
        v.push(m);
    };
    let metal = |c: [f32; 3], r: f32| {
        move |m: &mut Material| {
            m.base_color = c;
            m.metallic = 1.0;
            m.roughness = r;
        }
    };
    let glass = |ior: f32, abbe: f32| {
        move |m: &mut Material| {
            m.base_color = [1.0; 3];
            m.transmission = 1.0;
            m.ior = ior;
            m.abbe = abbe;
            m.roughness = 0.0;
        }
    };
    add(
        "acciaio-lucidato",
        "Acciaio lucidato a specchio",
        "Metalli",
        &metal([0.56, 0.57, 0.58], 0.06),
    );
    add("acciaio-satinato", "Acciaio satinato", "Metalli", &metal([0.56, 0.57, 0.58], 0.35));
    add("rame", "Rame", "Metalli", &metal([0.95, 0.64, 0.54], 0.2));
    add("ottone", "Ottone", "Metalli", &metal([0.91, 0.78, 0.42], 0.2));
    add("plastica-lucida-rossa", "Plastica lucida rossa", "Plastiche", &|m| {
        m.base_color = [
            srgb_to_linear(180.0 / 255.0),
            srgb_to_linear(20.0 / 255.0),
            srgb_to_linear(20.0 / 255.0),
        ];
        m.roughness = 0.12;
    });
    add("gomma", "Gomma nera", "Gomme", &|m| {
        m.base_color = [0.005; 3];
        m.roughness = 0.85;
        m.specular = 0.3;
    });
    add("vetro", "Vetro", "Vetri", &glass(1.52, 58.0));
    add("diamante", "Diamante", "Vetri", &glass(2.42, 55.0));
    v
}

/// Cartella del pacchetto aggiuntivo dei materiali, scaricabile a parte (stessa struttura di
/// quella compresa): `FORGE_EXTRA_MATERIALS_DIR`, oppure `materiali-aggiuntivi` accanto al
/// programma, oppure `assets/materiali-aggiuntivi` risalendo dal programma (compilazione locale).
pub fn extra_materials_dir() -> Option<&'static std::path::Path> {
    static DIR: std::sync::OnceLock<Option<PathBuf>> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        if let Some(d) = std::env::var_os("FORGE_EXTRA_MATERIALS_DIR") {
            return Some(PathBuf::from(d)).filter(|d| d.is_dir());
        }
        let exe = std::env::current_exe().ok()?;
        let dir = exe.parent()?;
        if dir.join("materiali-aggiuntivi").is_dir() {
            return Some(dir.join("materiali-aggiuntivi"));
        }
        dir.ancestors()
            .map(|a| a.join("assets").join("materiali-aggiuntivi"))
            .find(|d| d.is_dir())
    })
    .as_deref()
}

/// Cartelle in cui ritrovare le mappe dei materiali di serie salvate in un progetto con un'altra
/// installazione: ogni categoria della libreria compresa e di quella aggiuntiva (la mappa si
/// cerca come `<cartella>/<id del materiale>/<file>`); tra queste `_vecchi`, le
/// mappe dei predefiniti fino alla 0.37 (`legno-rovere`, `marmo-bianco`, `pelle-nera`).
pub fn builtin_map_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    for root in [builtin_textures_dir(), extra_materials_dir()].into_iter().flatten() {
        out.push(root.to_path_buf());
        if let Ok(list) = std::fs::read_dir(root) {
            let mut cats: Vec<PathBuf> = list.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
            cats.sort();
            out.extend(cats);
        }
    }
    out
}

/// Materiali di una cartella di predefiniti a dati (ordinati per cartella). Le mappe indicate
/// con il solo nome del file si cercano nella cartella del materiale; un file non valido si salta.
pub fn data_presets(dir: &std::path::Path) -> Vec<Material> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
    dirs.sort();
    let mut out = Vec::new();
    for d in dirs {
        let file = d.join("materiale.json");
        let Ok(text) = std::fs::read_to_string(&file) else { continue };
        let mut m: Material = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!("predefinito non valido {}: {e}", file.display());
                continue;
            }
        };
        for slot in m.texture_slots_mut().into_iter().flatten() {
            if slot.is_relative() {
                *slot = d.join(&*slot);
            }
        }
        out.push(m.sanitized());
    }
    out
}

/// Cartella dei materiali compresi nel programma: `FORGE_MATERIALS_DIR`, oppure `materiali`
/// accanto al programma (pacchetti), oppure `assets/materiali` risalendo dal programma
/// (compilazione locale e test). `None` se non c'è: restano i materiali di base senza mappe.
pub fn builtin_textures_dir() -> Option<&'static std::path::Path> {
    static DIR: std::sync::OnceLock<Option<PathBuf>> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        if let Some(d) = std::env::var_os("FORGE_MATERIALS_DIR") {
            return Some(PathBuf::from(d));
        }
        let exe = std::env::current_exe().ok()?;
        let dir = exe.parent()?;
        if dir.join("materiali").is_dir() {
            return Some(dir.join("materiali"));
        }
        dir.ancestors().map(|a| a.join("assets").join("materiali")).find(|d| d.is_dir())
    })
    .as_deref()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_is_consistent() {
        let p = presets();
        assert!(p.len() >= 100, "libreria troppo piccola: {}", p.len());
        let mut ids: Vec<_> = p.iter().map(|m| m.id.clone()).collect();
        ids.sort();
        let n = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), n, "id duplicati nella libreria");
        for m in &p {
            assert_eq!(&m.sanitized(), m, "{} ha valori fuori intervallo", m.id);
            assert!(!m.category.is_empty() && !m.name.is_empty());
        }
        // Gli id usati dalla scena di test e dai progetti esistenti restano validi.
        for id in [
            "acciaio-lucidato",
            "vetro",
            "plastica-lucida-rossa",
            "rame",
            "ottone",
            "alluminio-anodizzato-blu",
            "gomma",
        ] {
            assert!(p.iter().any(|m| m.id.0 == id), "manca {id}");
        }
    }

    /// Predefiniti a dati: letti con le mappe nella loro cartella (nome del file relativo).
    #[test]
    fn data_presets_load_with_their_maps() {
        let dir = std::env::temp_dir().join(format!("forge_dati_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let m_dir = dir.join("mio-metallo");
        std::fs::create_dir_all(&m_dir).unwrap();
        std::fs::write(m_dir.join("colore.png"), b"x").unwrap();
        let m = Material {
            id: MaterialId::new("mio-metallo"),
            name: "Mio metallo".into(),
            base_color_texture: Some(PathBuf::from("colore.png")),
            ..Default::default()
        };
        std::fs::write(m_dir.join("materiale.json"), serde_json::to_string(&m).unwrap()).unwrap();
        let data = data_presets(&dir);
        assert_eq!(data.len(), 1);
        let t = data[0].base_color_texture.as_ref().unwrap();
        assert!(t.is_file() && t.starts_with(&m_dir), "{}", t.display());
        std::fs::remove_dir_all(dir).ok();
    }

    /// I metalli BlenderKit (solo uso personale) non sono più tra i predefiniti; nessun id doppio.
    #[test]
    fn no_blenderkit_presets_and_unique_ids() {
        let all = presets();
        assert!(all.iter().all(|m| !m.id.0.starts_with("bk-")), "materiali BlenderKit nell'elenco");
        let mut ids: Vec<_> = all.iter().map(|m| &m.id).collect();
        let n = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), n, "id doppi");
    }

    /// La libreria compresa (assets/materiali) si legge anche dai test: categorie in ordine,
    /// mappe che esistono, trasparenti con la dispersione, liquidi nella loro categoria.
    #[test]
    fn builtin_library_is_complete() {
        let dir = builtin_textures_dir().expect("assets/materiali");
        let p = library_presets(dir);
        assert!(p.len() >= 100, "{}", p.len());
        let mut cats: Vec<&str> = Vec::new();
        for m in &p {
            if cats.last() != Some(&m.category.as_str()) {
                assert!(!cats.contains(&m.category.as_str()), "categoria {} spezzata", m.category);
                cats.push(&m.category);
            }
            for t in m.texture_slots().into_iter().flatten() {
                assert!(t.starts_with(dir) && t.is_file(), "{}: {}", m.id, t.display());
            }
            if m.transmission > 0.0 {
                assert!(m.abbe > 0.0, "{} trasparente senza dispersione", m.id);
            }
        }
        assert_eq!(cats.first(), Some(&"Metalli"));
        // Al massimo 5 materiali con mappe per categoria nella libreria compresa.
        for c in &cats {
            let n = p.iter().filter(|m| &m.category == c && m.has_textures()).count();
            assert!(n <= 5, "{c}: {n} materiali con mappe");
        }
        let water = p.iter().find(|m| m.id.0 == "acqua").unwrap();
        assert!(water.is_liquid());
        let oak = p.iter().find(|m| m.id.0 == "legno-rovere").unwrap();
        assert!(oak.base_color_texture.is_some() && oak.normal_texture.is_some());
        assert!(!p.iter().find(|m| m.id.0 == "acciaio-lucidato").unwrap().has_textures());
        // Le vecchie mappe (_vecchi) non sono materiali ma si ritrovano per i progetti salvati.
        assert!(p
            .iter()
            .all(|m| m.texture_slots().into_iter().flatten().all(|t| !t.starts_with(dir.join("_vecchi")))));
        assert!(builtin_map_dirs()
            .iter()
            .any(|d| d.join("legno-rovere").join("colore.jpg").is_file()));
    }

    /// Senza cartella dei materiali restano i materiali di base, con gli id della scena di prova.
    #[test]
    fn fallback_has_the_test_scene_ids() {
        let f = fallback_presets();
        for id in ["acciaio-lucidato", "vetro", "plastica-lucida-rossa", "rame", "gomma"] {
            assert!(f.iter().any(|m| m.id.0 == id), "manca {id}");
        }
        assert!(f.iter().all(|m| m.sanitized() == *m));
    }
}
