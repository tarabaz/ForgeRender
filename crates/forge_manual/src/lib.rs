//! Manuale interno di ForgeRender: contenuti separati dal codice, consultabili offline.
//!
//! Struttura su disco (cartella `manuale/` accanto al programma):
//!
//! ```text
//! manuale/
//!   indice.json            capitoli (titolo per lingua) e id delle pagine, uguali per tutte le lingue
//!   sinonimi.<lingua>.txt  "parola = sinonimo, sinonimo" per la ricerca
//!   <lingua>/<capitolo>/<id>.md   una pagina per funzione
//!   img/comune/…  img/<lingua>/…  immagini (render comuni, screenshot per lingua)
//! ```
//!
//! Ogni pagina inizia con un'intestazione tra due righe `---`:
//!
//! ```text
//! ---
//! id: mat.roughness
//! titolo: Roughness (rugosità)
//! parole: rugosità, lucido, opaco
//! ui: panel.materials.roughness
//! correlate: mat.metallic, mat.ior
//! ---
//! ```
//!
//! Le immagini si scrivono `![didascalia](img/comune/roughness.jpg)` (percorso dalla cartella
//! `manuale/`), i collegamenti interni `[IOR](mat.ior)`.

mod markdown;
mod search;

pub use markdown::{parse_blocks, Block, Span};
pub use search::{normalize, Hit};

use anyhow::{bail, Context};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize)]
pub struct ChapterDef {
    pub id: String,
    /// Titolo per codice lingua ("it", "en").
    pub titolo: BTreeMap<String, String>,
    pub pagine: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Index {
    pub capitoli: Vec<ChapterDef>,
}

#[derive(Clone, Debug, Default)]
pub struct Page {
    pub id: String,
    pub title: String,
    pub keywords: Vec<String>,
    /// Id dei controlli dell'interfaccia descritti (aiuto contestuale, F1).
    pub ui: Vec<String>,
    pub related: Vec<String>,
    /// Capitolo (id) in cui la pagina è elencata.
    pub chapter: String,
    pub body: String,
    pub file: PathBuf,
}

/// Il manuale in una lingua.
pub struct Manual {
    pub root: PathBuf,
    pub lang: String,
    pub index: Index,
    pub pages: HashMap<String, Page>,
    synonyms: Vec<Vec<String>>,
    search: search::SearchIndex,
}

impl Manual {
    /// Carica tutte le pagine di `lang`. Le pagine elencate nell'indice ma mancanti non
    /// impediscono l'apertura: [`Manual::problems`] le elenca.
    pub fn load(root: &Path, lang: &str) -> anyhow::Result<Self> {
        let index = load_index(root)?;
        let mut pages = HashMap::new();
        for ch in &index.capitoli {
            for id in &ch.pagine {
                let file = page_path(root, lang, &ch.id, id);
                if let Ok(text) = std::fs::read_to_string(&file) {
                    let mut p = parse_page(&text).with_context(|| format!("{}", file.display()))?;
                    p.chapter = ch.id.clone();
                    p.file = file;
                    pages.insert(id.clone(), p);
                }
            }
        }
        let synonyms = load_synonyms(&root.join(format!("sinonimi.{lang}.txt")));
        let search = search::SearchIndex::build(pages.values(), &synonyms);
        Ok(Self {
            root: root.to_path_buf(),
            lang: lang.to_string(),
            index,
            pages,
            synonyms,
            search,
        })
    }

    pub fn chapter_title(&self, ch: &ChapterDef) -> String {
        ch.titolo
            .get(&self.lang)
            .or_else(|| ch.titolo.values().next())
            .cloned()
            .unwrap_or_else(|| ch.id.clone())
    }

    /// Pagine trovate per la ricerca, dalla più pertinente.
    pub fn search(&self, query: &str) -> Vec<Hit> {
        self.search.query(query, &self.synonyms)
    }

