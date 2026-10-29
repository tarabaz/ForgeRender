//! Lingue dell'interfaccia. I testi del codice sono in italiano; ogni lingua è un file
//! `lang/Lan_<nome>.json` con la tabella "testo italiano" → "traduzione":
//!
//! ```json
//! { "code": "en", "name": "English", "strings": { "Importa modello…": "Import model…" } }
//! ```
//!
//! `Lan_ita.json` e `Lan_eng.json` sono compresi nel programma. I file nella cartella `lang`
//! accanto al programma li sostituiscono (stesso `code`) o aggiungono lingue nuove
//! (per esempio `Lan_spa.json`). Un testo che manca nel file resta in italiano.
//!
//! Nei testi con valori (`{n}`, `{}`, `{:.1}`) la traduzione deve contenere gli stessi segnaposto,
//! anche in un altro ordine: `"Campioni: {spp}"` → `"Samples: {spp}"`.
//!
//! Il cambio di lingua è immediato: ogni scritta passa da [`tr`] o [`tf`] a ogni fotogramma.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

/// Lingua del codice sorgente: nessuna tabella.
pub const SOURCE_CODE: &str = "it";

const BUILTIN: [&str; 2] = [
    include_str!("../../../lang/Lan_ita.json"),
    include_str!("../../../lang/Lan_eng.json"),
];

#[derive(Deserialize)]
struct LangFile {
    code: String,
    name: String,
    #[serde(default)]
    strings: HashMap<String, String>,
}

/// Una lingua caricata. Le traduzioni vivono per tutto il programma (sono poche e piccole).
pub struct Language {
    pub code: String,
    pub name: String,
    /// File da cui viene (`None` = compresa nel programma).
    pub source: Option<PathBuf>,
    strings: HashMap<String, &'static str>,
}

impl Language {
    fn parse(text: &str, source: Option<PathBuf>) -> Result<Language, String> {
        let f: LangFile = serde_json::from_str(text).map_err(|e| e.to_string())?;
        let strings = f
            .strings
            .into_iter()
            .filter(|(_, v)| !v.is_empty())
            .map(|(k, v)| (k, &*Box::leak(v.into_boxed_str())))
            .collect();
        Ok(Language {
            code: f.code,
            name: f.name,
            source,
            strings,
        })
    }

    /// Testi tradotti.
    #[cfg(test)]
    pub fn count(&self) -> usize {
        self.strings.len()
    }
}

/// Cartella delle lingue esterne: `lang` accanto al programma.
pub fn external_dir() -> Option<PathBuf> {
    std::env::current_exe().ok()?.parent().map(|d| d.join("lang"))
}

/// Lingue disponibili: quelle comprese, poi i file `lang/Lan_*.json` accanto al programma.
/// Restituisce anche gli errori dei file non validi (da mostrare nelle Impostazioni).
pub fn available() -> (Vec<Arc<Language>>, Vec<String>) {
    let mut out: Vec<Arc<Language>> = Vec::new();
    let mut errors = Vec::new();
    let put = |l: Language, out: &mut Vec<Arc<Language>>| {
        if let Some(i) = out.iter().position(|o| o.code == l.code) {
            out[i] = Arc::new(l);
        } else {
            out.push(Arc::new(l));
        }
    };
    for text in BUILTIN {
        match Language::parse(text, None) {
            Ok(l) => put(l, &mut out),
            Err(e) => errors.push(format!("lingua compresa non valida: {e}")),
        }
    }
    if let Some(dir) = external_dir() {
        let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
            .map(|rd| rd.filter_map(|e| e.ok().map(|e| e.path())).collect())
            .unwrap_or_default();
        files.sort();
        for p in files {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            if !(name.starts_with("Lan_") && name.ends_with(".json")) {
                continue;
            }
            match std::fs::read_to_string(&p)
                .map_err(|e| e.to_string())
                .and_then(|t| Language::parse(&t, Some(p.clone())))
            {
                Ok(l) => put(l, &mut out),
                Err(e) => errors.push(format!("{}: {e}", p.display())),
            }
        }
    }
    (out, errors)
}

