//! Ricerca locale nel manuale: titoli, id, parole chiave, sinonimi, controlli e testo.
//! Tollerante: minuscole, accenti ignorati, prefissi ("rugos" → rugosità) e un errore di
//! battitura nelle parole lunghe. Pesi: titolo > id e parole chiave > controlli > testo.

use crate::{parse_blocks, Page};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct Hit {
    pub id: String,
    pub score: f32,
}

pub(crate) struct SearchIndex {
    /// Per pagina: parola normalizzata → peso.
    docs: Vec<(String, HashMap<String, f32>)>,
}

/// Minuscole senza accenti, per confrontare "Opacità" con "opacita".
pub fn normalize(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ä' => 'a',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ò' | 'ó' | 'ô' | 'ö' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            c => c,
        })
        .collect()
}

fn words(s: &str) -> impl Iterator<Item = String> + '_ {
    s.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).map(normalize)
}

const TITLE: f32 = 10.0;
const ID: f32 = 8.0;
const KEYWORD: f32 = 6.0;
const UI: f32 = 4.0;
const TEXT: f32 = 1.0;

impl SearchIndex {
    pub(crate) fn build<'a>(pages: impl Iterator<Item = &'a Page>, synonyms: &[Vec<String>]) -> Self {
        let mut docs = Vec::new();
        for p in pages {
            let mut w: HashMap<String, f32> = HashMap::new();
            let mut add = |word: String, weight: f32| {
                let e = w.entry(word).or_default();
                *e = e.max(weight).max(*e + weight * 0.1);
            };
            for x in words(&p.title) {
                add(x, TITLE);
            }
            for x in words(&p.id) {
                add(x, ID);
            }
            for k in &p.keywords {
                for x in words(k) {
                    add(x, KEYWORD);
                }
            }
            for u in &p.ui {
                for x in words(u) {
                    add(x, UI);
                }
            }
            for b in parse_blocks(&p.body) {
                for x in words(&b.plain_text()) {
                    if x.len() > 2 {
                        add(x, TEXT);
                    }
                }
            }
            // Sinonimi: una parola della pagina porta con sé quelle del suo gruppo (peso ridotto).
            let extra: Vec<(String, f32)> = synonyms
                .iter()
                .flat_map(|g| {
                    let best = g.iter().filter_map(|x| w.get(x)).fold(0.0f32, |a, b| a.max(*b));
                    g.iter().map(move |x| (x.clone(), best * 0.8))
                })
                .filter(|(_, s)| *s > 0.0)
                .collect();
            for (x, s) in extra {
                let e = w.entry(x).or_default();
                *e = e.max(s);
            }
            docs.push((p.id.clone(), w));
        }
        docs.sort_by(|a, b| a.0.cmp(&b.0));
        Self { docs }
    }

    pub(crate) fn query(&self, q: &str, synonyms: &[Vec<String>]) -> Vec<Hit> {
        let terms: Vec<String> = words(q).collect();
        if terms.is_empty() {
            return Vec::new();
        }
        let mut hits: Vec<Hit> = self
            .docs
            .iter()
            .filter_map(|(id, w)| {
                let mut total = 0.0;
                for t in &terms {
                    // Il termine o un suo sinonimo.
                    let mut alts = vec![t.clone()];
                    for g in synonyms.iter().filter(|g| g.contains(t)) {
                        alts.extend(g.iter().cloned());
                    }
                    let best = alts.iter().map(|a| term_score(a, w)).fold(0.0f32, f32::max);
                    if best == 0.0 {
                        return None; // tutti i termini devono comparire
                    }
                    total += best;
                }
                Some(Hit {
                    id: id.clone(),
                    score: total,
                })
            })
            .collect();
        hits.sort_by(|a, b| b.score.total_cmp(&a.score).then_with(|| a.id.cmp(&b.id)));
        hits
    }
}

fn term_score(t: &str, w: &HashMap<String, f32>) -> f32 {
    if let Some(s) = w.get(t) {
        return *s;
    }
    let n = t.chars().count();
    let mut best = 0.0f32;
    for (word, s) in w {
        let score = if n >= 3 && word.starts_with(t) {
            s * 0.8
        } else if n >= 5 && word.chars().count().abs_diff(n) <= 1 && edit_distance_le1(t, word) {
            s * 0.6
        } else {
            0.0
        };
        best = best.max(score);
    }
    best
}

/// Vero se `a` e `b` differiscono al massimo per una lettera (aggiunta, tolta o cambiata).
fn edit_distance_le1(a: &str, b: &str) -> bool {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let (s, l) = if a.len() <= b.len() { (&a, &b) } else { (&b, &a) };
    if l.len() - s.len() > 1 {
        return false;
    }
    let (mut i, mut j, mut diff) = (0, 0, 0);
    while i < s.len() && j < l.len() {
        if s[i] == l[j] {
            i += 1;
            j += 1;
            continue;
        }
        diff += 1;
        if diff > 1 {
            return false;
        }
        if s.len() == l.len() {
            i += 1;
        }
        j += 1;
    }
    diff + (l.len() - j) + (s.len() - i) <= 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(id: &str, title: &str, kw: &[&str], body: &str) -> Page {
        Page {
            id: id.into(),
            title: title.into(),
            keywords: kw.iter().map(|s| s.to_string()).collect(),
            body: body.into(),
            ..Default::default()
        }
    }

    #[test]
    fn finds_by_title_keyword_prefix_typo_and_synonym() {
        let pages = [
            page(
                "mat.ior",
                "IOR (indice di rifrazione)",
                &["vetro"],
                "Quanto la luce si piega entrando nel materiale.",
            ),
            page(
                "mat.roughness",
                "Roughness (rugosità)",
                &["opaco", "lucido"],
                "Superficie liscia o ruvida.",
            ),
            page("tool.move", "Sposta", &["trasla"], "Strumento di spostamento con il gizmo."),
        ];
        let syn = vec![vec!["muovi".to_string(), "sposta".to_string()]];
        let idx = SearchIndex::build(pages.iter(), &syn);
        let first = |q: &str| idx.query(q, &syn).first().map(|h| h.id.clone());
        assert_eq!(first("IOR").as_deref(), Some("mat.ior"));
        assert_eq!(first("rugosita").as_deref(), Some("mat.roughness"), "accenti ignorati");
        assert_eq!(first("rugos").as_deref(), Some("mat.roughness"), "prefisso");
        assert_eq!(first("rougness").as_deref(), Some("mat.roughness"), "errore di battitura");
        assert_eq!(first("muovi").as_deref(), Some("tool.move"), "sinonimo");
        assert!(idx.query("sposta vetro", &syn).is_empty(), "tutti i termini devono esserci");
    }

    #[test]
    fn one_letter_distance() {
        assert!(edit_distance_le1("roughness", "rougness"));
        assert!(edit_distance_le1("denoiser", "denoizer"));
        assert!(!edit_distance_le1("denoiser", "dnoizer"));
    }
}
