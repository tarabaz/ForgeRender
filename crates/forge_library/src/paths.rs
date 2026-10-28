//! Cartelle della libreria.
//!
//! Ordine di scelta della radice:
//! 1. variabile d'ambiente `FORGE_LIBRARY_DIR`;
//! 2. versione portable (`portable.txt` accanto all'eseguibile): `<exe>/Libreria`;
//! 3. versione installata: `Documenti/ForgeRender/Libreria` (la cartella dei
//!    programmi non è scrivibile).

use std::path::{Path, PathBuf};

pub const HDRI: &str = "HDRI";
pub const MATERIALS: &str = "Materiali";
pub const TEXTURES: &str = "Texture";

fn documents() -> PathBuf {
    if let Some(p) = std::env::var_os("USERPROFILE") {
        return PathBuf::from(p).join("Documents");
    }
    if let Some(p) = std::env::var_os("XDG_DOCUMENTS_DIR") {
        return PathBuf::from(p);
    }
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    let docs = home.join("Documents");
    if docs.is_dir() {
        docs
    } else {
        home
    }
}

/// Radice della libreria.
pub fn root() -> PathBuf {
    if let Some(p) = std::env::var_os("FORGE_LIBRARY_DIR") {
        return PathBuf::from(p);
    }
    if let Some(exe_dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)) {
        if exe_dir.join("portable.txt").exists() {
            return exe_dir.join("Libreria");
        }
    }
    documents().join("ForgeRender").join("Libreria")
}

pub fn hdri_dir() -> PathBuf {
    root().join(HDRI)
}

pub fn materials_dir() -> PathBuf {
    root().join(MATERIALS)
}

pub fn textures_dir() -> PathBuf {
    root().join(TEXTURES)
}

/// Cache delle miniature dei siti online.
pub fn cache_dir() -> PathBuf {
    root().join(".cache")
}

/// Crea la struttura della libreria (se manca) con un breve LEGGIMI in ogni cartella.
pub fn ensure() -> std::io::Result<PathBuf> {
    let root = root();
    for (dir, text) in [
        (
            HDRI,
            "Metti qui le mappe HDRI (.hdr, .exr): compaiono nella scheda Ambiente di ForgeRender.\n\
             Le HDRI scaricate da Poly Haven finiscono qui.\n",
        ),
        (
            MATERIALS,
            "Materiali della libreria: una cartella per materiale con le mappe PBR e materiale.json.\n\
             Per aggiungerne uno: Libreria materiali → Importa da cartella o zip, oppure scaricalo da\n\
             ambientCG / Poly Haven. Non modificare a mano i file materiale.json.\n",
        ),
        (
            TEXTURES,
            "Texture sciolte da usare negli slot dei materiali (colore, rugosità, normal, …).\n",
        ),
    ] {
        let d = root.join(dir);
        std::fs::create_dir_all(&d)?;
        let readme = d.join("LEGGIMI.txt");
        if !readme.exists() {
            std::fs::write(readme, text)?;
        }
    }
    Ok(root)
}

/// Mappe HDRI presenti nella libreria (anche nelle sottocartelle di primo livello), in ordine alfabetico.
pub fn list_hdri() -> Vec<PathBuf> {
    list_files(&hdri_dir(), &["hdr", "exr"], 1)
}

pub(crate) fn list_files(dir: &Path, exts: &[&str], depth: u32) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else { return out };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            if depth > 0 {
                out.extend(list_files(&p, exts, depth - 1));
            }
        } else if p
            .extension()
            .and_then(|x| x.to_str())
            .is_some_and(|x| exts.iter().any(|e| x.eq_ignore_ascii_case(e)))
        {
            out.push(p);
        }
    }
    out.sort();
    out
}
