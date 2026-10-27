//! Test end-to-end: assieme STEP generato con Open CASCADE → conversione → scena.

use forge_import::frmesh::FrMesh;
use forge_import::{step, ImportOptions};
use forge_scene::{ComponentId, Scene};
use std::process::Command;

const EXE: &str = env!("CARGO_BIN_EXE_forgerender-step");

#[test]
fn step_assembly_roundtrip() {
    let dir = std::env::temp_dir().join(format!("forge_step_e2e_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let step_path = dir.join("staffa.step");
    let out = dir.join("staffa.frmesh");

    let st = Command::new(EXE).arg("--write-test-assembly").arg(&step_path).output().unwrap();
    assert!(st.status.success(), "{}", String::from_utf8_lossy(&st.stderr));
    let st = Command::new(EXE).arg(&step_path).arg(&out).output().unwrap();
    assert!(st.status.success(), "{}", String::from_utf8_lossy(&st.stderr));

    let model = FrMesh::read(&out).unwrap();
    assert!((model.header.unit_m - 0.001).abs() < 1e-12, "unità {}", model.header.unit_m);
    // Due istanze della stessa vite: una sola mesh per la vite + una per la piastra.
    assert_eq!(model.meshes.len(), 2);
    let names: Vec<&str> = model.header.nodes.iter().map(|n| n.name.as_str()).collect();
    for n in ["Staffa", "Piastra-1", "Vite M8-1", "Vite M8-2"] {
        assert!(names.contains(&n), "manca {n} in {names:?}");
    }
    assert!(model.header.nodes.iter().any(|n| n.color.is_some()), "colori non letti");

    let mut scene = Scene::default();
    let report = step::insert(&mut scene, "staffa.step", model, &ImportOptions::default()).unwrap();
    assert_eq!(report.components, 3);
    let plate = scene.find(&ComponentId("staffa.step/Staffa/Piastra-1".into())).unwrap();
    let e = scene.bounds_of(plate).extent();
    // Piastra 100 × 10 × 60 mm in metri (Y verso l'alto: nessuna rotazione).
    assert!(
        (e.x - 0.1).abs() < 1e-4 && (e.y - 0.01).abs() < 1e-4 && (e.z - 0.06).abs() < 1e-4,
        "{e}"
    );
    // La seconda vite è ruotata di 90° attorno a X: il suo asse diventa verticale.
    let v2 = scene.find(&ComponentId("staffa.step/Staffa/Vite M8-2".into())).unwrap();
    let ev = scene.bounds_of(v2).extent();
    assert!((ev.y - 0.04).abs() < 1e-3 && ev.x < 0.0085, "{ev}");
    // Il colore rosso del pezzo diventa un materiale assegnato.
    let body = scene.nodes[v2].children[0];
    let m = scene.materials.get(scene.resolve_material(body)).unwrap();
    assert!(m.base_color[0] > 0.5 && m.base_color[2] < 0.2, "{:?}", m.base_color);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn invalid_file_fails_cleanly() {
    let dir = std::env::temp_dir().join(format!("forge_step_bad_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let bad = dir.join("rotto.step");
    std::fs::write(&bad, "questo non è uno STEP").unwrap();
    let st = Command::new(EXE).arg(&bad).arg(dir.join("x.frmesh")).output().unwrap();
    assert!(!st.status.success());
    assert!(!String::from_utf8_lossy(&st.stderr).is_empty());
    std::fs::remove_dir_all(dir).ok();
}
