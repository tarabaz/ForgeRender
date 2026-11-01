//! Scene demo (File → Scene demo), divise per che cosa mostrano. Aprendone una compare una scheda
//! che spiega le funzioni usate e come rifare la scena da zero (`ui::demo_info`).
//!
//! Le scene generate si costruiscono qui (nessun file esterno); quelle con modelli importati sono
//! progetti nella cartella `samples` accanto al programma, copiati alla prima apertura nella
//! cartella dei dati dell'utente (il programma scrive accanto ai modelli le texture estratte, e in
//! Program Files non potrebbe).

use crate::i18n::{tk, tr};
use forge_math::{Mat4, Quat, Vec3};
use forge_render_core::{CausticSettings, ColorMode, RenderSettings};
use forge_scene::{primitives, AddedKind, AddedRecord, Camera, ComponentId, Environment, Light, LightKind, Material, MaterialId, Scene};
use std::path::{Path, PathBuf};

/// Gruppi del sottomenu, nell'ordine.
pub const CATEGORIES: [&str; 4] = [
    tk("Luce e dispersione"),
    tk("Caustiche"),
    tk("Effetti volumetrici"),
    tk("Studio e materiali"),
];

pub enum DemoSource {
    /// Scena costruita dal programma, con le impostazioni di render che servono.
    Generated(fn() -> (Scene, RenderSettings)),
    /// Progetto in `samples/` (percorso relativo).
    Project(&'static str),
    /// La scena di test (File → Nuova scena di test).
    TestScene,
}

pub struct Demo {
    /// Indice in [`CATEGORIES`].
    pub category: usize,
    pub title: &'static str,
    /// Che cosa si vede e che cosa mette alla prova.
    pub summary: &'static str,
    /// Funzioni del motore usate dalla scena.
    pub features: &'static [&'static str],
    /// Come rifarla da zero: materiali, luci, impostazioni.
    pub steps: &'static [&'static str],
    pub source: DemoSource,
}

