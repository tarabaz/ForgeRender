//! Riconoscimento delle mappe PBR dai nomi dei file.
//!
//! Il nome viene diviso in parole (separatori `_ - . spazio` e passaggi
//! minuscola→maiuscola, così `BaseColor` diventa `base color`) e confrontato
//! con i nomi usati dalle librerie più diffuse:
//!
//! | Mappa | Parole riconosciute |
//! |---|---|
//! | colore | color, colour, basecolor, albedo, diffuse, diff, col |
//! | rugosità | roughness, rough, rgh |
//! | lucidità (invertita) | gloss, glossiness |
//! | normal | normal, nor, nrm, norm (+ `gl`/`opengl` o `dx`/`directx`) |
//! | metallicità | metalness, metallic, metal |
//! | rilievo | displacement, disp, height, bump |
//! | occlusione | ao, ambientocclusion, occlusion |
//! | opacità | opacity, alpha, mask, transparency, cutout |
//! | impacchettata | arm, orm (R = occlusione, G = rugosità, B = metallicità) |
//!
//! Mappe ignorate: anteprime, cavity, specular, translucency, id.

use std::path::{Path, PathBuf};

pub const IMAGE_EXTENSIONS: [&str; 9] = ["png", "jpg", "jpeg", "tif", "tiff", "tga", "bmp", "exr", "hdr"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MapKind {
    BaseColor,
    Roughness,
    Gloss,
    /// `true` = convenzione DirectX.
    Normal(bool),
    Metallic,
    Height,
    Ao,
    Opacity,
    /// AO/rugosità/metallicità in R/G/B.
    Packed,
}

/// Mappe trovate per un materiale.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MapSet {
    pub base_color: Option<PathBuf>,
    pub roughness: Option<PathBuf>,
    /// La mappa in `roughness` è in realtà di lucidità (gloss).
    pub roughness_is_gloss: bool,
    pub normal: Option<PathBuf>,
    pub normal_directx: bool,
    pub metallic: Option<PathBuf>,
    pub height: Option<PathBuf>,
    pub ao: Option<PathBuf>,
    pub opacity: Option<PathBuf>,
    pub packed: Option<PathBuf>,
}

impl MapSet {
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    pub fn count(&self) -> usize {
        [
            &self.base_color,
            &self.roughness,
            &self.normal,
            &self.metallic,
            &self.height,
            &self.ao,
            &self.opacity,
            &self.packed,
        ]
        .iter()
        .filter(|p| p.is_some())
        .count()
    }
}