    /// Pagina che descrive il controllo `ui_id` (o il cui id è `ui_id`).
    pub fn page_for_ui(&self, ui_id: &str) -> Option<&Page> {
        self.pages
            .get(ui_id)
            .or_else(|| self.pages.values().find(|p| p.ui.iter().any(|u| u == ui_id)))
    }

    /// Percorso di un'immagine scritta nella pagina (relativo a `manuale/`).
    pub fn image_path(&self, src: &str) -> PathBuf {
        self.root.join(src)
    }

    /// Problemi di questa lingua: pagine mancanti, collegamenti e immagini rotti, intestazioni
    /// incomplete. Vuoto = tutto a posto.
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        for ch in &self.index.capitoli {
            if !ch.titolo.contains_key(&self.lang) {
                out.push(format!("capitolo {} senza titolo in \"{}\"", ch.id, self.lang));
            }
            for id in &ch.pagine {
                if !self.pages.contains_key(id) {
                    out.push(format!(
                        "[{}] pagina mancante: {}",
                        self.lang,
                        page_path(&self.root, &self.lang, &ch.id, id).display()
                    ));
                }
            }
        }
        for p in self.pages.values() {
            let name = p.file.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if p.id != name {
                out.push(format!(
                    "[{}] {}: l'id \"{}\" non corrisponde al nome del file",
                    self.lang,
                    p.file.display(),
                    p.id
                ));
            }
            if p.title.trim().is_empty() {
                out.push(format!("[{}] {}: titolo vuoto", self.lang, p.id));
            }
            if p.body.trim().len() < 40 {
                out.push(format!("[{}] {}: pagina senza contenuto", self.lang, p.id));
            }
            for r in &p.related {
                if !self.pages.contains_key(r) {
                    out.push(format!("[{}] {}: \"correlate\" punta a {r}, che non esiste", self.lang, p.id));
                }
            }
            for b in parse_blocks(&p.body) {
                b.visit_links(&mut |target| {
                    if is_internal(target) && !self.pages.contains_key(target) {
                        out.push(format!("[{}] {}: collegamento a {target}, che non esiste", self.lang, p.id));
                    }
                });
                if let Block::Image { src, .. } = &b {
                    if !self.image_path(src).is_file() {
                        out.push(format!("[{}] {}: immagine mancante {src}", self.lang, p.id));
                    }
                }
            }
        }
        out.sort();
        out
    }
}

/// Collegamento a un'altra pagina (non a un sito o a un file).
pub fn is_internal(target: &str) -> bool {
    !target.contains("://") && !target.contains('/') && !target.starts_with('#')
}

pub fn load_index(root: &Path) -> anyhow::Result<Index> {
    let path = root.join("indice.json");
    let text = std::fs::read_to_string(&path).with_context(|| format!("{} non trovato", path.display()))?;
    let index: Index = serde_json::from_str(&text).with_context(|| format!("{} non valido", path.display()))?;
    Ok(index)
}

/// Lingue presenti: le cartelle accanto a `indice.json` (escluse le immagini).
pub fn languages(root: &Path) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(root)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .filter_map(|e| e.file_name().to_str().map(str::to_string))
                .filter(|n| n.len() == 2)
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

pub fn page_path(root: &Path, lang: &str, chapter: &str, id: &str) -> PathBuf {
    root.join(lang).join(chapter).join(format!("{id}.md"))
}

/// Legge intestazione e corpo di una pagina.
pub fn parse_page(text: &str) -> anyhow::Result<Page> {
    let text = text.trim_start_matches('\u{feff}');
    let Some(rest) = text.strip_prefix("---") else {
        bail!("manca l'intestazione (--- id: … ---)");
    };
    let Some(end) = rest.find("\n---") else {
        bail!("intestazione non chiusa da ---");
    };
    let mut p = Page::default();
    let list = |v: &str| {
        v.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
    };
    for line in rest[..end].lines() {
        let Some((k, v)) = line.split_once(':') else { continue };
        let v = v.trim();
        match k.trim() {
            "id" => p.id = v.to_string(),
            "titolo" | "title" => p.title = v.to_string(),
            "parole" | "keywords" => p.keywords = list(v),
            "ui" => p.ui = list(v),
            "correlate" | "related" => p.related = list(v),
            _ => {}
        }
    }
    if p.id.is_empty() {
        bail!("intestazione senza id");
    }
    p.body = rest[end + 4..].trim_start_matches(['-', '\r', '\n']).to_string();
    Ok(p)
}