pub static DEMOS: [Demo; 6] = [
    Demo {
        category: 0,
        title: tk("Prisma e luce (Dark Side of the Moon)"),
        summary: tk(
            "Un fascio di luce bianca entra in un prisma di vetro su fondo nero ed esce aperto in un \
             arcobaleno. L'arcobaleno non è disegnato: ogni lunghezza d'onda si rifrange con il suo indice \
             e prende la sua strada.",
        ),
        features: &[
            tk("Render spettrale: la luce viaggia per lunghezze d'onda"),
            tk("Dispersione del vetro (numero di Abbe 8, molto più di un vetro comune)"),
            tk("Luce laser con il fascio visibile nell'aria"),
            tk("Ambiente tenue e invisibile: sfondo nero, il prisma si vede per i riflessi"),
        ],
        steps: &[
            tk("Aggiungi un prisma e dagli un vetro: Trasmissione 1, Ruvidità 0, IOR 1,6, Dispersione (Abbe) 8"),
            tk("Aggiungi una luce Laser, diametro 12 mm, fascio visibile 0,08, puntata su una faccia del prisma"),
            tk("Inclina il laser di circa 23° verso l'alto: è l'angolo di deviazione minima, il fascio dentro il prisma resta parallelo alla base"),
            tk("Ambiente uniforme a 0,15 e non visibile alla camera"),
            tk("Render, Colore: Spettrale; almeno 1024 campioni"),
        ],
        source: DemoSource::Generated(crate::presets::prism_demo_scene),
    },
    Demo {
        category: 0,
        title: tk("Laser nel diamante"),
        summary: tk(
            "Un diamante taglio brillante sospeso, colpito da un laser bianco e da uno spot stretto. Il \
             laser entra dalla tavola, rimbalza sulle facce del padiglione (riflessione totale) e riesce \
             diviso nei colori; lo spot sparge sul pavimento e sul fondale il \"fuoco\" del diamante.",
        ),
        features: &[
            tk("Diamante vero: IOR 2,417, Abbe 55, taglio brillante con 57 facce"),
            tk("Laser con il fascio visibile, anche dentro la pietra"),
            tk("Caustiche (photon mapping) dallo spot: le macchie colorate attorno"),
            tk("Render spettrale: dispersione per lunghezza d'onda"),
        ],
        steps: &[
            tk("Gemma: un brillante con il materiale Trasmissione 1, Ruvidità 0, IOR 2,417, Dispersione (Abbe) 55"),
            tk("Pavimento e fondale bianchi opachi, ambiente quasi spento"),
            tk("Luce Laser verso la tavola della pietra (diametro 3 mm, fascio visibile 0,15)"),
            tk("Spot con cono stretto (5°) puntato sulla pietra con \"Punta su\": la luce passa quasi tutta dal diamante"),
            tk("Render: Colore Spettrale, Caustiche attive, Rimbalzi massimi 24"),
        ],
        source: DemoSource::Generated(diamond_laser_scene),
    },
    Demo {
        category: 1,
        title: tk("Ciondoli di cristallo al sole"),
        summary: tk(
            "Pendenti di cristallo appesi davanti a una parete, colpiti da un fascio di sole dalla \
             finestra: sulle pareti compaiono gli arcobaleni che si vedono in casa con i cristalli \
             appesi alle finestre.",
        ),
        features: &[
            tk("Caustiche con il photon mapping: i fotoni attraversano i cristalli e si posano sulle pareti"),
            tk("Dispersione: ogni fotone ha la sua lunghezza d'onda, le macchie sono arcobaleni"),
            tk("Gemme procedurali a facce piane (pendenti a goccia e brillante)"),
        ],
        steps: &[
            tk("Pendenti di cristallo: Trasmissione 1, Ruvidità 0, IOR 1,56, Dispersione (Abbe) 30"),
            tk("Fili sottili in metallo cromato per appenderli"),
            tk("Stanza bianca opaca: pavimento e due pareti"),
            tk("Spot caldo (5800 K) lontano e con il cono che copre solo i cristalli: fa da raggio di sole dalla finestra"),
            tk("Render: Colore Spettrale, Caustiche attive con almeno 500 k fotoni"),
        ],
        source: DemoSource::Generated(pendants_sun_scene),
    },
    Demo {
        category: 1,
        title: tk("Vetri e caustiche sul tavolo"),
        summary: tk(
            "Una sfera di vetro chiaro, una di vetro ambra e un cubo di vetro su un tavolo bianco, con uno \
             spot di lato: la sfera concentra la luce in un punto brillante dentro l'ombra, il vetro ambra \
             fa una caustica colorata. Prova a spegnere Caustiche: con lo spot quelle macchie spariscono.",
        ),
        features: &[
            tk("Caustiche con il photon mapping anche da luci puntiformi e spot"),
            tk("Vetro colorato: tinta a ogni attraversamento"),
            tk("Raggio di raccolta che si restringe a ogni passata: caustiche sempre più nitide"),
            tk("Render RGB: le caustiche funzionano anche senza lo spettrale"),
        ],
        steps: &[
            tk("Sfere e cubo con il vetro: Trasmissione 1, Ruvidità 0, IOR 1,5 (per l'ambra un Colore arancio)"),
            tk("Tavolo bianco opaco, ambiente debole"),
            tk("Spot a 45° sopra il tavolo con \"Punta su\" verso la sfera"),
            tk("Render: Caustiche attive; prova Raggio iniziale 2–5 mm per caustiche più nitide"),
        ],
        source: DemoSource::Generated(glass_caustics_scene),
    },
    Demo {
        category: 2,
        title: tk("Cocktail con ghiaccio secco"),
        summary: tk(
            "Una coppa da cocktail con la nebbia del ghiaccio secco che trabocca e scende lungo lo stelo, \
             su un bancone in noce con bottiglie e lucine sfocate sullo sfondo.",
        ),
        features: &[
            tk("Nebbia simulata (fluido) con la sua densità in un volume"),
            tk("Vetro con liquido dentro (priorità dei volumi)"),
            tk("Profondità di campo e bokeh"),
            tk("Modelli importati con texture"),
        ],
        steps: &[
            tk("Importa i modelli (bicchiere, bancone, bottiglie) e assegna vetro e liquido; il liquido un filo più grande della cavità e con priorità più alta"),
            tk("Effetti: aggiungi la nebbia \"Cocktail: colata e riccioli\" con l'emettitore poco sotto il bordo"),
            tk("Camera: 42 mm, f/2.8, messa a fuoco sul bicchiere"),
            tk("Render GPU, 16 rimbalzi, 1024 campioni"),
        ],
        source: DemoSource::Project("scene/cocktail-ghiaccio-secco/cocktail-ghiaccio-secco.frproject"),
    },
    Demo {
        category: 3,
        title: tk("Scena di test"),
        summary: tk(
            "Sfere, cubo e toro con materiali diversi su un fondale, con luci da studio: la scena per \
             provare materiali, luci e impostazioni di render.",
        ),
        features: &[
            tk("Materiali di base: plastica, metallo, vetro"),
            tk("Luci da studio e fondale"),
        ],
        steps: &[
            tk("Aggiungi le forme dal menu Aggiungi e un fondale"),
            tk("Luci: Prepara scena, aspetto Studio"),
        ],
        source: DemoSource::TestScene,
    },
];

