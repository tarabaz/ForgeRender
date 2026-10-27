//! Importazione di file geometrici nella [`Scene`].
//!
//! Formati supportati in questa versione: OBJ (con gruppi/oggetti e colori
//! diffusi dal file .mtl) e STL (binario e ASCII). Ogni file importato diventa
//! un nodo radice con un figlio per oggetto; gli id dei componenti sono
//! costruiti come `"<nome file>/<nome oggetto>"`, stabili tra reimportazioni
//! dello stesso file.

pub mod frmesh;
mod obj;
pub mod step;
mod stl;

use forge_math::Mat4;
use forge_scene::{ComponentId, LengthUnit, Material, NodeIndex, Scene};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ImportOptions {
    /// Unità di misura del file (STL e OBJ non la dichiarano; lo STEP sì e la ignora).
    pub unit: LengthUnit,
    /// Angolo di piega per il calcolo delle normali quando mancano (gradi).
    pub crease_angle_deg: f32,
    /// Asse verticale del file. La scena usa Y verso l'alto.
    pub up: UpAxis,
    /// Qualità della triangolazione dei formati CAD (STEP).
    pub step_quality: StepQuality,
    /// Dopo l'importazione appoggia il modello a quota 0 (trasformazione utente, non distruttiva).
    pub drop_to_floor: bool,
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            unit: LengthUnit::Millimeter,
            crease_angle_deg: 40.0,
            up: UpAxis::Auto,
            step_quality: StepQuality::Media,
            drop_to_floor: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UpAxis {
    /// In base al formato: STL Z-up (slicer e CAD), OBJ e STEP Y-up (SolidWorks, Blender).
    #[default]
    Auto,
    Y,
    Z,
}

impl UpAxis {
    pub fn is_z_up(self, ext: &str) -> bool {
        match self {
            UpAxis::Auto => ext == "stl",
            UpAxis::Y => false,
            UpAxis::Z => true,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            UpAxis::Auto => "Automatico",
            UpAxis::Y => "Y verso l'alto",
            UpAxis::Z => "Z verso l'alto",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum StepQuality {
    Bassa,
    #[default]
    Media,
    Alta,
}

impl StepQuality {
    pub fn arg(self) -> &'static str {
        match self {
            StepQuality::Bassa => "bassa",
            StepQuality::Media => "media",
            StepQuality::Alta => "alta",
        }
    }
}

fn up_matrix(z_up: bool) -> Mat4 {
    if z_up {
        Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2)
    } else {
        Mat4::IDENTITY
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("formato non supportato: .{0}")]
    UnsupportedFormat(String),
    #[error("il formato .{0} richiede un convertitore esterno non ancora disponibile")]
    NeedsConverter(String),
    #[error("{0}")]
    Converter(String),
    #[error("il file non contiene geometria triangolabile")]
    Empty,
    #[error("errore di lettura: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Parse(String),
}

/// Risultato di un'importazione.
#[derive(Debug)]
pub struct ImportReport {
    pub root: NodeIndex,
    pub components: usize,
    pub triangles: usize,
    pub removed_triangles: usize,
    pub generated_normals: bool,
    pub materials_created: Vec<String>,
    pub warnings: Vec<String>,
    pub duration: std::time::Duration,
}

/// Oggetto letto da un file, prima dell'inserimento nella scena.
pub(crate) struct ImportedObject {
    pub name: String,
    pub mesh: forge_scene::Mesh,
    pub material: Option<Material>,
}

pub const SUPPORTED_EXTENSIONS: [&str; 4] = ["obj", "stl", "step", "stp"];
/// Formati SolidWorks: richiedono SolidWorks installato (esportazione STEP via COM).
pub const SOLIDWORKS_EXTENSIONS: [&str; 2] = ["sldprt", "sldasm"];

pub fn import_file(scene: &mut Scene, path: &Path, opts: &ImportOptions) -> Result<ImportReport, ImportError> {
    let t0 = std::time::Instant::now();
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("modello").to_string();
    let mut warnings = Vec::new();
    let objects = match ext.as_str() {
        "obj" => obj::read(path, &mut warnings)?,
        "stl" => stl::read(path)?,
        "step" | "stp" | "sldprt" | "sldasm" => {
            let model = step::convert(path, opts, |m| tracing::info!("{m}"))?;
            let report = step::insert(scene, &file_name, model, opts)?;
            return Ok(ImportReport {
                duration: t0.elapsed(),
                ..report
            });
        }
        "ply" => return Err(ImportError::NeedsConverter(ext)),
        _ => return Err(ImportError::UnsupportedFormat(ext)),
    };
    let mut opts = opts.clone();
    opts.up = if opts.up.is_z_up(&ext) { UpAxis::Z } else { UpAxis::Y };
    let report = insert_objects(scene, &file_name, objects, &opts, warnings)?;
    let report = ImportReport {
        duration: t0.elapsed(),
        ..report
    };
    tracing::info!(
        "importato {}: {} componenti, {} triangoli in {:.0} ms",
        path.display(),
        report.components,
        report.triangles,
        report.duration.as_secs_f64() * 1e3
    );
    for w in &report.warnings {
        tracing::warn!("{file_name}: {w}");
    }
    Ok(report)
}

pub(crate) fn insert_objects(
    scene: &mut Scene,
    file_name: &str,
    objects: Vec<ImportedObject>,
    opts: &ImportOptions,
    mut warnings: Vec<String>,
) -> Result<ImportReport, ImportError> {
    let scale = opts.unit.to_meters();
    let root_xf = up_matrix(opts.up == UpAxis::Z);
    let root = scene.add_node(None, ComponentId(file_name.to_string()), file_name, root_xf, None);
    let mut components = 0;
    let mut triangles = 0;
    let mut removed = 0;
    let mut generated_normals = false;
    let mut materials_created = Vec::new();
    let mut used_names = std::collections::HashMap::<String, usize>::new();

    for obj in objects {
        let mut mesh = obj.mesh;
        let report = mesh.validate();
        if report.non_finite_vertices > 0 {
            warnings.push(format!("{}: {} vertici non finiti", obj.name, report.non_finite_vertices));
        }
        removed += mesh.remove_invalid_triangles();
        if mesh.indices.is_empty() {
            warnings.push(format!("{}: nessun triangolo valido, ignorato", obj.name));
            continue;
        }
        mesh.scale(scale);
        if !mesh.has_normals() {
            mesh.compute_normals(opts.crease_angle_deg);
            generated_normals = true;
        }
        triangles += mesh.triangle_count();
        // Nomi duplicati nello stesso file: suffisso progressivo deterministico.
        let count = used_names.entry(obj.name.clone()).or_insert(0);
        *count += 1;
        let local = if *count == 1 {
            obj.name.clone()
        } else {
            format!("{}#{}", obj.name, count)
        };
        let id = ComponentId(format!("{file_name}/{local}"));
        mesh.name = local.clone();
        let mi = scene.add_mesh(mesh);
        scene.add_node(Some(root), id.clone(), local, Mat4::IDENTITY, Some(mi));
        if let Some(mat) = obj.material {
            if scene.materials.get(&mat.id).is_none() {
                materials_created.push(mat.name.clone());
                scene.materials.upsert(mat.clone());
            }
            scene.assignments.entry(id).or_insert(mat.id);
        }
        components += 1;
    }
    if components == 0 {
        scene.remove_subtree(root);
        return Err(ImportError::Empty);
    }
    if removed > 0 {
        warnings.push(format!("rimossi {removed} triangoli degeneri o non validi"));
    }
    scene.refresh_pivots();
    Ok(ImportReport {
        root,
        components,
        triangles,
        removed_triangles: removed,
        generated_normals,
        materials_created,
        warnings,
        duration: Default::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn tmp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("forge_import_{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d.join(name)
    }

    const CUBE_OBJ: &str = "mtllib cubo.mtl\no Base\nv 0 0 0\nv 10 0 0\nv 10 10 0\nv 0 10 0\nusemtl Rosso\nf 1 2 3 4\no Coperchio\nv 0 0 10\nv 10 0 10\nv 10 10 10\nf 5 6 7\n";
    const CUBE_MTL: &str = "newmtl Rosso\nKd 0.8 0.1 0.1\n";

    #[test]
    fn obj_import_with_groups_units_and_materials() {
        let p = tmp("cubo.obj");
        std::fs::write(&p, CUBE_OBJ).unwrap();
        std::fs::write(p.with_file_name("cubo.mtl"), CUBE_MTL).unwrap();
        let mut s = Scene::default();
        let r = import_file(&mut s, &p, &ImportOptions::default()).unwrap();
        assert_eq!(r.components, 2);
        assert_eq!(r.triangles, 3);
        assert!(r.generated_normals);
        assert!(s.find(&ComponentId("cubo.obj/Base".into())).is_some());
        let b = s.bounds();
        // 10 mm → 0.01 m.
        assert!((b.extent().max_element() - 0.01).abs() < 1e-6, "{b:?}");
        let base = s.find(&ComponentId("cubo.obj/Base".into())).unwrap();
        let m = s.materials.get(s.resolve_material(base)).unwrap();
        assert_eq!(m.name, "Rosso");
    }

    #[test]
    fn stl_binary_and_ascii() {
        let ascii = "solid t\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nvertex 1 0 0\nvertex 0 1 0\nendloop\nendfacet\nendsolid t\n";
        let p = tmp("tri.stl");
        std::fs::write(&p, ascii).unwrap();
        let mut s = Scene::default();
        let r = import_file(&mut s, &p, &ImportOptions::default()).unwrap();
        assert_eq!(r.triangles, 1);

        // STL binario: header 80 byte, conteggio, 1 triangolo da 50 byte.
        let mut bin = vec![0u8; 80];
        bin.extend_from_slice(&1u32.to_le_bytes());
        for v in [0f32, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0] {
            bin.extend_from_slice(&v.to_le_bytes());
        }
        bin.extend_from_slice(&[0, 0]);
        let p2 = tmp("tri_bin.stl");
        std::fs::File::create(&p2).unwrap().write_all(&bin).unwrap();
        let mut s = Scene::default();
        let r = import_file(&mut s, &p2, &ImportOptions::default()).unwrap();
        assert_eq!(r.triangles, 1);
    }

    #[test]
    fn unsupported_and_converter_formats() {
        let mut s = Scene::default();
        assert!(matches!(
            import_file(&mut s, Path::new("x.dwg"), &ImportOptions::default()),
            Err(ImportError::UnsupportedFormat(_))
        ));
        assert!(matches!(
            import_file(&mut s, Path::new("x.SLDASM"), &ImportOptions::default()),
            Err(ImportError::Converter(_))
        ));
    }
}
