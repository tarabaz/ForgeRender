//! Identificativo della build per il titolo della finestra: short SHA del commit Git
//! usato per compilare, calcolato una volta in compilazione (mai durante l'esecuzione).
//!
//! Ordine: variabile d'ambiente `FORGE_GIT_SHA` (es. impostata da uno script di build),
//! poi `git rev-parse --short=7 HEAD`, infine `dev` (sorgenti senza Git, Git non installato).

use std::path::PathBuf;
use std::process::Command;

fn git(args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (!s.is_empty()).then_some(s)
}

fn main() {
    println!("cargo:rerun-if-env-changed=FORGE_GIT_SHA");
    let sha = std::env::var("FORGE_GIT_SHA")
        .ok()
        .map(|s| s.trim().chars().take(12).collect::<String>())
        .filter(|s| !s.is_empty())
        .or_else(|| git(&["rev-parse", "--short=7", "HEAD"]))
        .unwrap_or_else(|| "dev".to_string());
    println!("cargo:rustc-env=FORGE_GIT_SHA={sha}");

    // Ricalcolo solo quando cambia il commit: HEAD, il ref a cui punta e i ref compattati.
    if let Some(dir) = git(&["rev-parse", "--git-dir"]) {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(dir);
        println!("cargo:rerun-if-changed={}", dir.join("HEAD").display());
        println!("cargo:rerun-if-changed={}", dir.join("packed-refs").display());
        if let Some(r) = git(&["symbolic-ref", "-q", "HEAD"]) {
            println!("cargo:rerun-if-changed={}", dir.join(r).display());
        }
    }
}