/// Cartella `samples`: accanto al programma; durante lo sviluppo quella del repository.
pub fn samples_root() -> Option<PathBuf> {
    let beside = std::env::current_exe().ok()?.parent()?.join("samples");
    if beside.is_dir() {
        return Some(beside);
    }
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../samples");
    dev.is_dir().then_some(dev)
}

/// La demo è disponibile? (I progetti richiedono la cartella `samples`.)
pub fn available(d: &Demo) -> bool {
    match d.source {
        DemoSource::Project(rel) => samples_root().is_some_and(|r| r.join(rel).is_file()),
        _ => true,
    }
}

/// Progetto demo pronto da aprire: la sua cartella copiata (una volta sola) nella cartella dei
/// dati dell'utente, accanto alle impostazioni.
pub fn writable_project(rel: &str) -> anyhow::Result<PathBuf> {
    let root = samples_root().ok_or_else(|| anyhow::anyhow!("cartella samples non trovata"))?;
    let src = root.join(rel);
    let rel_path = Path::new(rel);
    let first = rel_path.components().next().map(|c| c.as_os_str().to_owned());
    let second = rel_path.components().nth(1).map(|c| c.as_os_str().to_owned());
    let (Some(a), Some(b)) = (first, second) else {
        return Ok(src);
    };
    let base = crate::prefs::file_path()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("scene-demo");
    let dst_dir = base.join(&a).join(&b);
    let dst = base.join(rel);
    if !dst.is_file() {
        copy_dir(&root.join(&a).join(&b), &dst_dir)?;
    }
    Ok(dst)
}

fn copy_dir(from: &Path, to: &Path) -> anyhow::Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)? {
        let e = e?;
        let p = e.path();
        if p.is_dir() {
            copy_dir(&p, &to.join(e.file_name()))?;
        } else {
            std::fs::copy(&p, to.join(e.file_name()))?;
        }
    }
    Ok(())
}

// ------------------------------------------------------------------ Costruzione delle scene

fn mat(s: &mut Scene, id: &str, name: &str, f: impl FnOnce(&mut Material)) {
    let mut m = Material {
        id: MaterialId::new(id),
        name: name.into(),
        ..Default::default()
    };
    f(&mut m);
    s.materials.upsert(m);
}

