//! Nomi in italiano delle categorie dei siti (ambientCG, Poly Haven).
//!
//! Nel `materiale.json` resta la categoria originale del sito; qui c'è solo l'etichetta
//! mostrata nell'app. Una categoria sconosciuta si mostra com'è.

/// Etichetta italiana della categoria `cat` (come la scrive il sito).
pub fn label(cat: &str) -> String {
    let c = cat.trim().to_lowercase();
    if let Some(rest) = c.strip_prefix("collection:") {
        return format!("Collezione {}", rest.trim().replace('_', " "));
    }
    let it = match c.as_str() {
        "floor" => "Pavimenti",
        "wall" => "Muri",
        "wood" => "Legno",
        "raw wood" => "Legno grezzo",
        "planks" => "Assi di legno",
        "plaster-concrete" => "Intonaco e cemento",
        "plaster" => "Intonaco",
        "concrete" => "Cemento",
        "terrain" | "ground" => "Terreno",
        "rock" | "rocks" => "Roccia",
        "sandstone" => "Arenaria",
        "marble" => "Marmo",
        "brick" | "bricks" => "Mattoni",
        "sand" => "Sabbia",
        "gravel" => "Ghiaia",
        "snow" => "Neve",
        "cobblestone" | "paving stones" | "pavingstones" => "Ciottolato e pavé",
        "road" => "Strade",
        "asphalt" => "Asfalto",
        "aerial" => "Vista aerea",
        "metal" => "Metallo",
        "bark" => "Corteccia",
        "roofing" | "roofing tiles" | "roofingtiles" => "Coperture",
        "tiles" => "Piastrelle",
        "fabric" => "Tessuti",
        "cotton" => "Cotone",
        "denim" => "Denim",
        "stretchy" => "Elasticizzati",
        "leather" => "Pelle",
        "wool" => "Lana",
        "fleece" => "Pile",
        "knitted" => "Maglia",
        "hessian" => "Juta",
        "fur" => "Pelliccia",
        "crepe" => "Crêpe",
        "jacquard" => "Jacquard",
        "suede" => "Scamosciato",
        "velvet" => "Velluto",
        "woven" => "Intrecciati",
        "satin" => "Raso",
        "carpet" => "Moquette",
        "corduroy" => "Velluto a coste",
        "food" => "Cibo",
        "industrial" => "Industriale",
        "plastic" => "Plastica",
        "paint" => "Vernice",
        "outdoor" => "Esterni",
        "indoor" => "Interni",
        "man made" => "Artificiali",
        "natural" => "Naturali",
        "dirty" => "Sporchi",
        "clean" => "Puliti",
        _ => return cat.trim().to_string(),
    };
    it.to_string()
}

/// Categorie generiche (ambiente, origine, stato, collezioni): poco utili per trovare un
/// materiale, si mostrano in fondo all'elenco.
pub fn is_secondary(cat: &str) -> bool {
    let c = cat.trim().to_lowercase();
    c.starts_with("collection:") || ["outdoor", "indoor", "man made", "natural", "dirty", "clean"].contains(&c.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels() {
        assert_eq!(label("wood"), "Legno");
        assert_eq!(label("Wood"), "Legno");
        assert_eq!(label("plaster-concrete"), "Intonaco e cemento");
        assert_eq!(label("collection: pine_forest"), "Collezione pine forest");
        assert_eq!(label("Sconosciuta"), "Sconosciuta");
        assert!(is_secondary("man made") && is_secondary("collection: moon"));
        assert!(!is_secondary("brick"));
    }
}