static CURRENT: RwLock<Option<Arc<Language>>> = RwLock::new(None);

/// Attiva una lingua già caricata (`None` = italiano, la lingua del codice).
pub fn set(lang: Option<Arc<Language>>) {
    if let Ok(mut c) = CURRENT.write() {
        *c = lang.filter(|l| l.code != SOURCE_CODE);
    }
}

/// Codice della lingua attiva.
pub fn current_code() -> String {
    CURRENT
        .read()
        .ok()
        .and_then(|c| c.as_ref().map(|l| l.code.clone()))
        .unwrap_or_else(|| SOURCE_CODE.into())
}

/// Segna un testo da tradurre dentro una `const` (la traduzione si fa poi con [`tr`]).
pub const fn tk(s: &'static str) -> &'static str {
    s
}

/// Traduce un testo dell'interfaccia (il testo italiano è la chiave).
pub fn tr(s: &'static str) -> &'static str {
    match CURRENT.read() {
        Ok(c) => match c.as_ref() {
            Some(l) => l.strings.get(s).copied().unwrap_or(s),
            None => s,
        },
        Err(_) => s,
    }
}

/// Traduce una stringa non statica (per esempio l'etichetta di un tipo che arriva da un'altra
/// libreria). Restituisce l'originale se non c'è traduzione.
pub fn tr_dyn(s: &str) -> String {
    if let Ok(c) = CURRENT.read() {
        if let Some(t) = c.as_ref().and_then(|l| l.strings.get(s)) {
            return t.to_string();
        }
    }
    s.to_string()
}

/// Segnaposto di un modello `format!`, nell'ordine: il testo tra le graffe (`""`, `"n"`, `":.1"`).
fn placeholders(template: &str) -> Vec<(usize, usize)> {
    let b = template.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'{' if b.get(i + 1) == Some(&b'{') => i += 2,
            b'}' if b.get(i + 1) == Some(&b'}') => i += 2,
            b'{' => {
                let start = i;
                while i < b.len() && b[i] != b'}' {
                    i += 1;
                }
                out.push((start, (i + 1).min(b.len())));
                i += 1;
            }
            _ => i += 1,
        }
    }
    out
}

/// Inserisce i valori nel modello: `values[k]` è il valore del k-esimo segnaposto del modello
/// italiano `source`. Nella traduzione i segnaposto si riconoscono dal testo tra le graffe
/// (a parità di testo, nell'ordine).
fn fill(source: &str, target: &str, values: &[String]) -> String {
    let src = placeholders(source);
    let mut used = vec![false; src.len()];
    let mut out = String::with_capacity(target.len() + 16);
    let mut last = 0;
    for (s, e) in placeholders(target) {
        out.push_str(&unescape(&target[last..s]));
        let tok = &target[s..e];
        let k = src.iter().enumerate().position(|(k, &(a, b))| !used[k] && &source[a..b] == tok);
        match k.and_then(|k| values.get(k).map(|v| (k, v))) {
            Some((k, v)) => {
                used[k] = true;
                out.push_str(v);
            }
            None => out.push_str(tok),
        }
        last = e;
    }
    out.push_str(&unescape(&target[last..]));
    out
}

fn unescape(s: &str) -> String {
    s.replace("{{", "{").replace("}}", "}")
}