fn glass(m: &mut Material, ior: f32, abbe: f32) {
    m.base_color = [1.0; 3];
    m.transmission = 1.0;
    m.roughness = 0.0;
    m.ior = ior;
    m.abbe = abbe;
}

/// Oggetto della scena come oggetto aggiunto (menu Aggiungi): il progetto salvato lo ricrea.
/// `xf` posiziona la forma come la crea la sua primitiva (gemme e prisma centrati, le altre con la
/// base a y = 0); lo spostamento che mette gemme e prisma in piedi sul piano si toglie qui.
pub(crate) fn node(s: &mut Scene, id: &str, name: &str, kind: AddedKind, xf: Mat4, material: &str) {
    let raw = match kind {
        AddedKind::Prism { side, length } => Some(primitives::prism(side, length)),
        AddedKind::Gem { diameter } => Some(primitives::brilliant(diameter)),
        AddedKind::Pendant { width, length } => Some(primitives::crystal_drop(width, length)),
        _ => None,
    };
    let lift = raw.map_or(0.0, |m| -m.positions.iter().map(|p| p.y).fold(f32::INFINITY, f32::min));
    let r = AddedRecord {
        id: ComponentId(format!("demo/{id}")),
        name: name.into(),
        kind,
        placement: xf * Mat4::from_translation(Vec3::new(0.0, -lift, 0.0)),
    };
    s.instantiate_added(&r);
    s.assignments.insert(r.id.clone(), MaterialId::new(material));
    s.added.push(r);
}

fn spot(name: &str, position: Vec3, target: Vec3, angle_deg: f32, intensity: f32) -> Light {
    Light {
        kind: LightKind::Spot {
            inner_angle_deg: angle_deg * 0.6,
            outer_angle_deg: angle_deg,
        },
        target,
        ..Light::point(name, position, intensity)
    }
}

fn caustic_settings(color_mode: ColorMode, photons_k: u32, samples: u32, bounces: u32) -> RenderSettings {
    RenderSettings {
        color_mode,
        target_samples: samples,
        max_bounces: bounces,
        environment_visible: false,
        caustics: CausticSettings {
            enabled: true,
            photons_k,
            radius_mm: 0.0,
        },
        ..Default::default()
    }
}

/// Diamante sospeso con un laser e uno spot stretto; pavimento e fondale bianchi.
pub fn diamond_laser_scene() -> (Scene, RenderSettings) {
    let mut s = Scene::default();
    mat(&mut s, "demo-diamante", tr("Diamante"), |m| glass(m, 2.417, 55.0));
    mat(&mut s, "demo-bianco", tr("Bianco opaco"), |m| {
        m.base_color = [0.75; 3];
        m.roughness = 1.0;
    });
    let centre = Vec3::new(0.0, 0.25, 0.0);
    // Tavola rivolta verso il laser (in alto a sinistra), un po' girata verso la camera.
    let tilt = Quat::from_rotation_z(0.35) * Quat::from_rotation_x(0.25);
    node(
        &mut s,
        "diamante",
        tr("Diamante"),
        AddedKind::Gem { diameter: 0.12 },
        Mat4::from_rotation_translation(tilt, centre),
        "demo-diamante",
    );
    node(
        &mut s,
        "pavimento",
        tr("Pavimento"),
        AddedKind::Floor { size: 4.0 },
        Mat4::IDENTITY,
        "demo-bianco",
    );
    node(
        &mut s,
        "fondale",
        tr("Fondale"),
        AddedKind::Wall { width: 4.0, height: 2.0 },
        Mat4::from_translation(Vec3::new(0.0, 0.0, -0.7)),
        "demo-bianco",
    );
    s.environment = Environment::Uniform {
        color: [1.0; 3],
        strength: 0.03,
    };
    let mut laser = Light::point(tr("Laser bianco"), Vec3::new(-0.9, 0.75, 0.25), 0.6);
    laser.kind = LightKind::Laser {
        diameter_mm: 3.0,
        source: forge_scene::LaserSource::White,
    };
    laser.target = centre + Vec3::new(0.0, 0.03, 0.0);
    laser.beam_density = 0.15;
    s.lights.push(laser);
    let mut sp = spot(tr("Spot sul diamante"), Vec3::new(0.6, 1.4, 0.6), centre, 4.5, 25.0);
    sp.aim = Some(ComponentId("demo/diamante".into()));
    s.lights.push(sp);
    s.camera = Camera {
        position: Vec3::new(0.1, 0.45, 0.95),
        target: Vec3::new(0.0, 0.18, 0.0),
        up: Vec3::Y,
        vfov_deg: 32.0,
        ..Default::default()
    };
    (s, caustic_settings(ColorMode::Spectral, 500, 2048, 24))
}

