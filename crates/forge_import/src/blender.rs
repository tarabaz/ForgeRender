//! File `.blend`: Blender (se installato) viene avviato in background per esportare
//! la scena in glTF; le luci ad area, che glTF non supporta, vengono esportate a
//! parte in JSON. Il file `.blend` originale non viene modificato.

use crate::gltf_import::AreaLightRecord;
use crate::ImportError;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub struct BlendExport {
    pub glb: PathBuf,
    pub area_lights: Vec<AreaLightRecord>,
}

/// Script eseguito dentro Blender (`blender -b file.blend --python script.py -- out.glb out.json`).
const SCRIPT: &str = r#"
import bpy, json, sys
from mathutils import Matrix
argv = sys.argv[sys.argv.index("--") + 1:]
out_glb, out_json = argv[0], argv[1]
bpy.ops.export_scene.gltf(filepath=out_glb, export_format='GLB', export_cameras=True,
                          export_lights=True, export_apply=True, export_yup=True)
# Da Blender (Z in alto) a glTF (Y in alto): (x, y, z) -> (x, z, -y)
C = Matrix(((1, 0, 0, 0), (0, 0, 1, 0), (0, -1, 0, 0), (0, 0, 0, 1)))
lights = []
for o in bpy.context.scene.objects:
    if o.type == 'LIGHT' and o.data.type == 'AREA' and o.visible_get():
        m = C @ o.matrix_world
        d = o.data
        h = d.size_y if d.shape in ('RECTANGLE', 'ELLIPSE') else d.size
        lights.append({"name": o.name, "matrix": [m[r][c] for c in range(4) for r in range(4)],
                       "width": d.size, "height": h, "color": list(d.color), "power": d.energy})
with open(out_json, "w") as f:
    json.dump(lights, f)
"#;

/// Cerca l'eseguibile di Blender: variabile `FORGE_BLENDER`, `PATH`, cartelle standard.
pub fn blender_path() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("FORGE_BLENDER") {
        return Some(PathBuf::from(p));
    }
    let exe = if cfg!(windows) { "blender.exe" } else { "blender" };
    if let Some(paths) = std::env::var_os("PATH") {
        if let Some(p) = std::env::split_paths(&paths).map(|d| d.join(exe)).find(|p| p.is_file()) {
            return Some(p);
        }
    }
    if cfg!(windows) {
        // Installazioni standard: si sceglie la versione più recente.
        for base in [std::env::var_os("ProgramFiles"), std::env::var_os("ProgramFiles(x86)")]
            .into_iter()
            .flatten()
        {
            let root = PathBuf::from(base).join("Blender Foundation");
            if let Ok(rd) = std::fs::read_dir(&root) {
                let mut dirs: Vec<PathBuf> = rd
                    .filter_map(|e| e.ok().map(|e| e.path()))
                    .filter(|p| p.join(exe).is_file())
                    .collect();
                dirs.sort();
                if let Some(d) = dirs.pop() {
                    return Some(d.join(exe));
                }
            }
        }
    }
    None
}

pub fn blend_to_glb(blend: &Path) -> Result<BlendExport, ImportError> {
    let blender = blender_path().ok_or_else(|| {
        ImportError::Converter("Blender non trovato: installalo, oppure esporta da Blender in glTF (.glb) e importa quel file".into())
    })?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp = std::env::temp_dir();
    let script = tmp.join(format!("forgerender_blender_{stamp}.py"));
    let glb = tmp.join(format!("forgerender_blender_{stamp}.glb"));
    let json = tmp.join(format!("forgerender_blender_{stamp}.json"));
    std::fs::write(&script, SCRIPT)?;
    let mut cmd = Command::new(&blender);
    cmd.arg("-b")
        .arg(blend)
        .arg("--python")
        .arg(&script)
        .arg("--")
        .arg(&glb)
        .arg(&json)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let out = cmd.output();
    let _ = std::fs::remove_file(&script);
    let out = out.map_err(|e| ImportError::Converter(format!("avvio di Blender fallito: {e}")))?;
    if !out.status.success() || !glb.is_file() {
        let msg = String::from_utf8_lossy(&out.stderr);
        let last: Vec<&str> = msg.lines().rev().take(6).collect();
        return Err(ImportError::Converter(format!(
            "esportazione da Blender fallita: {}",
            last.into_iter().rev().collect::<Vec<_>>().join(" | ")
        )));
    }
    let area_lights = std::fs::read_to_string(&json)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let _ = std::fs::remove_file(&json);
    Ok(BlendExport { glb, area_lights })
}