fn load_synonyms(path: &Path) -> Vec<Vec<String>> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .map(|l| {
            l.split(['=', ','])
                .map(|w| normalize(w.trim()))
                .filter(|w| !w.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|g| g.len() > 1)
        .collect()
}

/// Controllo di tutto il manuale: ogni lingua completa e coerente, stessi id ovunque.
pub fn check_all(root: &Path) -> anyhow::Result<Vec<String>> {
    let langs = languages(root);
    if langs.is_empty() {
        bail!("nessuna lingua in {}", root.display());
    }
    let mut out = Vec::new();
    let mut ids_seen: HashMap<String, usize> = HashMap::new();
    let index = load_index(root)?;
    for ch in &index.capitoli {
        for id in &ch.pagine {
            *ids_seen.entry(id.clone()).or_default() += 1;
        }
    }
    for (id, n) in ids_seen {
        if n > 1 {
            out.push(format!("id ripetuto nell'indice: {id} ({n} volte)"));
        }
    }
    for lang in &langs {
        let m = Manual::load(root, lang)?;
        out.extend(m.problems());
        // Pagine presenti su disco ma non nell'indice: non si raggiungerebbero mai.
        for ch in &index.capitoli {
            let dir = root.join(lang).join(&ch.id);
            for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                let stem = e.path().file_stem().and_then(|s| s.to_str()).unwrap_or("").to_string();
                if e.path().extension().is_some_and(|x| x == "md") && !ch.pagine.contains(&stem) {
                    out.push(format!("[{lang}] {} non è nell'indice", e.path().display()));
                }
            }
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_header_and_body() {
        let p = parse_page("---\nid: mat.ior\ntitolo: IOR\nparole: rifrazione, vetro\nui: panel.materials.ior\ncorrelate: mat.transmission\n---\n\n# IOR\n\nTesto.").unwrap();
        assert_eq!(p.id, "mat.ior");
        assert_eq!(p.keywords, ["rifrazione", "vetro"]);
        assert_eq!(p.related, ["mat.transmission"]);
        assert!(p.body.starts_with("# IOR"));
        assert!(parse_page("# senza intestazione").is_err());
    }

    #[test]
    fn internal_links() {
        assert!(is_internal("mat.ior"));
        assert!(!is_internal("https://polyhaven.com"));
        assert!(!is_internal("img/comune/a.jpg"));
    }
}

#[cfg(test)]
mod repo_tests {
    /// Il manuale del repository è completo e coerente in tutte le lingue.
    #[test]
    fn repository_manual_is_consistent() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../manuale");
        let problems = super::check_all(&root).unwrap();
        assert!(problems.is_empty(), "problemi nel manuale:\n{}", problems.join("\n"));
        let langs = super::languages(&root);
        assert!(langs.contains(&"it".to_string()) && langs.contains(&"en".to_string()), "{langs:?}");
        // I casi di ricerca del piano trovano la pagina giusta al primo posto.
        for (lang, q, want) in [
            ("it", "IOR", "mat.ior"),
            ("it", "rugosità", "mat.roughness"),
            ("it", "grana", "render.denoiser"),
            ("it", "denoiser", "render.denoiser"),
            ("it", "tastiera", "keys.elenco"),
            ("en", "matte", "mat.roughness"),
            ("en", "refraction", "mat.ior"),
            ("en", "shortcuts", "keys.elenco"),
        ] {
            let m = super::Manual::load(&root, lang).unwrap();
            let first = m.search(q).first().map(|h| h.id.clone());
            assert_eq!(first.as_deref(), Some(want), "{lang}: \"{q}\"");
        }
    }
}
