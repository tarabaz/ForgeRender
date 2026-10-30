//! I progetti di esempio (demo e benchmark in `samples/`) si devono aprire anche da un'altra
//! cartella, come dopo l'estrazione dello zip portable su un altro PC: modelli e texture si
//! ritrovano dai percorsi relativi, senza file mancanti.

use std::path::{Path, PathBuf};

fn samples_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples")
}

/// Copia la cartella senza le texture estratte dai .glb (`*_texture`, si ricreano all'apertura).
fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let p = e.path();
        let name = e.file_name();
        if p.is_dir() {
            if !name.to_string_lossy().ends_with("_texture") {
                copy_dir(&p, &to.join(&name));
            }
        } else {
            std::fs::copy(&p, to.join(&name)).unwrap();
        }
    }
}

/// Sostituisce i percorsi assoluti del PC su cui è stato salvato il progetto con una cartella
/// inesistente: il progetto deve ritrovare tutto dalla propria cartella.
fn break_absolute_paths(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::String(s) => {
            let norm = s.replace('\\', "/");
            if let Some(i) = norm.find("/samples/") {
                *s = format!("/cartella/inesistente{}", &norm[i..]);
            }
        }
        serde_json::Value::Array(a) => a.iter_mut().for_each(break_absolute_paths),
        serde_json::Value::Object(o) => o.values_mut().for_each(break_absolute_paths),
        _ => {}
    }
}

fn projects(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            projects(&p, out);
        } else if p.extension().is_some_and(|x| x == "frproject") {
            out.push(p);
        }
    }
}

#[test]
fn sample_projects_open_from_another_folder() {
    let tmp = std::env::temp_dir().join(format!("forge_samples_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    copy_dir(&samples_dir(), &tmp);

    let mut list = Vec::new();
    projects(&tmp, &mut list);
    list.sort();
    let names: Vec<String> = list.iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
    // I benchmark pronti fanno parte del repository (e quindi del pacchetto portable).
    for b in ["bench_ad.frproject", "bench_intera.frproject", "bench_vicina.frproject"] {
        assert!(names.iter().any(|n| n == b), "manca {b} in samples/: {names:?}");
    }

    for path in &list {
        let mut v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        break_absolute_paths(&mut v);
        std::fs::write(path, serde_json::to_string(&v).unwrap()).unwrap();

        let p = forge_project::load(path).unwrap_or_else(|e| panic!("{}: {e:#}", path.display()));
        let r = &p.report;
        assert!(
            r.missing_sources.is_empty(),
            "{}: modelli mancanti {:?}",
            path.display(),
            r.missing_sources
        );
        assert!(
            r.missing_components.is_empty(),
            "{}: componenti mancanti {:?}",
            path.display(),
            r.missing_components
        );
        assert!(r.warnings.is_empty(), "{}: {:?}", path.display(), r.warnings);
        // Ogni texture usata esiste e viene dalla copia, non dalla cartella originale.
        // Le mappe dei materiali predefiniti stanno nella cartella del programma.
        let root = std::fs::canonicalize(&tmp).unwrap();
        let program = forge_scene::material::builtin_textures_dir()
            .and_then(std::path::Path::parent)
            .and_then(|p| std::fs::canonicalize(p).ok());
        for m in &p.scene.materials.materials {
            for t in m.texture_slots().into_iter().flatten() {
                let real = std::fs::canonicalize(t).unwrap_or_else(|_| panic!("{}: texture mancante {}", path.display(), t.display()));
                assert!(
                    real.starts_with(&root) || program.as_ref().is_some_and(|p| real.starts_with(p)),
                    "{}: texture fuori dalla copia {}",
                    path.display(),
                    real.display()
                );
            }
        }
    }
    std::fs::remove_dir_all(&tmp).ok();
}