/// Pendenti di cristallo appesi davanti alle pareti di una stanza bianca, con un fascio di sole.
pub fn pendants_sun_scene() -> (Scene, RenderSettings) {
    let mut s = Scene::default();
    mat(&mut s, "demo-cristallo", tr("Cristallo"), |m| glass(m, 1.56, 30.0));
    mat(&mut s, "demo-bianco", tr("Bianco opaco"), |m| {
        m.base_color = [0.8; 3];
        m.roughness = 1.0;
    });
    mat(&mut s, "demo-cromo", tr("Cromo"), |m| {
        m.base_color = [0.9; 3];
        m.metallic = 1.0;
        m.roughness = 0.1;
    });
    use std::f32::consts::FRAC_PI_2;
    node(
        &mut s,
        "pavimento",
        tr("Pavimento"),
        AddedKind::Floor { size: 4.0 },
        Mat4::IDENTITY,
        "demo-bianco",
    );
    node(
        &mut s,
        "parete-fondo",
        tr("Parete di fondo"),
        AddedKind::Wall { width: 4.0, height: 3.0 },
        Mat4::from_translation(Vec3::new(0.0, 0.0, -0.8)),
        "demo-bianco",
    );
    node(
        &mut s,
        "parete-sinistra",
        tr("Parete sinistra"),
        AddedKind::Wall { width: 4.0, height: 3.0 },
        Mat4::from_rotation_translation(Quat::from_rotation_y(FRAC_PI_2), Vec3::new(-1.2, 0.0, 0.0)),
        "demo-bianco",
    );
    // Pendenti a quote e rotazioni diverse, ognuno col suo filo.
    let hang = 1.6;
    let gems: [(Vec3, f32, bool); 5] = [
        (Vec3::new(-0.18, 1.02, 0.0), 0.3, false),
        (Vec3::new(-0.06, 0.92, 0.08), 1.1, false),
        (Vec3::new(0.06, 1.08, -0.05), 2.0, true),
        (Vec3::new(0.18, 0.96, 0.04), 0.7, false),
        (Vec3::new(0.0, 1.18, 0.1), 2.6, false),
    ];
    for (i, (p, rot, round)) in gems.into_iter().enumerate() {
        let (kind, top) = if round {
            // Brillante appeso di taglio (tavola verso la luce).
            (AddedKind::Gem { diameter: 0.09 }, 0.045)
        } else {
            (AddedKind::Pendant { width: 0.07, length: 0.15 }, 0.15 * 0.22)
        };
        let q = if round {
            Quat::from_rotation_y(rot) * Quat::from_rotation_x(FRAC_PI_2)
        } else {
            Quat::from_rotation_y(rot) * Quat::from_rotation_z(0.08)
        };
        node(
            &mut s,
            &format!("pendente-{i}"),
            &format!("{} {}", tr("Pendente"), i + 1),
            kind,
            Mat4::from_rotation_translation(q, p),
            "demo-cristallo",
        );
        let len = hang - (p.y + top);
        node(
            &mut s,
            &format!("filo-{i}"),
            &format!("{} {}", tr("Filo"), i + 1),
            AddedKind::Cylinder {
                radius: 0.0008,
                height: len,
            },
            Mat4::from_translation(Vec3::new(p.x, p.y + top, p.z)),
            "demo-cromo",
        );
    }
    s.environment = Environment::Uniform {
        color: [0.85, 0.9, 1.0],
        strength: 0.08,
    };
    let mut sun = spot(
        tr("Sole dalla finestra"),
        Vec3::new(2.6, 1.6, 0.3),
        Vec3::new(0.0, 1.05, 0.03),
        8.0,
        40.0,
    );
    sun.color = forge_scene::light::kelvin_to_rgb(5800.0);
    s.lights.push(sun);
    s.camera = Camera {
        position: Vec3::new(1.1, 1.15, 1.5),
        target: Vec3::new(-0.8, 0.95, -0.35),
        up: Vec3::Y,
        vfov_deg: 55.0,
        ..Default::default()
    };
    (s, caustic_settings(ColorMode::Spectral, 1000, 2048, 24))
}

