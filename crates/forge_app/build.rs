//! Identificativo della build per il titolo della finestra: short SHA del commit Git
//! usato per compilare, calcolato una volta in compilazione (mai durante l'esecuzione).
//!
//! Su Windows (MSVC) incorpora anche l'icona `assets/forgerender.ico` nelle risorse di
//! `forgerender.exe` (Explorer, collegamenti, installer): il file `.res` si scrive qui, senza
//! `rc.exe` né dipendenze, e lo legge il linker.
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

/// Risorse Windows (.res) con l'icona: ogni immagine del .ico come RT_ICON (3) e il gruppo
/// RT_GROUP_ICON (14) numero 1, quello che Explorer mostra per l'eseguibile.
fn icon_res(ico: &[u8]) -> Option<Vec<u8>> {
    let u16_at = |o: usize| Some(u16::from_le_bytes(ico.get(o..o + 2)?.try_into().ok()?));
    let u32_at = |o: usize| Some(u32::from_le_bytes(ico.get(o..o + 4)?.try_into().ok()?));
    if u16_at(0)? != 0 || u16_at(2)? != 1 {
        return None;
    }
    let count = u16_at(4)? as usize;
    fn resource(out: &mut Vec<u8>, kind: u16, id: u16, flags: u16, data: &[u8]) {
        out.extend((data.len() as u32).to_le_bytes());
        out.extend(32u32.to_le_bytes());
        out.extend([0xFF, 0xFF]);
        out.extend(kind.to_le_bytes());
        out.extend([0xFF, 0xFF]);
        out.extend(id.to_le_bytes());
        out.extend(0u32.to_le_bytes());
        out.extend(flags.to_le_bytes());
        out.extend(0x0409u16.to_le_bytes());
        out.extend(0u32.to_le_bytes());
        out.extend(0u32.to_le_bytes());
        out.extend(data);
        while !out.len().is_multiple_of(4) {
            out.push(0);
        }
    }
    // Primo record vuoto: è così che inizia ogni file .res a 32 bit.
    let mut out = vec![0, 0, 0, 0, 32, 0, 0, 0, 0xFF, 0xFF, 0, 0, 0xFF, 0xFF, 0, 0];
    out.extend([0u8; 16]);
    let mut group = Vec::new();
    group.extend(0u16.to_le_bytes());
    group.extend(1u16.to_le_bytes());
    group.extend((count as u16).to_le_bytes());
    for i in 0..count {
        let e = 6 + 16 * i;
        let size = u32_at(e + 8)? as usize;
        let offset = u32_at(e + 12)? as usize;
        let data = ico.get(offset..offset + size)?;
        let id = (i + 1) as u16;
        resource(&mut out, 3, id, 0x1010, data);
        group.extend(ico.get(e..e + 8)?);
        group.extend((size as u32).to_le_bytes());
        group.extend(id.to_le_bytes());
    }
    resource(&mut out, 14, 1, 0x1030, &group);
    Some(out)
}

fn main() {
    println!("cargo:rerun-if-env-changed=FORGE_GIT_SHA");
    let ico_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/forgerender.ico");
    println!("cargo:rerun-if-changed={}", ico_path.display());
    let windows_msvc =
        std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    if windows_msvc {
        let res = std::fs::read(&ico_path).ok().and_then(|ico| icon_res(&ico));
        match res {
            Some(res) => {
                let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR")).join("forgerender-icona.res");
                std::fs::write(&out, res).expect("scrittura delle risorse dell'icona");
                println!("cargo:rustc-link-arg-bin=forgerender={}", out.display());
            }
            None => println!("cargo:warning=icona non incorporata: {} mancante o non valida", ico_path.display()),
        }
    }
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
