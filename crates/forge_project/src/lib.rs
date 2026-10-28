//! Formato di progetto `.frproject` (JSON leggibile e versionato).
//!
//! Il progetto **non contiene la geometria**: salva i riferimenti ai file
//! sorgente (percorso relativo al progetto + percorso assoluto di riserva) e
//! le opzioni di importazione, poi tutto ciò che appartiene al renderer
//! (materiali, assegnazioni per id stabile, visibilità, luci, camera,
//! ambiente, impostazioni di rendering). I file CAD originali non vengono mai
//! modificati. Le texture (Fase 3) saranno referenziate esternamente con la
//! stessa strategia relativo/assoluto.

use anyhow::{bail, Context};
use forge_import::{import_file, ImportOptions};
use forge_render_core::RenderSettings;
use forge_scene::{Camera, ComponentId, Environment, Light, Material, MaterialId, Scene};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const FORMAT_ID: &str = "forgerender-project";
pub const SCHEMA_VERSION: u32 = 1;
pub const EXTENSION: &str = "frproject";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SourceRef {
    /// Percorso relativo alla cartella del progetto (se possibile).
    pub relative_path: Option<PathBuf>,
    pub absolute_path: PathBuf,
    pub import_options: ImportOptions,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ComponentState {
    pub visible: bool,
    /// Trasformazione impostata dall'utente (assente se identità).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform: Option<forge_scene::UserTransform>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectFile {
    pub format: String,
    pub schema_version: u32,
    pub app_version: String,
    /// Scena procedurale di test invece di file sorgente.
    #[serde(default)]
    pub test_scene: bool,
    #[serde(default)]
    pub sources: Vec<SourceRef>,
    #[serde(default)]
    pub materials: Vec<Material>,
    #[serde(default)]
    pub assignments: BTreeMap<ComponentId, MaterialId>,
    pub default_material: MaterialId,
    #[serde(default)]
    pub components: BTreeMap<ComponentId, ComponentState>,
    #[serde(default)]
    pub lights: Vec<Light>,
    pub environment: Environment,
    pub camera: Camera,
    #[serde(default)]
    pub render_cameras: Vec<forge_scene::RenderCamera>,
    #[serde(default)]
    pub active_camera: Option<usize>,
    #[serde(default)]
    pub render: RenderSettings,
}

/// Metadati del progetto aperto che non stanno nella [`Scene`].
#[derive(Clone, Debug, Default)]
pub struct ProjectMeta {
    pub path: Option<PathBuf>,
    pub sources: Vec<SourceRef>,
    pub test_scene: bool,
}

#[derive(Debug, Default)]
pub struct LoadReport {
    pub missing_sources: Vec<PathBuf>,
    /// Componenti citati nel progetto ma non più presenti nei file sorgente
    /// (rinominati o rimossi). Le loro assegnazioni vengono conservate.
    pub missing_components: Vec<ComponentId>,
    pub warnings: Vec<String>,
}

pub struct LoadedProject {
    pub scene: Scene,
    pub meta: ProjectMeta,
    pub settings: RenderSettings,
    pub report: LoadReport,
}

pub fn source_ref(project_dir: Option<&Path>, file: &Path, opts: ImportOptions) -> SourceRef {
    let absolute_path = std::path::absolute(file).unwrap_or_else(|_| file.to_path_buf());
    let relative_path = project_dir.and_then(|d| relative_to(&absolute_path, d));
    SourceRef {
        relative_path,
        absolute_path,
        import_options: opts,
    }
}

/// Percorso relativo semplice (solo se il file sta sotto la cartella del progetto
/// o in una sua sorella, risalendo con `..`).
fn relative_to(file: &Path, dir: &Path) -> Option<PathBuf> {
    let dir = std::path::absolute(dir).ok()?;
    let fc: Vec<_> = file.components().collect();
    let dc: Vec<_> = dir.components().collect();
    let common = fc.iter().zip(&dc).take_while(|(a, b)| a == b).count();
    if common == 0 {
        return None; // dischi diversi su Windows
    }
    let mut p = PathBuf::new();
    for _ in common..dc.len() {
        p.push("..");
    }
    for c in &fc[common..] {
        p.push(c);
    }
    Some(p)
}

pub fn to_project_file(scene: &Scene, meta: &ProjectMeta, settings: &RenderSettings) -> ProjectFile {
    let project_dir = meta.path.as_deref().and_then(Path::parent);
    let sources = meta
        .sources
        .iter()
        .map(|s| source_ref(project_dir, &s.absolute_path, s.import_options.clone()))
        .collect();
    ProjectFile {
        format: FORMAT_ID.into(),
        schema_version: SCHEMA_VERSION,
        app_version: env!("CARGO_PKG_VERSION").into(),
        test_scene: meta.test_scene,
        sources,
        materials: scene.materials.materials.clone(),
        assignments: scene.assignments.clone(),
        default_material: scene.default_material.clone(),
        components: scene
            .nodes
            .iter()
            .map(|n| {
                (
                    n.id.clone(),
                    ComponentState {
                        visible: n.visible,
                        transform: (!n.user.is_identity()).then_some(n.user),
                    },
                )
            })
            .collect(),
        lights: scene.lights.clone(),
        environment: scene.environment.clone(),
        camera: scene.camera.clone(),
        render_cameras: scene.render_cameras.clone(),
        active_camera: scene.active_camera,
        render: settings.clone(),
    }
}

/// Salvataggio atomico: scrittura su file temporaneo, flush su disco, rinomina.
pub fn save(path: &Path, scene: &Scene, meta: &ProjectMeta, settings: &RenderSettings) -> anyhow::Result<()> {
    let meta = ProjectMeta {
        path: Some(path.to_path_buf()),
        ..meta.clone()
    };
    let pf = to_project_file(scene, &meta, settings);
    let json = serde_json::to_vec_pretty(&pf)?;
    let mut tmp_name = path.file_name().context("percorso del progetto non valido")?.to_os_string();
    tmp_name.push(".tmp");
    let tmp = path.with_file_name(tmp_name);
    {
        let mut f = std::fs::File::create(&tmp).with_context(|| format!("creazione di {}", tmp.display()))?;
        f.write_all(&json)?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, path).with_context(|| format!("salvataggio di {}", path.display()))?;
    tracing::info!("progetto salvato: {}", path.display());
    Ok(())
}

pub fn parse(json: &str) -> anyhow::Result<ProjectFile> {
    let v: serde_json::Value = serde_json::from_str(json).context("il file non è un JSON valido")?;
    if v.get("format").and_then(|f| f.as_str()) != Some(FORMAT_ID) {
        bail!("il file non è un progetto ForgeRender");
    }
    let ver = v.get("schema_version").and_then(|x| x.as_u64()).unwrap_or(0) as u32;
    if ver > SCHEMA_VERSION {
        bail!("progetto creato con una versione più recente (schema {ver}, supportato fino a {SCHEMA_VERSION})");
    }
    // Qui verranno inserite le migrazioni tra versioni dello schema (ver < SCHEMA_VERSION).
    serde_json::from_value(v).context("struttura del progetto non valida")
}

/// Se un file esterno (HDRI, texture) non esiste più nel percorso salvato, lo
/// cerca accanto al progetto o nelle sottocartelle indicate.
fn relocate_asset(path: &mut PathBuf, project_dir: Option<&Path>, subdirs: &[&str], report: &mut LoadReport) {
    if path.is_file() {
        return;
    }
    let Some(name) = path.file_name().map(PathBuf::from) else { return };
    let found = std::iter::once(name.clone())
        .chain(subdirs.iter().map(|d| Path::new(d).join(&name)))
        .filter_map(|r| project_dir.map(|d| d.join(r)))
        .find(|p| p.is_file());
    match found {
        Some(p) => *path = p,
        None => report.warnings.push(format!("file non trovato: {}", path.display())),
    }
}

pub fn load(path: &Path) -> anyhow::Result<LoadedProject> {
    let text = std::fs::read_to_string(path).with_context(|| format!("lettura di {}", path.display()))?;
    let pf = parse(&text)?;
    let project_dir = path.parent();
    let mut report = LoadReport::default();
    let mut scene = if pf.test_scene { Scene::test_scene() } else { Scene::default() };
    // Le assegnazioni del progetto sostituiscono quelle predefinite.
    scene.assignments.clear();

    let mut sources = Vec::new();
    for src in &pf.sources {
        let candidates: Vec<PathBuf> = src
            .relative_path
            .iter()
            .filter_map(|r| project_dir.map(|d| d.join(r)))
            .chain(std::iter::once(src.absolute_path.clone()))
            .collect();
        let Some(found) = candidates.iter().find(|p| p.is_file()) else {
            report.missing_sources.push(src.absolute_path.clone());
            continue;
        };
        match import_file(&mut scene, found, &src.import_options) {
            Ok(r) => {
                report.warnings.extend(r.warnings);
                sources.push(SourceRef {
                    absolute_path: found.clone(),
                    ..src.clone()
                });
            }
            Err(e) => report.warnings.push(format!("{}: {e}", found.display())),
        }
    }

    for m in pf.materials {
        scene.materials.upsert(m);
    }
    scene.materials.ensure_default();
    scene.default_material = pf.default_material;
    for (id, mat) in pf.assignments {
        if scene.find(&id).is_none() {
            report.missing_components.push(id.clone());
        }
        if scene.materials.get(&mat).is_none() {
            report.warnings.push(format!("materiale {mat} assegnato a {id} non trovato"));
        }
        scene.assignments.insert(id, mat);
    }
    for (id, st) in pf.components {
        if let Some(i) = scene.find(&id) {
            scene.nodes[i].visible = st.visible;
            if let Some(t) = st.transform {
                scene.nodes[i].user = t;
            }
        }
    }
    scene.lights = pf.lights;
    scene.environment = pf.environment;
    // HDRI e texture spostate insieme al progetto: si cercano nella cartella del progetto.
    if let forge_scene::Environment::Hdri { path: hdri, .. } = &mut scene.environment {
        relocate_asset(hdri, project_dir, &["hdri"], &mut report);
    }
    for m in &mut scene.materials.materials {
        for slot in m.texture_slots_mut().into_iter().flatten() {
            relocate_asset(slot, project_dir, &["textures", "texture"], &mut report);
        }
    }
    scene.camera = pf.camera;
    scene.active_camera = pf.active_camera.filter(|&i| i < pf.render_cameras.len());
    scene.render_cameras = pf.render_cameras;

    if !report.missing_components.is_empty() {
        tracing::warn!(
            "{} componenti del progetto non trovati nei file sorgente",
            report.missing_components.len()
        );
    }
    for s in &report.missing_sources {
        tracing::warn!("file sorgente non trovato: {}", s.display());
    }
    Ok(LoadedProject {
        scene,
        meta: ProjectMeta {
            path: Some(path.to_path_buf()),
            sources,
            test_scene: pf.test_scene,
        },
        settings: pf.render,
        report,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_math::Vec3;

    fn tmpdir() -> PathBuf {
        let d = std::env::temp_dir().join(format!("forge_project_{}_{}", std::process::id(), rand_suffix()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn rand_suffix() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    }

    #[test]
    fn roundtrip_with_sources_materials_and_visibility() {
        let dir = tmpdir();
        let obj = dir.join("pezzo.obj");
        std::fs::write(
            &obj,
            "o Staffa\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\no Vite\nv 0 0 1\nv 1 0 1\nv 0 1 1\nf 1 2 3\n",
        )
        .unwrap();
        let opts = ImportOptions::default();
        let mut scene = Scene::default();
        import_file(&mut scene, &obj, &opts).unwrap();
        let staffa = ComponentId("pezzo.obj/Staffa".into());
        let vite = ComponentId("pezzo.obj/Vite".into());
        scene.assignments.insert(staffa.clone(), MaterialId::new("ottone"));
        let mut custom = Material {
            id: MaterialId::new("mio"),
            name: "Mio".into(),
            roughness: 0.123,
            ..Default::default()
        };
        custom.base_color = [0.1, 0.2, 0.3];
        scene.materials.upsert(custom.clone());
        scene.assignments.insert(vite.clone(), MaterialId::new("mio"));
        let vi = scene.find(&vite).unwrap();
        scene.nodes[vi].visible = false;
        scene.lights.push(Light::point("P", Vec3::new(1.0, 2.0, 3.0), 5.0));
        scene.camera.vfov_deg = 27.0;
        scene.render_cameras.push(forge_scene::RenderCamera {
            name: "Frontale".into(),
            camera: forge_scene::Camera {
                position: Vec3::new(0.0, 0.5, 2.0),
                vfov_deg: 35.0,
                ..Default::default()
            },
        });
        scene.active_camera = Some(0);
        let si = scene.find(&staffa).unwrap();
        scene.nodes[si].user = forge_scene::UserTransform {
            translation: Vec3::new(0.1, 0.2, 0.3),
            rotation_deg: Vec3::new(0.0, 90.0, 0.0),
            scale: Vec3::splat(1.5),
        };
        let settings = RenderSettings {
            width: 640,
            height: 480,
            target_samples: 512,
            ..Default::default()
        };
        let meta = ProjectMeta {
            path: None,
            sources: vec![source_ref(None, &obj, opts)],
            test_scene: false,
        };

        let pfile = dir.join("progetto.frproject");
        save(&pfile, &scene, &meta, &settings).unwrap();
        let loaded = load(&pfile).unwrap();
        assert!(loaded.report.missing_components.is_empty());
        assert!(loaded.report.missing_sources.is_empty());
        let s2 = &loaded.scene;
        assert_eq!(s2.resolve_material(s2.find(&staffa).unwrap()), &MaterialId::new("ottone"));
        assert_eq!(s2.materials.get(&MaterialId::new("mio")), Some(&custom));
        assert!(!s2.nodes[s2.find(&vite).unwrap()].visible);
        assert_eq!(s2.nodes[s2.find(&staffa).unwrap()].user, scene.nodes[si].user);
        assert!((s2.bounds_of(s2.find(&staffa).unwrap()).center() - scene.bounds_of(si).center()).length() < 1e-6);
        assert_eq!(s2.lights, scene.lights);
        assert_eq!(s2.camera, scene.camera);
        assert_eq!(s2.render_cameras, scene.render_cameras);
        assert_eq!(s2.active_camera, Some(0));
        assert_eq!(s2.render_camera().vfov_deg, 35.0);
        assert_eq!(loaded.settings, settings);

        // Il progetto funziona anche dopo aver spostato l'intera cartella (percorso relativo).
        let moved = dir.with_file_name(format!("{}_spostata", dir.file_name().unwrap().to_str().unwrap()));
        std::fs::rename(&dir, &moved).unwrap();
        let loaded = load(&moved.join("progetto.frproject")).unwrap();
        assert!(loaded.report.missing_sources.is_empty());
        assert_eq!(loaded.scene.triangle_count(), 2);
        std::fs::remove_dir_all(moved).ok();
    }

    #[test]
    fn missing_components_and_sources_are_reported() {
        let dir = tmpdir();
        let mut scene = Scene::default();
        scene
            .assignments
            .insert(ComponentId("sparito/pezzo".into()), MaterialId::new("rame"));
        let meta = ProjectMeta {
            path: None,
            sources: vec![source_ref(None, &dir.join("non_esiste.stl"), ImportOptions::default())],
            test_scene: false,
        };
        let p = dir.join("p.frproject");
        save(&p, &scene, &meta, &RenderSettings::default()).unwrap();
        let l = load(&p).unwrap();
        assert_eq!(l.report.missing_sources.len(), 1);
        assert_eq!(l.report.missing_components, vec![ComponentId("sparito/pezzo".into())]);
        // L'assegnazione orfana viene conservata (il componente potrebbe ricomparire).
        assert!(l.scene.assignments.contains_key(&ComponentId("sparito/pezzo".into())));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn rejects_foreign_and_future_files() {
        assert!(parse("{}").is_err());
        assert!(parse("non json").is_err());
        let future = format!("{{\"format\":\"{FORMAT_ID}\",\"schema_version\":999}}");
        assert!(parse(&future).unwrap_err().to_string().contains("più recente"));
    }

    #[test]
    fn test_scene_roundtrip() {
        let dir = tmpdir();
        let scene = Scene::test_scene();
        let p = dir.join("t.frproject");
        save(
            &p,
            &scene,
            &ProjectMeta {
                test_scene: true,
                ..Default::default()
            },
            &RenderSettings::default(),
        )
        .unwrap();
        let l = load(&p).unwrap();
        assert_eq!(l.scene.triangle_count(), scene.triangle_count());
        assert_eq!(l.scene.assignments, scene.assignments);
        std::fs::remove_dir_all(dir).ok();
    }
}