/// Sfera di vetro chiaro, sfera ambra e cubo di vetro su un tavolo bianco, spot di lato.
pub fn glass_caustics_scene() -> (Scene, RenderSettings) {
    let mut s = Scene::default();
    mat(&mut s, "demo-vetro", tr("Vetro chiaro"), |m| glass(m, 1.5, 0.0));
    mat(&mut s, "demo-ambra", tr("Vetro ambra"), |m| {
        glass(m, 1.5, 0.0);
        m.base_color = [1.0, 0.6, 0.2];
    });
    mat(&mut s, "demo-tavolo", tr("Tavolo bianco"), |m| {
        m.base_color = [0.8; 3];
        m.roughness = 0.9;
    });
    node(
        &mut s,
        "tavolo",
        tr("Tavolo"),
        AddedKind::Floor { size: 3.0 },
        Mat4::IDENTITY,
        "demo-tavolo",
    );
    let r = 0.08;
    node(
        &mut s,
        "sfera",
        tr("Sfera di vetro"),
        AddedKind::Sphere { radius: r },
        Mat4::IDENTITY,
        "demo-vetro",
    );
    node(
        &mut s,
        "sfera-ambra",
        tr("Sfera ambra"),
        AddedKind::Sphere { radius: r * 0.75 },
        Mat4::from_translation(Vec3::new(0.24, 0.0, -0.1)),
        "demo-ambra",
    );
    node(
        &mut s,
        "cubo",
        tr("Cubo di vetro"),
        AddedKind::Box { size: Vec3::splat(0.1) },
        Mat4::from_rotation_translation(Quat::from_rotation_y(0.5), Vec3::new(-0.24, 0.0, -0.08)),
        "demo-vetro",
    );
    s.environment = Environment::Uniform {
        color: [1.0; 3],
        strength: 0.15,
    };
    let mut sp = spot(tr("Spot"), Vec3::new(-0.7, 0.75, 0.55), Vec3::new(0.0, 0.05, -0.05), 50.0, 4.0);
    sp.aim = Some(ComponentId("demo/sfera".into()));
    s.lights.push(sp);
    s.camera = Camera {
        position: Vec3::new(0.35, 0.55, 0.9),
        target: Vec3::new(0.05, 0.02, -0.12),
        up: Vec3::Y,
        vfov_deg: 40.0,
        ..Default::default()
    };
    (s, caustic_settings(ColorMode::Rgb, 300, 1024, 16))
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_render_core::{Accumulator, RenderBackend, RenderScene};

    /// Ogni demo ha titolo, descrizione, funzioni e passi, e i gruppi esistono.
    #[test]
    fn demos_are_described() {
        for d in &DEMOS {
            assert!(d.category < CATEGORIES.len());
            assert!(!d.title.is_empty() && !d.summary.is_empty());
            assert!(!d.features.is_empty() && !d.steps.is_empty(), "{}", d.title);
        }
        for (c, name) in CATEGORIES.iter().enumerate() {
            assert!(DEMOS.iter().any(|d| d.category == c), "gruppo vuoto {name}");
        }
    }

    /// Salvate e riaperte, le scene generate tornano uguali: ogni oggetto ricreato con la sua
    /// posizione e il suo materiale (prima i pezzi delle demo non finivano nel progetto).
    #[test]
    fn generated_demos_survive_save_and_load() {
        let dir = std::env::temp_dir().join(format!("forge_demo_rt_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for d in &DEMOS {
            let DemoSource::Generated(build) = d.source else { continue };
            let (scene, st) = build();
            let path = dir.join("demo.frproject");
            forge_project::save(&path, &scene, &forge_project::ProjectMeta::default(), &st).unwrap();
            let l = forge_project::load(&path).unwrap();
            let bounds = |s: &Scene| {
                let mut v: Vec<(String, [i32; 6])> = s
                    .nodes
                    .iter()
                    .filter(|n| n.mesh.is_some())
                    .map(|n| {
                        let i = s.find(&n.id).unwrap();
                        let b = s.exact_bounds_of(i);
                        let q = |x: f32| (x * 1e4).round() as i32;
                        (
                            n.id.0.clone(),
                            [q(b.min.x), q(b.min.y), q(b.min.z), q(b.max.x), q(b.max.y), q(b.max.z)],
                        )
                    })
                    .collect();
                v.sort();
                v
            };
            assert!(!bounds(&scene).is_empty(), "{}", d.title);
            assert_eq!(bounds(&scene), bounds(&l.scene), "{}", d.title);
            for id in scene.assignments.keys() {
                assert!(l.scene.find(id).is_some(), "{}: manca {}", d.title, id.0);
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Il progetto del cocktail è nel repository.
    #[test]
    fn project_demos_exist() {
        for d in &DEMOS {
            if let DemoSource::Project(rel) = d.source {
                assert!(available(d), "manca {rel}");
            }
        }
    }

    /// Le scene generate con le caustiche hanno vetri lisci e luci, e i fotoni si posano: a basso
    /// campionamento l'immagine con le caustiche è più luminosa che senza.
    #[test]
    fn caustic_demos_produce_caustics() {
        for build in [diamond_laser_scene, pendants_sun_scene, glass_caustics_scene] {
            let (scene, st) = build();
            assert!(st.caustics.enabled);
            let rs = RenderScene::compile(&scene);
            assert!(forge_render_cpu::photons::specular_bounds(&rs).is_some());
            let small = RenderSettings {
                width: 48,
                height: 32,
                caustics: CausticSettings {
                    photons_k: 40,
                    ..st.caustics
                },
                ..st.clone()
            };
            let map = forge_render_cpu::caustic_map(&rs, &small, 0).expect("mappa");
            assert!(!map.photons.is_empty(), "nessun fotone posato");
            let render = |st: &RenderSettings| {
                let mut be = forge_render_cpu::CpuBackend::new(0).unwrap();
                let mut acc = Accumulator::new(st.width, st.height);
                let mut pass = vec![[0.0; 3]; acc.pixel_count()];
                let cancel = std::sync::atomic::AtomicBool::new(false);
                for k in 0..4 {
                    be.render_pass(&rs, st, k, 1, &mut pass, &cancel).unwrap();
                    acc.add_pass(&pass);
                }
                acc.resolve().iter().map(|p| (p[0] + p[1] + p[2]) as f64).sum::<f64>()
            };
            let off = render(&RenderSettings {
                caustics: CausticSettings::default(),
                ..small.clone()
            });
            let on = render(&small);
            eprintln!("caustiche: senza {off:.2}, con {on:.2}, fotoni {}", map.photons.len());
            assert!(on > off, "le caustiche devono aggiungere luce");
        }
    }
}