/// Parole del nome del file (senza estensione), in minuscolo.
pub fn words(stem: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut prev_lower = false;
    for ch in stem.chars() {
        if !ch.is_alphanumeric() {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            prev_lower = false;
            continue;
        }
        if ch.is_uppercase() && prev_lower && !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
        prev_lower = ch.is_lowercase();
        cur.extend(ch.to_lowercase());
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Tipo di mappa di un file immagine, dal nome.
pub fn classify(path: &Path) -> Option<MapKind> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    if !IMAGE_EXTENSIONS.contains(&ext.as_str()) {
        return None;
    }
    let w = words(path.file_stem()?.to_str()?);
    let has = |list: &[&str]| w.iter().any(|x| list.contains(&x.as_str()));
    if has(&[
        "preview",
        "thumb",
        "thumbnail",
        "sphere",
        "render",
        "cavity",
        "specular",
        "spec",
        "translucency",
        "idmap",
        "id",
        "swatch",
    ]) {
        return None;
    }
    if has(&["normal", "nor", "nrm", "norm", "normalgl", "normaldx", "normalmap"]) {
        let dx = has(&["dx", "directx", "normaldx"]);
        return Some(MapKind::Normal(dx));
    }
    if has(&["arm", "orm"]) {
        return Some(MapKind::Packed);
    }
    if has(&["roughness", "rough", "rgh"]) {
        return Some(MapKind::Roughness);
    }
    if has(&["gloss", "glossiness"]) {
        return Some(MapKind::Gloss);
    }
    if has(&["metalness", "metallic", "metal"]) {
        return Some(MapKind::Metallic);
    }
    if has(&["displacement", "disp", "height", "bump", "heightmap"]) {
        return Some(MapKind::Height);
    }
    if has(&["ao", "ambientocclusion", "occlusion"]) {
        return Some(MapKind::Ao);
    }
    if has(&["opacity", "alpha", "mask", "transparency", "cutout"]) {
        return Some(MapKind::Opacity);
    }
    if has(&["color", "colour", "basecolor", "albedo", "diffuse", "diff", "col", "basecol"]) {
        return Some(MapKind::BaseColor);
    }
    None
}

/// Parole che indicano il tipo di mappa (da togliere per ricavare il nome del materiale).
const MAP_WORDS: [&str; 41] = [
    "color",
    "colour",
    "basecolor",
    "albedo",
    "diffuse",
    "diff",
    "col",
    "basecol",
    "base",
    "roughness",
    "rough",
    "rgh",
    "gloss",
    "glossiness",
    "normal",
    "nor",
    "nrm",
    "norm",
    "normalgl",
    "normaldx",
    "gl",
    "dx",
    "opengl",
    "directx",
    "metalness",
    "metallic",
    "metal",
    "displacement",
    "disp",
    "height",
    "bump",
    "ao",
    "ambient",
    "occlusion",
    "ambientocclusion",
    "opacity",
    "alpha",
    "mask",
    "arm",
    "orm",
    "mixed",
];

/// Nome del materiale ricavato dal nome di una mappa: "Bricks_2K_Color" → "Bricks",
/// "rock_wall_diff_2k" → "rock wall", "Wood049_2K-JPG_NormalGL" → "Wood049".
pub fn material_name_from_file(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    let parts: Vec<&str> = stem
        .split(['_', '-', ' ', '.'])
        .filter(|t| !t.is_empty())
        .filter(|t| {
            let l = t.to_lowercase();
            let res = l.ends_with('k') && l.len() > 1 && l[..l.len() - 1].chars().all(|c| c.is_ascii_digit());
            let fmt = ["jpg", "jpeg", "png", "exr", "tif", "tiff"].contains(&l.as_str());
            // "NormalGL", "BaseColor": si controllano anche le parole interne.
            let all_map = words(t).iter().all(|w| MAP_WORDS.contains(&w.as_str()));
            !(res || fmt || all_map)
        })
        .collect();
    (!parts.is_empty()).then(|| parts.join(" "))
}

/// Preferenza tra più file dello stesso tipo: PNG > TIFF/TGA > JPG > EXR/HDR (8 bit per canale nel renderer).
fn format_rank(p: &Path) -> u8 {
    match p.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref() {
        Some("png") => 0,
        Some("tif" | "tiff" | "tga") => 1,
        Some("jpg" | "jpeg") => 2,
        Some("bmp") => 3,
        _ => 4,
    }
}

/// Raggruppa i file di una cartella (o un elenco) nelle mappe di un materiale.
/// Per la normal map si preferisce la versione OpenGL se ci sono entrambe.
pub fn detect(files: &[PathBuf]) -> MapSet {
    let mut set = MapSet::default();
    let pick = |slot: &mut Option<PathBuf>, p: &Path| {
        if slot.as_ref().is_none_or(|old| format_rank(p) < format_rank(old)) {
            *slot = Some(p.to_path_buf());
        }
    };
    let mut gloss: Option<PathBuf> = None;
    let mut normal_gl: Option<PathBuf> = None;
    let mut normal_dx: Option<PathBuf> = None;
    let mut sorted = files.to_vec();
    sorted.sort();
    for f in &sorted {
        match classify(f) {
            Some(MapKind::BaseColor) => pick(&mut set.base_color, f),
            Some(MapKind::Roughness) => pick(&mut set.roughness, f),
            Some(MapKind::Gloss) => pick(&mut gloss, f),
            Some(MapKind::Normal(false)) => pick(&mut normal_gl, f),
            Some(MapKind::Normal(true)) => pick(&mut normal_dx, f),
            Some(MapKind::Metallic) => pick(&mut set.metallic, f),
            Some(MapKind::Height) => pick(&mut set.height, f),
            Some(MapKind::Ao) => pick(&mut set.ao, f),
            Some(MapKind::Opacity) => pick(&mut set.opacity, f),
            Some(MapKind::Packed) => pick(&mut set.packed, f),
            None => {}
        }
    }
    if set.roughness.is_none() {
        if let Some(g) = gloss {
            set.roughness = Some(g);
            set.roughness_is_gloss = true;
        }
    }
    match (normal_gl, normal_dx) {
        (Some(gl), _) => set.normal = Some(gl),
        (None, Some(dx)) => {
            set.normal = Some(dx);
            set.normal_directx = true;
        }
        _ => {}
    }
    set
}

/// File immagine di una cartella (anche nelle sottocartelle di primo livello).
pub fn image_files(dir: &Path) -> Vec<PathBuf> {
    crate::paths::list_files(dir, &IMAGE_EXTENSIONS, 1)
}

/// Divide una mappa ARM/ORM in tre PNG in scala di grigi (occlusione, rugosità,
/// metallicità) dentro `out_dir`. Restituisce i percorsi creati.
pub fn split_packed(packed: &Path, out_dir: &Path, stem: &str) -> anyhow::Result<(PathBuf, PathBuf, PathBuf)> {
    let img = image::open(packed)?.to_rgb8();
    let (w, h) = img.dimensions();
    let mut out = Vec::new();
    for (c, name) in [(0usize, "AO"), (1, "Roughness"), (2, "Metalness")] {
        let ch = image::GrayImage::from_fn(w, h, |x, y| image::Luma([img.get_pixel(x, y).0[c]]));
        let p = out_dir.join(format!("{stem}_{name}.png"));
        ch.save(&p)?;
        out.push(p);
    }
    let mut it = out.into_iter();
    Ok((it.next().unwrap(), it.next().unwrap(), it.next().unwrap()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(name: &str) -> Option<MapKind> {
        classify(Path::new(name))
    }

    #[test]
    fn names_from_files() {
        let n = |s: &str| material_name_from_file(Path::new(s));
        assert_eq!(n("Bricks_2K_Color.jpg").as_deref(), Some("Bricks"));
        assert_eq!(n("rock_wall_diff_2k.jpg").as_deref(), Some("rock wall"));
        assert_eq!(n("Wood049_2K-JPG_NormalGL.png").as_deref(), Some("Wood049"));
        assert_eq!(n("T_Fabric_BaseColor.png").as_deref(), Some("T Fabric"));
        assert_eq!(n("Color.png"), None);
    }

    #[test]
    fn splits_words() {
        assert_eq!(words("Wood049_1K-JPG_NormalGL"), ["wood049", "1k", "jpg", "normal", "gl"]);
        assert_eq!(words("T_Brick_BaseColor"), ["t", "brick", "base", "color"]);
    }

    #[test]
    fn recognizes_common_libraries() {
        // ambientCG
        assert_eq!(k("Wood049_2K-JPG_Color.jpg"), Some(MapKind::BaseColor));
        assert_eq!(k("Wood049_2K-JPG_NormalGL.jpg"), Some(MapKind::Normal(false)));
        assert_eq!(k("Wood049_2K-JPG_NormalDX.jpg"), Some(MapKind::Normal(true)));
        assert_eq!(k("Wood049_2K-JPG_Roughness.jpg"), Some(MapKind::Roughness));
        assert_eq!(k("Wood049_2K-JPG_Displacement.jpg"), Some(MapKind::Height));
        assert_eq!(k("Wood049_2K-JPG_AmbientOcclusion.jpg"), Some(MapKind::Ao));
        assert_eq!(k("Metal012_2K-JPG_Metalness.jpg"), Some(MapKind::Metallic));
        assert_eq!(k("Leaf001_2K-JPG_Opacity.jpg"), Some(MapKind::Opacity));
        assert_eq!(k("Wood049.png"), None);
        assert_eq!(k("Wood049_PREVIEW.jpg"), None);
        // Poly Haven
        assert_eq!(k("rock_wall_diff_2k.jpg"), Some(MapKind::BaseColor));
        assert_eq!(k("rock_wall_nor_gl_2k.png"), Some(MapKind::Normal(false)));
        assert_eq!(k("rock_wall_nor_dx_2k.png"), Some(MapKind::Normal(true)));
        assert_eq!(k("rock_wall_rough_2k.jpg"), Some(MapKind::Roughness));
        assert_eq!(k("rock_wall_disp_2k.png"), Some(MapKind::Height));
        assert_eq!(k("rock_wall_arm_2k.jpg"), Some(MapKind::Packed));
        assert_eq!(k("rock_wall_ao_2k.jpg"), Some(MapKind::Ao));
        // Substance / Quixel
        assert_eq!(k("Fabric_BaseColor.png"), Some(MapKind::BaseColor));
        assert_eq!(k("Fabric_Normal_OpenGL.png"), Some(MapKind::Normal(false)));
        assert_eq!(k("Fabric_Height.png"), Some(MapKind::Height));
        assert_eq!(k("Fabric_Mixed_AO.png"), Some(MapKind::Ao));
        assert_eq!(k("ugkmbdkn_4K_Albedo.jpg"), Some(MapKind::BaseColor));
        assert_eq!(k("ugkmbdkn_4K_Gloss.jpg"), Some(MapKind::Gloss));
        assert_eq!(k("ugkmbdkn_4K_Cavity.jpg"), None);
        assert_eq!(k("ugkmbdkn_4K_Specular.jpg"), None);
        assert_eq!(k("notes.txt"), None);
    }

    #[test]
    fn detect_prefers_png_and_opengl_and_uses_gloss() {
        let f = |n: &str| PathBuf::from(n);
        let set = detect(&[
            f("x_Color.jpg"),
            f("x_Color.png"),
            f("x_NormalDX.png"),
            f("x_NormalGL.png"),
            f("x_Gloss.jpg"),
            f("x_Displacement.png"),
        ]);
        assert_eq!(set.base_color, Some(f("x_Color.png")));
        assert_eq!(set.normal, Some(f("x_NormalGL.png")));
        assert!(!set.normal_directx);
        assert_eq!(set.roughness, Some(f("x_Gloss.jpg")));
        assert!(set.roughness_is_gloss);
        assert_eq!(set.count(), 4);
        let dx = detect(&[f("y_nor_dx_1k.jpg")]);
        assert!(dx.normal_directx);
    }
}
