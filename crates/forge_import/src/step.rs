//! Importazione STEP (tramite il convertitore `forgerender-step`, basato su Open
//! CASCADE) e SolidWorks (SolidWorks esporta uno STEP via COM, poi stessa pipeline).
//!
//! La conversione gira in un processo separato: un file malformato che manda in
//! crash Open CASCADE non chiude l'applicazione.

use crate::frmesh::FrMesh;
use crate::{up_matrix, ImportError, ImportOptions, ImportReport};
use forge_math::{Mat4, Vec3};
use forge_scene::{material::linear_to_srgb, ComponentId, Material, MaterialId, Mesh, Scene};
use std::collections::HashMap;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const CONVERTER: &str = if cfg!(windows) {
    "forgerender-step.exe"
} else {
    "forgerender-step"
};

/// Percorso del convertitore: variabile `FORGE_STEP_CONVERTER`, oppure accanto all'eseguibile.
pub fn converter_path() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("FORGE_STEP_CONVERTER") {
        return Some(PathBuf::from(p));
    }
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    // Accanto all'eseguibile, oppure una cartella sopra (binari dei test in target/*/deps).
    [dir.join(CONVERTER), dir.join("..").join(CONVERTER)]
        .into_iter()
        .find(|p| p.is_file())
}

fn hide_console(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

fn temp_path(stem: &str, ext: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("forgerender_{stem}_{}_{nanos}.{ext}", std::process::id()))
}

/// File a cui rimanda uno STEP salvato "a più file" (assieme con le parti in file separati,
/// `DOCUMENT_FILE('pezzo.STEP', …)`): percorsi relativi alla cartella dello STEP.
pub fn external_refs(path: &Path) -> Vec<PathBuf> {
    let Ok(bytes) = std::fs::read(path) else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&bytes);
    let mut out: Vec<PathBuf> = Vec::new();
    let mut rest = &text[..];
    while let Some(i) = rest.find("DOCUMENT_FILE") {
        rest = &rest[i + 13..];
        let Some(open) = rest.find('\x27') else { break };
        let after = &rest[open + 1..];
        let Some(close) = after.find('\x27') else { break };
        let name = after[..close].trim();
        let ext = Path::new(name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if matches!(ext.as_str(), "step" | "stp") && !name.contains("..") {
            let p = PathBuf::from(name);
            if !out.contains(&p) {
                out.push(p);
            }
        }
        rest = &after[close + 1..];
    }
    out
}

/// Impronta di una conversione: contenuto dello STEP e dei pezzi a cui rimanda, e qualità.
/// Uguale = la conversione salvata vale ancora.
pub fn fingerprint(path: &Path, opts: &ImportOptions) -> Option<String> {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |b: &[u8]| {
        let (chunks, tail) = b.as_chunks::<8>();
        for c in chunks {
            h = (h ^ u64::from_le_bytes(*c)).wrapping_mul(0x0000_0100_0000_01b3).rotate_left(29);
        }
        for &x in tail {
            h = (h ^ x as u64).wrapping_mul(0x0000_0100_0000_01b3);
        }
        h ^= b.len() as u64;
    };
    eat(&std::fs::read(path).ok()?);
    let dir = path.parent().unwrap_or(Path::new("."));
    for r in external_refs(path) {
        match std::fs::read(dir.join(&r)) {
            Ok(b) => eat(&b),
            Err(_) => eat(b"mancante"),
        }
    }
    eat(opts.step_quality.arg().as_bytes());
    if let Some([l, a]) = opts.step_custom {
        eat(format!("{l}|{a}").as_bytes());
    }
    Some(format!("{h:016x}"))
}

/// Mancano dei pezzi a cui lo STEP rimanda (convertirlo darebbe un assieme vuoto o a metà).
pub fn missing_refs(path: &Path) -> bool {
    let dir = path.parent().unwrap_or(Path::new("."));
    external_refs(path).iter().any(|r| !dir.join(r).is_file())
}

/// Cartella della cache delle conversioni (accanto alle impostazioni dell'utente).
pub fn cache_dir() -> PathBuf {
    std::env::var_os("FORGE_STEP_CACHE")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("LOCALAPPDATA").map(|p| PathBuf::from(p).join("ForgeRender").join("conversioni-step")))
        .unwrap_or_else(|| std::env::temp_dir().join("forgerender-conversioni-step"))
}

