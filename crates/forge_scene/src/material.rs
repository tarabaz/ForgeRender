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
    /// Categoria per la libreria (solo UI).
    pub category: String,
}

/// Come si proiettano le texture sulla superficie.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Projection {
    /// Coordinate UV della mesh (OBJ con `vt`). Senza UV si ricade sulla proiezione box.
    Uv,
    /// Proiezione cubica in spazio mondo lungo l'asse dominante della normale:
    /// adatta ai modelli CAD, che di solito non hanno UV.
    #[default]
    Box,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TextureMapping {
    pub projection: Projection,
    /// Box: lato di una ripetizione della texture in metri. UV: numero di ripetizioni.
    pub scale: f32,
    pub rotation_deg: f32,
    pub offset: [f32; 2],
}

impl Default for TextureMapping {
    fn default() -> Self {
        Self {
            projection: Projection::Box,
            scale: 0.1,
            rotation_deg: 0.0,
            offset: [0.0; 2],
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

fn srgb(r: u8, g: u8, b: u8) -> [f32; 3] {
    [r, g, b].map(|v| srgb_to_linear(v as f32 / 255.0))
}

/// Libreria di materiali pronti all'uso, divisa per categorie.
///
/// I valori sono punti di partenza fisicamente plausibili, non misure di
/// laboratorio: un singolo valore di roughness non descrive ogni finitura reale.
/// Colori dei metalli: riflettanza a incidenza normale (F0) da dati spettrali noti
/// (es. "Real-Time Rendering", 4ª ed., tab. 9.2), già in spazio lineare.
/// IOR dei dielettrici da valori tabulati (vetro sodico-calcico 1.52, PMMA 1.49,
/// policarbonato 1.585, acqua 1.333, diamante 2.42…).
pub fn presets() -> Vec<Material> {
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
    let diel = |c: [f32; 3], r: f32| {
        move |m: &mut Material| {
            m.base_color = c;
            m.roughness = r;
        }
    };
    let glass = |c: [f32; 3], ior: f32, r: f32| {
        move |m: &mut Material| {
            m.base_color = c;
            m.transmission = 1.0;
            m.ior = ior;
            m.roughness = r;
        }
    };

    // --- Metalli (F0 lineari) ---
    const STEEL: [f32; 3] = [0.56, 0.57, 0.58];
    const STAINLESS: [f32; 3] = [0.62, 0.62, 0.60];
    const ALU: [f32; 3] = [0.91, 0.92, 0.92];
    const CHROME: [f32; 3] = [0.55, 0.556, 0.554];
    const GOLD: [f32; 3] = [1.0, 0.71, 0.29];
    const SILVER: [f32; 3] = [0.97, 0.96, 0.91];
    const COPPER: [f32; 3] = [0.95, 0.64, 0.54];
    const BRASS: [f32; 3] = [0.91, 0.78, 0.42];
    const BRONZE: [f32; 3] = [0.80, 0.55, 0.35];
    const TITANIUM: [f32; 3] = [0.54, 0.50, 0.46];
    const NICKEL: [f32; 3] = [0.66, 0.61, 0.53];
    const ZINC: [f32; 3] = [0.66, 0.68, 0.69];
    let c = "Metalli";
    add("acciaio-lucidato", "Acciaio lucidato a specchio", c, &metal(STEEL, 0.06));
    add("acciaio-satinato", "Acciaio satinato", c, &metal(STEEL, 0.35));
    add("acciaio-sabbiato", "Acciaio sabbiato", c, &metal(STEEL, 0.6));
    add("acciaio-inox", "Acciaio inox", c, &metal(STAINLESS, 0.22));
    add("acciaio-inox-spazzolato", "Acciaio inox spazzolato", c, &metal(STAINLESS, 0.4));
    add("acciaio-zincato", "Acciaio zincato", c, &metal([0.62, 0.64, 0.65], 0.55));
    add("ghisa", "Ghisa grezza", c, &metal([0.35, 0.35, 0.36], 0.75));
    add("ferro-brunito", "Ferro brunito", c, &metal([0.12, 0.12, 0.13], 0.35));
    add("alluminio", "Alluminio", c, &metal(ALU, 0.25));
    add("alluminio-lucido", "Alluminio lucidato", c, &metal(ALU, 0.06));
    add("alluminio-spazzolato", "Alluminio spazzolato", c, &metal(ALU, 0.42));
    add("alluminio-sabbiato", "Alluminio sabbiato", c, &metal(ALU, 0.62));
    add("cromo", "Cromo", c, &metal(CHROME, 0.03));
    add("cromo-satinato", "Cromo satinato", c, &metal(CHROME, 0.3));
    add("nichel", "Nichel", c, &metal(NICKEL, 0.2));
    add("titanio", "Titanio", c, &metal(TITANIUM, 0.3));
    add("zinco", "Zinco", c, &metal(ZINC, 0.4));
    add("oro", "Oro lucido", c, &metal(GOLD, 0.08));
    add("oro-satinato", "Oro satinato", c, &metal(GOLD, 0.35));
    add("argento", "Argento", c, &metal(SILVER, 0.08));
    add("specchio", "Specchio", c, &metal(SILVER, 0.0));
    add("rame", "Rame", c, &metal(COPPER, 0.2));
    add("rame-ossidato", "Rame brunito", c, &metal([0.45, 0.25, 0.18], 0.5));
    add("ottone", "Ottone", c, &metal(BRASS, 0.2));
    add("ottone-satinato", "Ottone satinato", c, &metal(BRASS, 0.45));
    add("bronzo", "Bronzo", c, &metal(BRONZE, 0.3));

    // --- Alluminio anodizzato (approssimazione: metallo colorato) ---
    let c = "Anodizzati";
    for (id, name, col) in [
        ("blu", "blu", srgb(40, 70, 140)),
        ("nero", "nero", srgb(28, 28, 32)),
        ("rosso", "rosso", srgb(150, 25, 30)),
        ("oro", "oro", srgb(200, 150, 60)),
        ("verde", "verde", srgb(40, 110, 60)),
        ("viola", "viola", srgb(90, 40, 120)),
        ("titanio", "grigio titanio", srgb(90, 92, 98)),
        ("argento", "naturale", srgb(200, 202, 205)),
    ] {
        add(
            &format!("alluminio-anodizzato-{id}"),
            &format!("Alluminio anodizzato {name}"),
            c,
            &metal(col, 0.3),
        );
    }

    // --- Vernici ---
    let c = "Vernici";
    add("vernice", "Vernice lucida blu", c, &diel(srgb(20, 60, 120), 0.08));
    add(
        "vernice-lucida-bianca",
        "Vernice lucida bianca",
        c,
        &diel(srgb(235, 235, 232), 0.08),
    );
    add("vernice-lucida-nera", "Vernice lucida nera", c, &diel(srgb(12, 12, 13), 0.06));
    add(
        "vernice-lucida-rossa",
        "Vernice lucida rossa (Ferrari)",
        c,
        &diel(srgb(180, 10, 12), 0.07),
    );
    add("vernice-opaca-nera", "Vernice opaca nera", c, &diel(srgb(18, 18, 19), 0.65));
    add("vernice-opaca-bianca", "Vernice opaca bianca", c, &diel(srgb(225, 225, 222), 0.7));
    add(
        "verniciatura-polvere",
        "Verniciatura a polvere (RAL 7016)",
        c,
        &diel(srgb(56, 62, 66), 0.45),
    );
    add(
        "verniciatura-polvere-nera",
        "Verniciatura a polvere nera goffrata",
        c,
        &diel(srgb(22, 22, 23), 0.6),
    );
    add("vernice-metallizzata-argento", "Metallizzato argento", c, &|m| {
        metal(srgb(190, 192, 196), 0.28)(m);
        m.metallic = 0.75;
    });
    add("vernice-metallizzata-blu", "Metallizzato blu", c, &|m| {
        metal(srgb(25, 55, 130), 0.25)(m);
        m.metallic = 0.6;
    });
    add("vernice-metallizzata-grafite", "Metallizzato grafite", c, &|m| {
        metal(srgb(60, 62, 66), 0.3)(m);
        m.metallic = 0.65;
    });
    add("vernice-perlata", "Perlato bianco", c, &|m| {
        diel(srgb(238, 236, 228), 0.18)(m);
        m.metallic = 0.25;
        m.specular = 0.7;
    });

    // --- Plastiche ---
    let c = "Plastiche";
    add("plastica-opaca-nera", "Plastica opaca nera", c, &diel(srgb(30, 30, 32), 0.6));
    add("plastica-lucida-rossa", "Plastica lucida rossa", c, &diel(srgb(180, 20, 20), 0.12));
    add("abs-nero", "ABS nero (stampato)", c, &diel(srgb(22, 22, 24), 0.45));
    add("abs-bianco", "ABS bianco", c, &diel(srgb(230, 230, 225), 0.4));
    add("abs-grigio", "ABS grigio", c, &diel(srgb(120, 122, 125), 0.45));
    add("plastica-soft-touch", "Plastica soft-touch", c, &|m| {
        diel(srgb(35, 35, 37), 0.85)(m);
        m.specular = 0.3;
    });
    add("nylon-pa12", "Nylon PA12 (SLS)", c, &|m| {
        diel(srgb(205, 203, 196), 0.9)(m);
        m.specular = 0.35;
    });
    add("nylon-nero", "Nylon nero tinto (MJF)", c, &|m| {
        diel(srgb(40, 40, 42), 0.85)(m);
        m.specular = 0.35;
    });
    add("polipropilene", "Polipropilene naturale", c, &diel(srgb(220, 218, 205), 0.35));
    add("pom-delrin", "POM / Delrin bianco", c, &diel(srgb(240, 238, 230), 0.3));
    add("ptfe", "PTFE / Teflon", c, &diel(srgb(245, 245, 242), 0.55));
    add("policarbonato", "Policarbonato trasparente", c, &glass([1.0; 3], 1.585, 0.0));
    add("plexiglass", "Plexiglass / PMMA", c, &glass([1.0; 3], 1.49, 0.0));
    add("plexiglass-satinato", "Plexiglass satinato", c, &glass([1.0; 3], 1.49, 0.35));
    add("plexiglass-fume", "Plexiglass fumé", c, &glass(srgb(120, 120, 125), 1.49, 0.0));

    // --- Stampa 3D (FDM e resina) ---
    let c = "Stampa 3D";
    for (id, name, col) in [
        ("bianco", "bianco", srgb(235, 235, 230)),
        ("nero", "nero", srgb(25, 25, 27)),
        ("grigio", "grigio", srgb(128, 130, 132)),
        ("rosso", "rosso", srgb(190, 25, 25)),
        ("arancione", "arancione", srgb(235, 110, 20)),
        ("giallo", "giallo", srgb(240, 200, 20)),
        ("verde", "verde", srgb(40, 160, 70)),
        ("blu", "blu", srgb(30, 80, 190)),
    ] {
        add(&format!("pla-{id}"), &format!("PLA {name}"), c, &diel(col, 0.45));
    }
    add("pla-seta-oro", "PLA silk oro", c, &|m| {
        metal(srgb(215, 165, 70), 0.3)(m);
        m.metallic = 0.55;
    });
    add("pla-seta-argento", "PLA silk argento", c, &|m| {
        metal(srgb(195, 197, 200), 0.3)(m);
        m.metallic = 0.55;
    });
    add("petg-trasparente", "PETG trasparente", c, &glass(srgb(235, 245, 240), 1.57, 0.08));
    add("petg-nero", "PETG nero", c, &diel(srgb(20, 20, 22), 0.25));
    add("tpu-nero", "TPU nero (flessibile)", c, &|m| {
        diel(srgb(25, 25, 25), 0.7)(m);
        m.specular = 0.35;
    });
    add("resina-grigia", "Resina standard grigia", c, &diel(srgb(140, 142, 145), 0.35));
    add("resina-bianca", "Resina standard bianca", c, &diel(srgb(232, 232, 228), 0.35));
    add(
        "resina-trasparente",
        "Resina trasparente",
        c,
        &glass(srgb(250, 248, 235), 1.51, 0.02),
    );
    add("resina-acqua", "Resina «water washable» beige", c, &diel(srgb(214, 196, 160), 0.4));
    add("primer-grigio", "Primer grigio (pre-pittura)", c, &diel(srgb(150, 150, 150), 0.75));

    // --- Gomme ---
    let c = "Gomme";
    add("gomma", "Gomma nera", c, &|m| {
        diel(srgb(15, 15, 15), 0.85)(m);
        m.specular = 0.3;
    });
    add("gomma-lucida", "Gomma nera lucida", c, &|m| {
        diel(srgb(15, 15, 15), 0.45)(m);
        m.specular = 0.4;
    });
    add("silicone-grigio", "Silicone grigio", c, &|m| {
        diel(srgb(130, 132, 135), 0.6)(m);
        m.specular = 0.35;
    });
    add(
        "silicone-trasparente",
        "Silicone trasparente",
        c,
        &glass(srgb(240, 240, 235), 1.41, 0.25),
    );
    add("guarnizione-rossa", "Guarnizione rossa", c, &|m| {
        diel(srgb(160, 30, 25), 0.7)(m);
        m.specular = 0.3;
    });

    // --- Vetri e trasparenti ---
    let c = "Vetri";
    add("vetro", "Vetro", c, &glass([1.0; 3], 1.52, 0.0));
    add("vetro-satinato", "Vetro satinato", c, &glass([1.0; 3], 1.52, 0.4));
    add("vetro-verde", "Vetro verde", c, &glass(srgb(170, 215, 180), 1.52, 0.0));
    add("vetro-bronzo", "Vetro bronzato", c, &glass(srgb(200, 165, 120), 1.52, 0.0));
    add("vetro-blu", "Vetro blu", c, &glass(srgb(150, 180, 230), 1.52, 0.0));
    add("vetro-ambra", "Vetro ambra", c, &glass(srgb(230, 150, 50), 1.52, 0.0));
    add("cristallo", "Cristallo al piombo", c, &glass([1.0; 3], 1.7, 0.0));
    add("diamante", "Diamante", c, &glass([1.0; 3], 2.42, 0.0));
    add("acqua", "Acqua", c, &glass([1.0; 3], 1.333, 0.0));
    add("lente-ottica", "Lente ottica", c, &glass([1.0; 3], 1.517, 0.0));

    // --- Ceramiche e pietre ---
    let c = "Ceramiche e pietre";
    add("ceramica", "Ceramica smaltata", c, &diel(srgb(240, 240, 235), 0.08));
    add("porcellana", "Porcellana", c, &diel(srgb(246, 244, 238), 0.15));
    add("terracotta", "Terracotta", c, &diel(srgb(170, 90, 55), 0.85));
    add("marmo-bianco", "Marmo bianco lucido", c, &diel(srgb(235, 233, 228), 0.12));
    add("marmo-nero", "Marmo nero lucido", c, &diel(srgb(25, 25, 27), 0.1));
    add("granito-grigio", "Granito grigio", c, &diel(srgb(110, 108, 106), 0.35));
    add("cemento", "Cemento", c, &diel(srgb(150, 148, 142), 0.9));
    add(
        "allumina-ceramica",
        "Ceramica tecnica (allumina)",
        c,
        &diel(srgb(238, 232, 220), 0.4),
    );

    // --- Legni (tinta unita: per le venature applicare una texture di colore) ---
    let c = "Legni";
    add("legno", "Legno (tinta unita)", c, &diel(srgb(140, 90, 50), 0.55));
    add("legno-rovere", "Rovere", c, &diel(srgb(170, 125, 80), 0.5));
    add("legno-noce", "Noce", c, &diel(srgb(95, 60, 38), 0.45));
    add("legno-ciliegio", "Ciliegio", c, &diel(srgb(150, 75, 45), 0.4));
    add("legno-abete", "Abete", c, &diel(srgb(215, 180, 130), 0.6));
    add("legno-laccato-nero", "Legno laccato nero", c, &diel(srgb(12, 12, 12), 0.08));
    add("compensato-betulla", "Compensato di betulla", c, &diel(srgb(220, 195, 155), 0.6));
    add("mdf", "MDF grezzo", c, &diel(srgb(170, 135, 95), 0.8));

    // --- Tessuti e pelle ---
    let c = "Tessuti e pelle";
    add("tessuto-grigio", "Tessuto grigio", c, &|m| {
        diel(srgb(110, 110, 112), 1.0)(m);
        m.specular = 0.2;
    });
    add("feltro-nero", "Feltro nero", c, &|m| {
        diel(srgb(25, 25, 26), 1.0)(m);
        m.specular = 0.15;
    });
    add("pelle-nera", "Pelle nera", c, &diel(srgb(20, 18, 17), 0.45));
    add("pelle-cuoio", "Cuoio naturale", c, &diel(srgb(140, 85, 45), 0.5));
    add("alcantara", "Alcantara grigio", c, &|m| {
        diel(srgb(70, 70, 72), 0.95)(m);
        m.specular = 0.2;
    });
    add(
        "carbonio-lucido",
        "Fibra di carbonio lucida (tinta, senza trama)",
        c,
        &diel(srgb(18, 18, 20), 0.12),
    );

    // --- Emissivi e generici ---
    let c = "Emissivi";
    let emit = |col: [f32; 3], strength: f32| {
        move |m: &mut Material| {
            m.base_color = [1.0; 3];
            m.emission = col;
            m.emission_strength = strength;
            m.roughness = 1.0;
        }
    };
    add("emissivo", "Superficie emissiva", c, &emit([1.0; 3], 10.0));
    add(
        "led-bianco-freddo",
        "LED bianco freddo (6500 K)",
        c,
        &emit(crate::light::kelvin_to_rgb(6500.0), 30.0),
    );
    add(
        "led-bianco-caldo",
        "LED bianco caldo (3000 K)",
        c,
        &emit(crate::light::kelvin_to_rgb(3000.0), 30.0),
    );
    add("led-rosso", "LED rosso", c, &emit([1.0, 0.02, 0.02], 25.0));
    add("led-blu", "LED blu", c, &emit([0.02, 0.1, 1.0], 25.0));
    add("led-verde", "LED verde", c, &emit([0.05, 1.0, 0.1], 25.0));
    add("display", "Display acceso (bianco)", c, &emit([1.0; 3], 3.0));
    if let Some(dir) = builtin_textures_dir() {
        for m in &mut v {
            attach_builtin_maps(m, dir);
        }
        // Predefiniti scritti come dati, nelle cartelle accanto a `materiali`.
        if let Some(parent) = dir.parent() {
            for extra in DATA_PRESET_DIRS {
                for m in data_presets(&parent.join(extra)) {
                    if !v.iter().any(|p| p.id == m.id) {
                        v.push(m);
                    }
                }
            }
        }
    }
    v
}

/// Cartelle di predefiniti "a dati": una sottocartella per materiale con `materiale.json` e le
/// mappe. `materiali-blenderkit`: metalli BlenderKit (licenza Royalty Free, solo uso personale;
/// da togliere prima di distribuire il programma).
pub const DATA_PRESET_DIRS: [&str; 1] = ["materiali-blenderkit"];

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

/// Materiali predefiniti con mappe (cartella `materiali/<id>/` del programma, preparate da
/// `forge_render_core/examples/materiali_predefiniti.rs`): id, lato della ripetizione in metri,
/// colore base e rugosità (moltiplicano le mappe; senza mappa di rugosità vale il numero).
const MAPPED: [(&str, f32, [f32; 3], f32); 3] = [
    // Poly Haven Oak Veneer 01 (CC0). Venatura un po' più fitta del foglio vero (1,83 m):
    // a 1024 px resta nitida anche sui prodotti piccoli.
    ("legno-rovere", 0.6, [1.0; 3], 0.8),
    // Poly Haven Leather White (CC0), tinta nera dal colore base.
    ("pelle-nera", 0.3, [0.06; 3], 1.0),
    // Generato (venature su fondo bianco), lucidato: rugosità costante.
    ("marmo-bianco", 0.35, [1.0; 3], 0.12),
];

/// Cartella delle mappe dei materiali predefiniti: `FORGE_MATERIALS_DIR`, oppure `materiali`
/// accanto al programma (pacchetti), oppure `assets/materiali` risalendo dal programma
/// (compilazione locale e test). `None` se non c'è: i predefiniti restano senza mappe.
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

/// Collega colore, rugosità e normal dalla cartella del materiale, se ci sono.
fn attach_builtin_maps(m: &mut Material, dir: &std::path::Path) {
    let Some(&(_, scale, color, roughness)) = MAPPED.iter().find(|e| e.0 == m.id.0) else {
        return;
    };
    let file = |name: &str| Some(dir.join(&m.id.0).join(name)).filter(|p| p.is_file());
    m.base_color_texture = file("colore.jpg");
    m.roughness_texture = file("rugosita.jpg");
    m.normal_texture = file("normal.jpg");
    if !m.has_textures() {
        return;
    }
    m.base_color = color;
    m.roughness = roughness;
    m.mapping = TextureMapping {
        projection: Projection::Box,
        scale,
        ..TextureMapping::default()
    };
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

    /// Predefiniti a dati (assets/materiali-blenderkit): letti con le mappe nella loro cartella,
    /// senza id doppi. Se la cartella è stata tolta (versione da distribuire) non c'è niente.
    #[test]
    fn data_presets_load_with_their_maps() {
        let Some(root) = builtin_textures_dir().and_then(std::path::Path::parent) else {
            return;
        };
        let dir = root.join(DATA_PRESET_DIRS[0]);
        if !dir.is_dir() {
            return;
        }
        let data = data_presets(&dir);
        assert!(!data.is_empty(), "nessun predefinito letto da {}", dir.display());
        for m in &data {
            assert!(m.has_textures(), "{} senza mappe", m.id.0);
            for t in m.texture_slots().into_iter().flatten() {
                assert!(t.is_file() && t.starts_with(&dir), "{}: {}", m.id.0, t.display());
            }
        }
        let all = presets();
        let mut ids: Vec<_> = all.iter().map(|m| &m.id).collect();
        let n = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), n, "id doppi");
        assert!(
            data.iter().all(|d| all.iter().any(|m| m.id == d.id)),
            "predefiniti a dati nell'elenco"
        );
    }

    /// Le mappe dei predefiniti (assets/materiali) si trovano anche dai test e sono complete.
    #[test]
    fn builtin_maps_are_attached() {
        let dir = builtin_textures_dir().expect("assets/materiali");
        let p = presets();
        for (id, scale, ..) in MAPPED {
            let m = p.iter().find(|m| m.id.0 == id).unwrap_or_else(|| panic!("manca {id}"));
            let color = m.base_color_texture.as_ref().unwrap_or_else(|| panic!("{id} senza colore"));
            assert!(color.starts_with(dir) && color.is_file());
            assert_eq!(m.mapping.scale, scale);
        }
        let rovere = p.iter().find(|m| m.id.0 == "legno-rovere").unwrap();
        assert!(rovere.roughness_texture.is_some() && rovere.normal_texture.is_some());
        // Gli altri restano senza mappe.
        assert!(!p.iter().find(|m| m.id.0 == "acciaio-lucidato").unwrap().has_textures());
    }
}