/// Testo con valori: `source` è il modello italiano di `format!`, `values` i suoi segnaposto già
/// formattati, nell'ordine in cui compaiono nel modello.
pub fn tf(source: &'static str, values: &[String]) -> String {
    let target = tr(source);
    fill(source, target, values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_languages_parse() {
        let (langs, errors) = available();
        assert!(errors.is_empty(), "{errors:?}");
        let en = langs.iter().find(|l| l.code == "en").expect("inglese");
        assert_eq!(en.name, "English");
        assert!(en.count() > 300, "poche traduzioni: {}", en.count());
        assert!(langs.iter().any(|l| l.code == "it"));
    }

    #[test]
    fn fill_keeps_values_and_reorders() {
        let v = vec!["3".to_string(), "12".to_string()];
        assert_eq!(fill("Passo {} di {n}", "Passo {} di {n}", &v), "Passo 3 di 12");
        assert_eq!(fill("Passo {} di {n}", "{n} total, step {}", &v), "12 total, step 3");
        assert_eq!(fill("{:.1} m {{x}}", "{:.1} m {{x}}", &["2.5".into()]), "2.5 m {x}");
        // Segnaposto uguali: nell'ordine.
        assert_eq!(fill("{} × {}", "{} by {}", &["4".into(), "5".into()]), "4 by 5");
    }

    /// Ogni testo passato a `tr` o `tf` nel codice ha la traduzione inglese, e il file inglese
    /// non contiene testi che il codice non usa più. Le traduzioni mantengono i segnaposto.
    #[test]
    fn english_covers_every_interface_text() {
        let keys = source_keys();
        let en = BUILTIN[1];
        let f: LangFile = serde_json::from_str(en).unwrap();
        let missing: Vec<_> = keys.iter().filter(|k| !f.strings.contains_key(*k)).collect();
        assert!(missing.is_empty(), "{} testi senza traduzione inglese: {missing:#?}", missing.len());
        let stale: Vec<_> = f.strings.keys().filter(|k| !keys.contains(*k)).collect();
        assert!(stale.is_empty(), "testi tradotti non più usati: {stale:#?}");
        for (k, v) in &f.strings {
            let mut a: Vec<_> = placeholders(k).iter().map(|&(s, e)| &k[s..e]).collect();
            let mut b: Vec<_> = placeholders(v).iter().map(|&(s, e)| &v[s..e]).collect();
            a.sort();
            b.sort();
            assert_eq!(a, b, "segnaposto diversi in {k:?} → {v:?}");
        }
    }

    /// Testi letterali passati a `tr(` / `tf(` nei sorgenti dell'app.
    fn source_keys() -> std::collections::BTreeSet<String> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = vec![];
        let mut stack = vec![root];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "rs") && !p.ends_with("i18n.rs") {
                    files.push(p);
                }
            }
        }
        let mut keys = std::collections::BTreeSet::new();
        for f in files {
            let text = std::fs::read_to_string(&f).unwrap();
            for marker in ["tr(", "tf(", "tk("] {
                let mut rest = text.as_str();
                while let Some(i) = rest.find(marker) {
                    let prev = rest[..i].chars().last();
                    rest = &rest[i + marker.len()..];
                    let trimmed = rest.trim_start();
                    if prev.is_some_and(|c| c.is_alphanumeric() || c == '_') || !trimmed.starts_with('"') {
                        continue;
                    }
                    let (lit, after) = read_literal(&trimmed[1..]);
                    keys.insert(lit);
                    rest = after;
                }
            }
        }
        keys
    }

    /// Legge un letterale Rust (dopo le virgolette di apertura) e lo decodifica.
    fn read_literal(s: &str) -> (String, &str) {
        let mut out = String::new();
        let mut it = s.char_indices();
        while let Some((i, c)) = it.next() {
            match c {
                '"' => return (out, &s[i + 1..]),
                '\\' => match it.next().map(|x| x.1) {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('\'') => out.push('\''),
                    Some('\n') => {
                        // Continuazione di riga: salta gli spazi iniziali.
                        let rest = &s[i + 2..];
                        let skip = rest.len() - rest.trim_start().len();
                        for _ in 0..rest[..skip].chars().count() {
                            it.next();
                        }
                    }
                    Some('u') => {
                        let rest = &s[i + 2..];
                        let end = rest.find('}').unwrap();
                        let code = u32::from_str_radix(&rest[1..end], 16).unwrap();
                        out.push(char::from_u32(code).unwrap());
                        for _ in 0..=end {
                            it.next();
                        }
                    }
                    Some(o) => out.push(o),
                    None => {}
                },
                c => out.push(c),
            }
        }
        (out, "")
    }
}