/// Conversione già fatta per questa impronta (cache dell'utente).
pub fn cached_conversion(key: &str) -> PathBuf {
    cache_dir().join(format!("{key}.frmesh"))
}

/// Converte un file STEP (o SolidWorks) nel modello intermedio. Operazione lenta:
/// va eseguita fuori dal thread dell'interfaccia. `progress` riceve messaggi testuali.
/// Una conversione già fatta (stesso contenuto, stessa qualità) si rilegge dalla cache.
pub fn convert(path: &Path, opts: &ImportOptions, progress: impl FnMut(String)) -> Result<FrMesh, ImportError> {
    convert_cancellable(path, opts, &std::sync::atomic::AtomicBool::new(false), progress)
}

/// Come [`convert`], e `cancel` ferma il convertitore.
pub fn convert_cancellable(
    path: &Path,
    opts: &ImportOptions,
    cancel: &std::sync::atomic::AtomicBool,
    mut progress: impl FnMut(String),
) -> Result<FrMesh, ImportError> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    if !path.is_file() {
        return Err(ImportError::Converter(format!("file non trovato: {}", path.display())));
    }
    // Assiemi SolidWorks: le parti stanno in altri file che l'impronta non vede, niente cache.
    let key = if matches!(ext.as_str(), "step" | "stp") {
        fingerprint(path, opts)
    } else {
        None
    };
    if let Some(k) = &key {
        let c = cached_conversion(k);
        if let Ok(mut model) = FrMesh::read(&c) {
            progress("conversione già fatta: riletta dalla cache".into());
            model.header.source = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            return Ok(model);
        }
    }
    let model = convert_uncached(path, &ext, opts, cancel, &mut progress)?;
    if let Some(k) = &key {
        let c = cached_conversion(k);
        let _ = std::fs::create_dir_all(cache_dir());
        let tmp = c.with_extension("tmp");
        if model.write(&tmp).is_ok() {
            let _ = std::fs::rename(&tmp, &c);
        }
    }
    Ok(model)
}

fn convert_uncached(
    path: &Path,
    ext: &str,
    opts: &ImportOptions,
    cancel: &std::sync::atomic::AtomicBool,
    progress: &mut impl FnMut(String),
) -> Result<FrMesh, ImportError> {
    let (step_path, cleanup) = if ext == "sldprt" || ext == "sldasm" {
        progress("esportazione STEP da SolidWorks…".into());
        let out = temp_path("solidworks", "step");
        solidworks_to_step(path, &out)?;
        (out.clone(), Some(out))
    } else {
        (path.to_path_buf(), None)
    };
    let result = run_converter(&step_path, opts, cancel, progress);
    if let Some(p) = cleanup {
        let _ = std::fs::remove_file(p);
    }
    let mut model = result?;
    model.header.source = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    Ok(model)
}

/// Argomenti della qualità per il convertitore.
fn quality_args(opts: &ImportOptions) -> Vec<String> {
    match opts.step_custom {
        Some([l, a]) => vec!["--deflection".into(), l.to_string(), "--angular".into(), a.to_string()],
        None => vec!["--quality".into(), opts.step_quality.arg().into()],
    }
}

fn run_converter(
    step: &Path,
    opts: &ImportOptions,
    cancel: &std::sync::atomic::AtomicBool,
    progress: &mut impl FnMut(String),
) -> Result<FrMesh, ImportError> {
    let conv = converter_path().ok_or_else(|| {
        ImportError::Converter(format!(
            "convertitore STEP non incluso in questa versione ({CONVERTER} deve trovarsi accanto a ForgeRender).\n\
             Da SolidWorks esporta in \"Extended Reality (*.glb)\": porta anche materiali e struttura dell'assieme"
        ))
    })?;
    let out = temp_path("step", "frmesh");
    let mut cmd = Command::new(&conv);
    cmd.arg(step)
        .arg(&out)
        .args(quality_args(opts))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = hide_console(&mut cmd)
        .spawn()
        .map_err(|e| ImportError::Converter(format!("avvio di {} fallito: {e}", conv.display())))?;
    // Le righe arrivano da un thread: intanto si guarda se la conversione va fermata.
    if let Some(stdout) = child.stdout.take() {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(stdout).lines().map_while(Result::ok) {
                let _ = tx.send(line);
            }
        });
        loop {
            match rx.recv_timeout(std::time::Duration::from_millis(100)) {
                Ok(line) => progress(line.trim_start_matches("progresso: ").to_string()),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
            if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                let _ = child.kill();
                let _ = child.wait();
                let _ = std::fs::remove_file(&out);
                return Err(ImportError::Converter("conversione fermata".into()));
            }
        }
    }
    let output = child
        .wait_with_output()
        .map_err(|e| ImportError::Converter(format!("conversione STEP interrotta: {e}")))?;
    if !output.status.success() {
        let _ = std::fs::remove_file(&out);
        let msg = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(ImportError::Converter(if msg.is_empty() {
            format!("il convertitore STEP è terminato in modo anomalo ({})", output.status)
        } else {
            msg
        }));
    }
    let model = FrMesh::read(&out).map_err(|e| ImportError::Converter(format!("risultato della conversione non leggibile: {e}")));
    let _ = std::fs::remove_file(&out);
    model
}

