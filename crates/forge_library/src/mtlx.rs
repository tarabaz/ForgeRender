//! Lettura dei file MaterialX (`.mtlx`) che ambientCG e Poly Haven distribuiscono con le mappe.
//!
//! Si leggono solo i valori scritti nel file (input con `value="…"`, non collegati a una
//! texture o a un altro nodo): niente viene dedotto o inventato. I valori si mostrano nel
//! pannello del materiale e si applicano su richiesta.
//!
//! Lettore minimo (tag, attributi tra virgolette, annidamento) senza dipendenze: basta per la
//! struttura dei file MaterialX (`<nodo name=…><input name=… type=… value=…/></nodo>`).
//! **Non ancora verificato su file reali dei due siti** (non raggiungibili dal container di
//! sviluppo): la struttura segue la specifica MaterialX.

use std::path::{Path, PathBuf};

/// Un valore scritto nel file: `categoria.input = valore` (es. `displacement.scale = 0.02`).
#[derive(Clone, Debug, PartialEq)]
pub struct MtlxValue {
    /// Tipo del nodo (`standard_surface`, `displacement`, `normalmap`…).
    pub node: String,
    /// Nome dell'istanza del nodo nel file.
    pub node_name: String,
    pub input: String,
    pub ty: String,
    pub value: String,
}

impl MtlxValue {
    pub fn as_f32(&self) -> Option<f32> {
        self.value.trim().parse().ok().filter(|v: &f32| v.is_finite())
    }
}

/// Valori scritti nel file, nell'ordine in cui compaiono.
pub fn values(xml: &str) -> Vec<MtlxValue> {
    let mut out = Vec::new();
    // Pila degli elementi aperti: (tipo, nome).
    let mut stack: Vec<(String, String)> = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find('<') {
        rest = &rest[start + 1..];
        // Commenti, dichiarazioni e istruzioni.
        if let Some(r) = rest.strip_prefix("!--") {
            rest = r.find("-->").map(|e| &r[e + 3..]).unwrap_or("");
            continue;
        }
        if rest.starts_with('?') || rest.starts_with('!') {
            rest = rest.find('>').map(|e| &rest[e + 1..]).unwrap_or("");
            continue;
        }
        let Some(end) = tag_end(rest) else { break };
        let tag = &rest[..end];
        rest = &rest[end + 1..];
        if let Some(closing) = tag.strip_prefix('/') {
            let name = closing.trim();
            if let Some(pos) = stack.iter().rposition(|(t, _)| t == name) {
                stack.truncate(pos);
            }
            continue;
        }
        let self_closing = tag.trim_end().ends_with('/');
        let body = tag.trim_end().trim_end_matches('/');
        let (kind, attrs) = match body.find(char::is_whitespace) {
            Some(i) => (&body[..i], parse_attrs(&body[i..])),
            None => (body, Vec::new()),
        };
        let attr = |k: &str| attrs.iter().find(|(a, _)| a == k).map(|(_, v)| v.clone());
        if kind == "input" {
            let connected = attr("nodename").is_some() || attr("output").is_some() || attr("interfacename").is_some();
            if let (Some(name), Some(value), false) = (attr("name"), attr("value"), connected) {
                if let Some((node, node_name)) = stack.last() {
                    out.push(MtlxValue {
                        node: node.clone(),
                        node_name: node_name.clone(),
                        input: name,
                        ty: attr("type").unwrap_or_default(),
                        value,
                    });
                }
            }
        }
        if !self_closing {
            stack.push((kind.to_string(), attr("name").unwrap_or_default()));
        }
    }
    out
}

/// Fine del tag (`>` fuori dalle virgolette).
fn tag_end(s: &str) -> Option<usize> {
    let mut quote: Option<char> = None;
    for (i, c) in s.char_indices() {
        match (quote, c) {
            (None, '"' | '\'') => quote = Some(c),
            (Some(q), _) if c == q => quote = None,
            (None, '>') => return Some(i),
            _ => {}
        }
    }
    None
}

fn parse_attrs(s: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = s;
    while let Some(eq) = rest.find('=') {
        let key = rest[..eq].trim().to_string();
        let after = rest[eq + 1..].trim_start();
        let Some(q) = after.chars().next().filter(|c| *c == '"' || *c == '\'') else {
            break;
        };
        let Some(close) = after[1..].find(q) else { break };
        out.push((key, unescape(&after[1..1 + close])));
        rest = &after[close + 2..];
    }
    out
}

fn unescape(s: &str) -> String {
    s.replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// Scala del displacement scritta nel file (nodo `displacement`, input `scale`), in unità
/// della scena come da specifica MaterialX.
pub fn displacement_scale(values: &[MtlxValue]) -> Option<f32> {
    values
        .iter()
        .find(|v| v.node == "displacement" && v.input == "scale")
        .and_then(MtlxValue::as_f32)
}

/// File `.mtlx` nella cartella di un materiale della libreria.
pub fn find_in(dir: &Path) -> Option<PathBuf> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("mtlx")))
        .collect();
    files.sort();
    files.into_iter().next()
}

/// Legge i valori dal file.
pub fn read(path: &Path) -> anyhow::Result<Vec<MtlxValue>> {
    Ok(values(&std::fs::read_to_string(path)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0"?>
<materialx version="1.38" fileprefix="./">
  <!-- esempio nello stile della specifica: texture collegate e valori scritti -->
  <standard_surface name="Metal049B_StandardSurface" type="surfaceshader">
    <input name="base" type="float" value="1"/>
    <input name="base_color" type="color3" nodename="Metal049B_Color"/>
    <input name="metalness" type="float" nodename="Metal049B_Metalness"/>
    <input name="specular_IOR" type="float" value="1.5" />
    <input name="note" type="string" value="a &amp; b &gt; c"/>
  </standard_surface>
  <displacement name="Metal049B_Displacement" type="displacementshader">
    <input name="displacement" type="float" nodename="Metal049B_Height"/>
    <input name='scale' type='float' value='0.02'/>
  </displacement>
  <image name="Metal049B_Color" type="color3">
    <input name="file" type="filename" value="Metal049B_1K-JPG_Color.jpg" colorspace="srgb_texture"/>
  </image>
</materialx>"#;

    #[test]
    fn reads_written_values_only() {
        let v = values(SAMPLE);
        let get = |node: &str, input: &str| v.iter().find(|x| x.node == node && x.input == input).map(|x| x.value.as_str());
        assert_eq!(get("standard_surface", "base"), Some("1"));
        assert_eq!(get("standard_surface", "specular_IOR"), Some("1.5"));
        assert_eq!(get("standard_surface", "note"), Some("a & b > c"));
        // Collegati a una texture: nessun valore inventato.
        assert_eq!(get("standard_surface", "base_color"), None);
        assert_eq!(get("standard_surface", "metalness"), None);
        assert_eq!(get("image", "file"), Some("Metal049B_1K-JPG_Color.jpg"));
        assert_eq!(displacement_scale(&v), Some(0.02));
        assert_eq!(v.iter().find(|x| x.input == "scale").unwrap().node_name, "Metal049B_Displacement");
    }

    #[test]
    fn tolerates_garbage() {
        assert!(values("").is_empty());
        assert!(values("<materialx><input name=\"x\" value=\"1\"").is_empty());
        assert_eq!(
            displacement_scale(&values("<displacement><input name=\"scale\" value=\"abc\"/></displacement>")),
            None
        );
    }
}