fn solidworks_to_step(input: &Path, out: &Path) -> Result<(), ImportError> {
    crate::solidworks::export(input, out)
}

fn hex(c: [f32; 3]) -> String {
    let s = c.map(|v| (linear_to_srgb(v.clamp(0.0, 1.0)) * 255.0 + 0.5) as u8);
    format!("{:02x}{:02x}{:02x}", s[0], s[1], s[2])
}

/// Inserisce il modello convertito nella scena: un nodo radice per il file e
/// l'albero dell'assieme sotto di esso. Gli id sono i percorsi dei nomi di
/// istanza (`file.step/Assieme/Staffa-1/...`), stabili tra reimportazioni.
pub fn insert(scene: &mut Scene, file_name: &str, model: FrMesh, opts: &ImportOptions) -> Result<ImportReport, ImportError> {
    let t0 = std::time::Instant::now();
    if model.header.nodes.is_empty() {
        return Err(ImportError::Empty);
    }
    let unit = if model.header.unit_m > 0.0 {
        model.header.unit_m as f32
    } else {
        0.001
    };
    let z_up = opts.up.is_z_up("step");
    let root_xf = up_matrix(z_up) * Mat4::from_scale(Vec3::splat(unit));
    let root = scene.add_node(None, ComponentId(file_name.to_string()), file_name, root_xf, None);

    let mut warnings = Vec::new();
    let mut mesh_map: HashMap<u32, usize> = HashMap::new();
    let mut triangles = 0usize;
    let mut removed = 0usize;
    for (k, m) in model.meshes.into_iter().enumerate() {
        let mut mesh = Mesh {
            name: format!("mesh-{k}"),
            positions: m.positions.into_iter().map(Vec3::from_array).collect(),
            normals: m.normals.into_iter().map(|n| Vec3::from_array(n).normalize_or(Vec3::Y)).collect(),
            uvs: Vec::new(),
            indices: m.indices,
        };
        removed += mesh.remove_invalid_triangles();
        if !mesh.indices.is_empty() {
            mesh_map.insert(k as u32, scene.add_mesh(mesh));
        }
    }

    let mut node_map: Vec<usize> = Vec::with_capacity(model.header.nodes.len());
    let mut materials_created = Vec::new();
    let mut components = 0;
    for rec in &model.header.nodes {
        let parent = match rec.parent {
            Some(p) if (p as usize) < node_map.len() => node_map[p as usize],
            Some(_) => {
                warnings.push(format!("{}: genitore non valido, collegato alla radice", rec.name));
                root
            }
            None => root,
        };
        let id = ComponentId(format!("{}/{}", scene.nodes[parent].id.0, rec.name.replace('/', "∕")));
        let m: Vec<f32> = rec.matrix.iter().map(|&v| v as f32).collect();
        let local = Mat4::from_cols_slice(&m);
        let mesh = rec.mesh.and_then(|m| mesh_map.get(&m).copied());
        if let Some(mi) = mesh {
            triangles += scene.meshes[mi].triangle_count();
            components += 1;
        }
        let idx = scene.add_node(Some(parent), id, rec.name.clone(), local, mesh);
        if let Some(c) = rec.color {
            let h = hex(c);
            let mat_id = MaterialId::new(format!("step:{h}"));
            if scene.materials.get(&mat_id).is_none() {
                scene.materials.upsert(Material {
                    id: mat_id.clone(),
                    name: format!("Colore CAD #{h}"),
                    base_color: c,
                    roughness: 0.4,
                    category: "Importati".into(),
                    ..Default::default()
                });
                materials_created.push(format!("#{h}"));
            }
            let node_id = scene.nodes[idx].id.clone();
            scene.assignments.entry(node_id).or_insert(mat_id);
        }
        node_map.push(idx);
    }
    if triangles == 0 {
        scene.remove_subtree(root);
        return Err(ImportError::Empty);
    }
    if removed > 0 {
        warnings.push(format!("rimossi {removed} triangoli degeneri"));
    }
    scene.refresh_pivots();
    tracing::info!(
        "importato {file_name}: {} nodi, {} pezzi, {} mesh uniche, {} triangoli, unità {} m",
        node_map.len(),
        components,
        mesh_map.len(),
        triangles,
        unit
    );
    Ok(ImportReport {
        root,
        components,
        triangles,
        removed_triangles: removed,
        generated_normals: false,
        materials_created,
        warnings,
        duration: t0.elapsed(),
        camera: None,
        lights: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ogni parametro della finestra arriva davvero al convertitore e a una conversione sua.
    #[test]
    fn custom_tessellation_reaches_the_converter() {
        let preset = ImportOptions::default();
        assert_eq!(quality_args(&preset), ["--quality", "media"]);
        let custom = ImportOptions {
            step_custom: Some([0.002, 0.25]),
            ..Default::default()
        };
        assert_eq!(quality_args(&custom), ["--deflection", "0.002", "--angular", "0.25"]);
        let dir = std::env::temp_dir().join(format!("forge_step_key_{}.step", std::process::id()));
        std::fs::write(&dir, b"ISO-10303-21;").unwrap();
        let (a, b) = (fingerprint(&dir, &preset), fingerprint(&dir, &custom));
        let _ = std::fs::remove_file(&dir);
        assert_ne!(a, b, "parametri diversi, conversione diversa");
    }
    use crate::frmesh::{Header, MeshData, NodeRecord};

    fn mat(t: [f64; 3]) -> [f64; 16] {
        let mut m = [0.0; 16];
        m[0] = 1.0;
        m[5] = 1.0;
        m[10] = 1.0;
        m[15] = 1.0;
        m[12] = t[0];
        m[13] = t[1];
        m[14] = t[2];
        m
    }

    /// Assieme con due istanze dello stesso pezzo: mesh condivisa, id per istanza,
    /// unità millimetri, colori trasformati in materiali.
    #[test]
    fn insert_assembly_with_instances() {
        let tri = MeshData {
            positions: vec![[0.0; 3], [10.0, 0.0, 0.0], [0.0, 10.0, 0.0]],
            normals: vec![[0.0, 0.0, 1.0]; 3],
            indices: vec![[0, 1, 2]],
        };
        let n = |name: &str, parent: Option<u32>, t: [f64; 3], mesh: Option<u32>, color: Option<[f32; 3]>| NodeRecord {
            name: name.into(),
            parent,
            matrix: mat(t),
            color,
            mesh,
        };
        let model = FrMesh {
            header: Header {
                unit_m: 0.001,
                source: "a.step".into(),
                nodes: vec![
                    n("Assieme", None, [0.0; 3], None, None),
                    n("Vite-1", Some(0), [0.0; 3], None, None),
                    n("Vite-1 (corpo)", Some(1), [0.0; 3], Some(0), Some([0.8, 0.1, 0.1])),
                    n("Vite-2", Some(0), [100.0, 0.0, 0.0], None, None),
                    n("Vite-2 (corpo)", Some(3), [0.0; 3], Some(0), Some([0.8, 0.1, 0.1])),
                ],
                meshes: vec![],
            },
            meshes: vec![tri],
        };
        let mut s = Scene::default();
        let r = insert(&mut s, "a.step", model, &ImportOptions::default()).unwrap();
        assert_eq!(r.components, 2);
        assert_eq!(s.meshes.len(), 1, "il pezzo ripetuto deve condividere la mesh");
        let v2 = s.find(&ComponentId("a.step/Assieme/Vite-2/Vite-2 (corpo)".into())).unwrap();
        let b = s.bounds_of(v2);
        // 100 mm di traslazione → 0.1 m; il triangolo è 10 mm.
        assert!((b.min.x - 0.1).abs() < 1e-5 && (b.extent().x - 0.01).abs() < 1e-5, "{b:?}");
        assert!(s.resolve_material(v2).0.starts_with("step:"), "{}", s.resolve_material(v2));
        assert_eq!(r.materials_created.len(), 1);
    }
}
