//! Strumento `inspect_components`: immagini veloci dei pezzi, per far capire all'assistente che
//! cosa sono (forma e funzione) prima di scegliere i materiali, senza un render.
//!
//! Ogni riquadro numerato ha due viste: a sinistra il pezzo da solo, inquadrato; a destra
//! l'assieme intero con il pezzo in arancione (in trasparenza dove è coperto da altro).
//! I pezzi identici (stesse mesh, come viti e ruote ripetute) compaiono una volta sola.
//!
//! Le immagini si disegnano sulla CPU con un'ombreggiatura simile a quella della viewport
//! (z-buffer, luce frontale e d'ambiente, spigoli scuriti): non toccano la scena, non
//! spostano la camera e non salvano file.

use forge_math::{Mat4, Vec3};
use forge_scene::{Camera, Scene};
use rayon::prelude::*;
use std::collections::BTreeMap;

/// Lato di una vista nel riquadro (pixel dell'immagine restituita).
pub const VIEW_PX: usize = 200;
/// Colonne della griglia (ogni riquadro ha due viste affiancate).
pub const COLUMNS: usize = 3;
const GAP: usize = 6;
/// Sovracampionamento per lato (bordi meno seghettati).
const SS: usize = 2;
const HIGHLIGHT: Vec3 = Vec3::new(1.0, 0.42, 0.05);

/// Un pezzo da mostrare: nodi con mesh che lo compongono e copie identiche.
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    /// Nodo che rappresenta il pezzo (la parte, o il nodo con la mesh).
    pub root: usize,
    /// Nodi con mesh del pezzo.
    pub nodes: Vec<usize>,
    /// Altri pezzi con le stesse mesh (radici), nello stesso ordine della scena.
    pub copies: Vec<usize>,
}

/// Direzione da cui si guardano i pezzi (verso il centro), in coordinate della scena.
pub fn view_direction(name: &str) -> Option<Vec3> {
    Some(match name {
        "tre_quarti" => Vec3::new(1.0, 0.8, 1.4),
        "alto" => Vec3::new(0.0, 1.0, 0.02),
        "basso" => Vec3::new(0.0, -1.0, 0.02),
        "fronte" => Vec3::new(0.0, 0.15, 1.0),
        "retro" => Vec3::new(0.0, 0.15, -1.0),
        "lato" => Vec3::new(1.0, 0.15, 0.0),
        _ => return None,
    })
}

/// Pezzi sotto i nodi indicati. Un nodo senza mesh i cui figli sono tutti foglie con mesh
/// (una parte STEP divisa per colore) è un pezzo solo; altrimenti ogni nodo con mesh è un pezzo.
/// Si usano solo i nodi visibili.
pub fn pieces(scene: &Scene, mesh_nodes: &[usize]) -> Vec<Piece> {
    let is_leaf_mesh = |i: usize| scene.nodes[i].mesh.is_some() && scene.nodes[i].children.is_empty();
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for &i in mesh_nodes {
        if !scene.is_visible(i) {
            continue;
        }
        let root = match scene.nodes[i].parent {
            Some(p) if is_leaf_mesh(i) && scene.nodes[p].mesh.is_none() && scene.nodes[p].children.iter().all(|&c| is_leaf_mesh(c)) => p,
            _ => i,
        };
        groups.entry(root).or_default().push(i);
    }
    let mut out: Vec<Piece> = Vec::new();
    let mut by_shape: BTreeMap<Vec<usize>, usize> = BTreeMap::new();
    for (root, nodes) in groups {
        let mut key: Vec<usize> = nodes.iter().filter_map(|&n| scene.nodes[n].mesh).collect();
        key.sort_unstable();
        match by_shape.get(&key) {
            Some(&k) => out[k].copies.push(root),
            None => {
                by_shape.insert(key, out.len());
                out.push(Piece {
                    root,
                    nodes,
                    copies: Vec::new(),
                });
            }
        }
    }
    out
}

/// Triangolo in spazio mondo con colore lineare e indice del pezzo.
#[derive(Clone, Copy)]
struct Tri {
    p: [Vec3; 3],
    color: Vec3,
    piece: u32,
    /// Del pezzo mostrato (non di una sua copia identica).
    main: bool,
}

/// Colore della viewport per un nodo: base del materiale, vetri azzurri, emissivi bianchi.
fn node_color(scene: &Scene, i: usize) -> Vec3 {
    scene
        .materials
        .get(scene.resolve_material(i))
        .map(|m| {
            if m.is_emissive() {
                Vec3::ONE
            } else if m.transmission > 0.5 {
                Vec3::new(0.65, 0.8, 0.9)
            } else {
                Vec3::from_array(m.base_color)
            }
        })
        .unwrap_or(Vec3::splat(0.8))
}

fn triangles(scene: &Scene, nodes: &[usize], piece_of: &dyn Fn(usize) -> (u32, bool)) -> Vec<Tri> {
    let mut out = Vec::new();
    for &i in nodes {
        let Some(m) = scene.nodes[i].mesh else { continue };
        let mesh = &scene.meshes[m];
        let w = scene.world_transform(i);
        let wp: Vec<Vec3> = mesh.positions.iter().map(|p| w.transform_point3(*p)).collect();
        // I colori molto scuri (gomma nera) nasconderebbero la forma: si schiariscono un po'.
        let color = node_color(scene, i) * 0.85 + Vec3::splat(0.12);
        let (piece, main) = piece_of(i);
        out.extend(mesh.indices.iter().map(|t| Tri {
            p: t.map(|k| wp[k as usize]),
            color,
            piece,
            main,
        }));
    }
    out
}

/// Camera che inquadra una sfera da una direzione.
fn framing(center: Vec3, radius: f32, dir: Vec3) -> Camera {
    let vfov: f32 = 30.0;
    let r = radius.max(1e-4);
    let dist = r / (vfov.to_radians() * 0.5).sin() * 1.08;
    Camera {
        position: center + dir.normalize() * dist,
        target: center,
        up: if dir.normalize().y.abs() > 0.95 { Vec3::Z } else { Vec3::Y },
        vfov_deg: vfov,
        ..Default::default()
    }
}

fn sphere_of(tris: &[Tri]) -> (Vec3, f32) {
    let mut b = forge_math::Aabb::EMPTY;
    for t in tris {
        for p in t.p {
            b.grow_point(p);
        }
    }
    if b.is_empty() {
        return (Vec3::ZERO, 1.0);
    }
    (b.center(), (b.max - b.min).length() * 0.5)
}

/// Immagine sovracampionata: colore, profondità, pezzo e normale per pixel (spigoli).
struct Frame {
    w: usize,
    h: usize,
    color: Vec<Vec3>,
    depth: Vec<f32>,
    piece: Vec<u32>,
    normal: Vec<Vec3>,
}

const NO_PIECE: u32 = u32::MAX;

/// Pixel sRGB di una vista, riga per riga.
type Pixels = Vec<[u8; 3]>;

impl Frame {
    fn new(w: usize, h: usize, bg: Vec3) -> Frame {
        Frame {
            w,
            h,
            color: vec![bg; w * h],
            depth: vec![f32::INFINITY; w * h],
            piece: vec![NO_PIECE; w * h],
            normal: vec![Vec3::ZERO; w * h],
        }
    }

    /// Disegna i triangoli con z-buffer. `color_of` decide il colore (prima dell'ombreggiatura).
    fn draw(&mut self, tris: &[Tri], cam: &Camera, color_of: &dyn Fn(&Tri) -> Vec3) {
        let (r, u, f) = cam.basis();
        let light = (-f + u * 0.8 - r * 0.5).normalize();
        let dist = (cam.position - cam.target).length();
        let far = dist * 4.0 + 1.0;
        let near = (dist * 1e-3).max(1e-5);
        let vp: Mat4 = cam.projection_matrix(self.w as f32 / self.h as f32, near, far) * cam.view_matrix();
        let (wf, hf) = (self.w as f32, self.h as f32);
        for t in tris {
            let clip = t.p.map(|p| vp * p.extend(1.0));
            if clip.iter().any(|c| c.w <= near) {
                continue;
            }
            let s = clip.map(|c| Vec3::new((c.x / c.w * 0.5 + 0.5) * wf, (0.5 - c.y / c.w * 0.5) * hf, c.z / c.w));
            let area = (s[1].x - s[0].x) * (s[2].y - s[0].y) - (s[1].y - s[0].y) * (s[2].x - s[0].x);
            if area.abs() < 1e-12 {
                continue;
            }
            let x0 = s.iter().map(|p| p.x).fold(f32::INFINITY, f32::min).floor().max(0.0) as usize;
            let x1 = s.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max).ceil().min(wf - 1.0);
            let y0 = s.iter().map(|p| p.y).fold(f32::INFINITY, f32::min).floor().max(0.0) as usize;
            let y1 = s.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max).ceil().min(hf - 1.0);
            if x1 < 0.0 || y1 < 0.0 {
                continue;
            }
            let (x1, y1) = (x1 as usize, y1 as usize);
            // Ombreggiatura piatta come nella viewport (luce frontale + emisfero).
            let mut n = (t.p[1] - t.p[0]).cross(t.p[2] - t.p[0]).normalize_or_zero();
            let v = (cam.position - (t.p[0] + t.p[1] + t.p[2]) / 3.0).normalize_or_zero();
            if n.dot(v) < 0.0 {
                n = -n;
            }
            let key = n.dot(light).max(0.0);
            let head = n.dot(v).max(0.0);
            let hemi = 0.18 + (0.4 - 0.18) * (n.y * 0.5 + 0.5);
            let shaded = color_of(t) * (hemi + 0.5 * key + 0.3 * head);
            for y in y0..=y1 {
                for x in x0..=x1 {
                    let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                    let e = |a: Vec3, b: Vec3| (b.x - a.x) * (py - a.y) - (b.y - a.y) * (px - a.x);
                    let (w0, w1, w2) = (e(s[1], s[2]) / area, e(s[2], s[0]) / area, e(s[0], s[1]) / area);
                    if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                        continue;
                    }
                    let z = w0 * s[0].z + w1 * s[1].z + w2 * s[2].z;
                    let k = y * self.w + x;
                    if z < self.depth[k] {
                        self.depth[k] = z;
                        self.color[k] = shaded;
                        self.piece[k] = t.piece;
                        self.normal[k] = n;
                    }
                }
            }
        }
    }

    /// Pixel coperti dai triangoli (senza profondità): per la trasparenza dei pezzi coperti.
    fn coverage(&self, tris: &[Tri], cam: &Camera) -> Vec<bool> {
        let mut f = Frame::new(self.w, self.h, Vec3::ZERO);
        f.draw(tris, cam, &|_| Vec3::ONE);
        f.depth.iter().map(|d| d.is_finite()).collect()
    }

    /// Spigoli scuriti (cambio di pezzo, di orientamento o bordo con lo sfondo), poi riduzione
    /// del sovracampionamento. Restituisce i pixel sRGB della vista.
    fn finish(&self) -> Pixels {
        let (w, h) = (self.w, self.h);
        let edge = |k: usize, j: usize| {
            self.depth[k].is_finite() != self.depth[j].is_finite()
                || (self.depth[k].is_finite() && (self.piece[k] != self.piece[j] || self.normal[k].dot(self.normal[j]) < 0.8))
        };
        let (ow, oh) = (w / SS, h / SS);
        let mut out = vec![[0u8; 3]; ow * oh];
        for oy in 0..oh {
            for ox in 0..ow {
                let mut acc = Vec3::ZERO;
                for sy in 0..SS {
                    for sx in 0..SS {
                        let (x, y) = (ox * SS + sx, oy * SS + sy);
                        let k = y * w + x;
                        let mut c = self.color[k];
                        let near_edge = (x + 1 < w && edge(k, k + 1)) || (y + 1 < h && edge(k, k + w));
                        if near_edge {
                            c *= 0.35;
                        }
                        acc += c;
                    }
                }
                let c = acc / (SS * SS) as f32;
                out[oy * ow + ox] = c
                    .to_array()
                    .map(|v| (forge_scene::material::linear_to_srgb(v.clamp(0.0, 1.0)) * 255.0 + 0.5) as u8);
            }
        }
        out
    }
}

/// Le due viste di un pezzo: da solo e nell'assieme.
fn piece_views(all: &[Tri], piece: u32, dir: Vec3, context_cam: &Camera) -> (Pixels, Pixels) {
    let n = VIEW_PX * SS;
    let own: Vec<Tri> = all.iter().filter(|t| t.piece == piece && t.main).copied().collect();
    let (c, r) = sphere_of(&own);
    let cam = framing(c, r, dir);
    let mut alone = Frame::new(n, n, Vec3::splat(0.86));
    alone.draw(&own, &cam, &|t| t.color);
    // Assieme: gli altri pezzi più chiari e meno saturi, il pezzo in arancione; dove è coperto
    // da altro si vede in trasparenza.
    let mut ctx = Frame::new(n, n, Vec3::splat(0.8));
    ctx.draw(all, context_cam, &|t| {
        if t.piece == piece {
            HIGHLIGHT
        } else {
            t.color * 0.35 + Vec3::splat(0.5)
        }
    });
    let copies: Vec<Tri> = all.iter().filter(|t| t.piece == piece).copied().collect();
    let covered = ctx.coverage(&copies, context_cam);
    for (k, &cov) in covered.iter().enumerate() {
        if cov && ctx.piece[k] != piece {
            ctx.color[k] = ctx.color[k] * 0.5 + HIGHLIGHT * 0.5;
        }
    }
    (alone.finish(), ctx.finish())
}

/// Cifre 3×5 per i numeri dei riquadri.
const DIGITS: [[u8; 5]; 10] = [
    [0b111, 0b101, 0b101, 0b101, 0b111],
    [0b010, 0b110, 0b010, 0b010, 0b111],
    [0b111, 0b001, 0b111, 0b100, 0b111],
    [0b111, 0b001, 0b111, 0b001, 0b111],
    [0b101, 0b101, 0b111, 0b001, 0b001],
    [0b111, 0b100, 0b111, 0b001, 0b111],
    [0b111, 0b100, 0b111, 0b101, 0b111],
    [0b111, 0b001, 0b010, 0b010, 0b010],
    [0b111, 0b101, 0b111, 0b101, 0b111],
    [0b111, 0b101, 0b111, 0b001, 0b111],
];

/// Numero in alto a sinistra, bianco su fondo scuro.
fn draw_label(img: &mut image::RgbImage, x: u32, y: u32, n: usize) {
    let text = n.to_string();
    let scale = 4;
    let w = (text.len() as u32 * 4 + 1) * scale;
    let h = 7 * scale;
    for yy in y..(y + h).min(img.height()) {
        for xx in x..(x + w).min(img.width()) {
            img.put_pixel(xx, yy, image::Rgb([30, 30, 30]));
        }
    }
    for (i, ch) in text.bytes().enumerate() {
        let glyph = DIGITS[(ch - b'0') as usize];
        for (row, bits) in glyph.iter().enumerate() {
            for col in 0..3 {
                if bits & (0b100 >> col) == 0 {
                    continue;
                }
                for dy in 0..scale {
                    for dx in 0..scale {
                        let px = x + (1 + i as u32 * 4 + col) * scale + dx;
                        let py = y + (1 + row as u32) * scale + dy;
                        if px < img.width() && py < img.height() {
                            img.put_pixel(px, py, image::Rgb([255, 255, 255]));
                        }
                    }
                }
            }
        }
    }
}

/// Griglia dei pezzi `shown` (indici in `pieces`), numerati da `first_number`. PNG.
pub fn contact_sheet(scene: &Scene, pieces: &[Piece], shown: &[usize], first_number: usize, dir: Vec3) -> Vec<u8> {
    // Triangoli dell'assieme: tutti i pezzi elencati e le loro copie.
    let mut piece_of: BTreeMap<usize, (u32, bool)> = BTreeMap::new();
    let mut nodes = Vec::new();
    for (k, p) in pieces.iter().enumerate() {
        for (c, &root) in std::iter::once(&p.root).chain(&p.copies).enumerate() {
            let under = super::ops::mesh_nodes_under(scene, &[root]);
            for i in under.into_iter().filter(|&i| scene.is_visible(i)) {
                piece_of.insert(i, (k as u32, c == 0));
                nodes.push(i);
            }
        }
    }
    let all = triangles(scene, &nodes, &|i| piece_of[&i]);
    let (c, r) = sphere_of(&all);
    let context_cam = framing(c, r, dir);
    let views: Vec<(Pixels, Pixels)> = shown.par_iter().map(|&k| piece_views(&all, k as u32, dir, &context_cam)).collect();
    let tile_w = 2 * VIEW_PX + GAP;
    let rows = shown.len().div_ceil(COLUMNS).max(1);
    let cols = shown.len().clamp(1, COLUMNS);
    let (iw, ih) = (cols * (tile_w + GAP) + GAP, rows * (VIEW_PX + GAP) + GAP);
    let mut img = image::RgbImage::from_pixel(iw as u32, ih as u32, image::Rgb([255, 255, 255]));
    for (slot, (alone, ctx)) in views.iter().enumerate() {
        let (tx, ty) = (GAP + (slot % COLUMNS) * (tile_w + GAP), GAP + (slot / COLUMNS) * (VIEW_PX + GAP));
        for (ox, px) in [(tx, alone), (tx + VIEW_PX + GAP, ctx)] {
            for y in 0..VIEW_PX {
                for x in 0..VIEW_PX {
                    img.put_pixel((ox + x) as u32, (ty + y) as u32, image::Rgb(px[y * VIEW_PX + x]));
                }
            }
        }
        draw_label(&mut img, tx as u32 + 3, ty as u32 + 3, first_number + slot);
    }
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img)
        .write_to(&mut out, image::ImageFormat::Png)
        .expect("PNG in memoria");
    out.into_inner()
}

/// Ingombro in spazio mondo dei nodi con mesh di un pezzo (vertici veri).
pub fn piece_bounds(scene: &Scene, p: &Piece) -> forge_math::Aabb {
    let mut b = forge_math::Aabb::EMPTY;
    for &i in &p.nodes {
        b.grow(&scene.exact_bounds_of(i));
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_scene::{primitives, ComponentId};

    /// Assieme come uno STEP: una parte divisa per colore (due foglie), due viti uguali (stessa
    /// mesh) e un cubo con mesh propria.
    fn assembly() -> (Scene, [usize; 4]) {
        let mut s = Scene::default();
        let cube = s.add_mesh(primitives::cuboid(Vec3::splat(0.1)));
        let screw = s.add_mesh(primitives::uv_sphere(0.01, 8, 4));
        let body = s.add_mesh(primitives::cuboid(Vec3::splat(0.05)));
        let root = s.add_node(None, ComponentId("a".into()), "Assieme", Mat4::IDENTITY, None);
        let part = s.add_node(Some(root), ComponentId("a/p".into()), "Parte", Mat4::IDENTITY, None);
        s.add_node(
            Some(part),
            ComponentId("a/p/1".into()),
            "Parte · colore 1",
            Mat4::IDENTITY,
            Some(cube),
        );
        s.add_node(
            Some(part),
            ComponentId("a/p/2".into()),
            "Parte · colore 2",
            Mat4::from_translation(Vec3::X * 0.2),
            Some(body),
        );
        let v1 = s.add_node(
            Some(root),
            ComponentId("a/v1".into()),
            "Vite 1",
            Mat4::from_translation(Vec3::Y * 0.2),
            Some(screw),
        );
        let v2 = s.add_node(
            Some(root),
            ComponentId("a/v2".into()),
            "Vite 2",
            Mat4::from_translation(Vec3::Y * 0.3),
            Some(screw),
        );
        let b = s.add_node(
            Some(root),
            ComponentId("a/b".into()),
            "Blocco",
            Mat4::from_translation(Vec3::Z * 0.3),
            Some(body),
        );
        s.refresh_pivots();
        (s, [part, v1, v2, b])
    }

    #[test]
    fn groups_color_parts_and_merges_identical_pieces() {
        let (s, [part, v1, v2, b]) = assembly();
        let nodes = super::super::ops::mesh_nodes_under(&s, &crate::presets::product_roots(&s));
        let p = pieces(&s, &nodes);
        assert_eq!(p.len(), 3, "{p:?}");
        assert_eq!(p[0].root, part, "la parte con i colori è un pezzo solo");
        assert_eq!(p[0].nodes.len(), 2);
        let screw = p.iter().find(|x| x.root == v1).unwrap();
        assert_eq!(screw.copies, vec![v2], "viti uguali mostrate una volta");
        assert!(
            p.iter().any(|x| x.root == b && x.copies.is_empty()),
            "stessa mesh ma pezzo diverso dalla parte"
        );
    }

    #[test]
    fn sheet_shows_pieces_and_leaves_the_scene_alone() {
        let (s, _) = assembly();
        let state = |s: &Scene| format!("{:?} {:?} {:?}", s.nodes, s.camera, s.assignments);
        let before = state(&s);
        let nodes = super::super::ops::mesh_nodes_under(&s, &crate::presets::product_roots(&s));
        let p = pieces(&s, &nodes);
        let png = contact_sheet(&s, &p, &[0, 1, 2], 1, view_direction("tre_quarti").unwrap());
        assert_eq!(state(&s), before, "la scena non cambia");
        let img = image::load_from_memory(&png).unwrap().to_rgb8();
        if let Ok(dir) = std::env::var("FORGE_TEST_OUT") {
            std::fs::write(std::path::Path::new(&dir).join("inspect_test.png"), &png).unwrap();
        }
        let tile_w = 2 * VIEW_PX + GAP;
        assert_eq!(img.width() as usize, 3 * (tile_w + GAP) + GAP);
        assert_eq!(img.height() as usize, VIEW_PX + 2 * GAP);
        // Nella vista "da solo" di ogni riquadro il pezzo copre una parte ben visibile.
        let bg = (forge_scene::material::linear_to_srgb(0.86) * 255.0 + 0.5) as u8;
        for slot in 0..3 {
            let x0 = GAP + slot * (tile_w + GAP);
            let drawn = (0..VIEW_PX)
                .flat_map(|y| (0..VIEW_PX).map(move |x| (x, y)))
                .filter(|&(x, y)| img.get_pixel((x0 + x) as u32, (GAP + y) as u32).0 != [bg; 3])
                .count();
            assert!(drawn > VIEW_PX * VIEW_PX / 20, "riquadro {slot}: {drawn} pixel del pezzo");
        }
        // Nella vista d'assieme c'è l'arancione del pezzo evidenziato.
        let ctx_x0 = GAP + VIEW_PX + GAP;
        let orange = (0..VIEW_PX).flat_map(|y| (0..VIEW_PX).map(move |x| (x, y))).any(|(x, y)| {
            // In i32: con u8 "g + 50" trabocca (e in debug il test si ferma) sui pixel chiari.
            let [r, g, b] = img.get_pixel((ctx_x0 + x) as u32, (GAP + y) as u32).0.map(i32::from);
            r > 150 && r > g + 50 && g > b + 30
        });
        assert!(orange, "pezzo evidenziato nell'assieme");
    }

    #[test]
    fn view_names() {
        for v in ["tre_quarti", "alto", "basso", "fronte", "retro", "lato"] {
            assert!(view_direction(v).is_some());
        }
        assert!(view_direction("boh").is_none());
    }

    /// Prova su un modello vero già convertito (`forgerender-step modello.step out.frmesh`):
    /// `FORGE_INSPECT_FRMESH=…\out.frmesh FORGE_TEST_OUT=cartella cargo test -- --ignored real_model`.
    #[test]
    #[ignore]
    fn real_model_sheets() {
        let path = std::env::var("FORGE_INSPECT_FRMESH").expect("FORGE_INSPECT_FRMESH");
        let out = std::path::PathBuf::from(std::env::var("FORGE_TEST_OUT").expect("FORGE_TEST_OUT"));
        let model = forge_import::frmesh::FrMesh::read(std::path::Path::new(&path)).unwrap();
        let mut s = Scene::default();
        forge_import::step::insert(&mut s, "modello.step", model, &forge_import::ImportOptions::default()).unwrap();
        s.refresh_pivots();
        let nodes = super::super::ops::mesh_nodes_under(&s, &crate::presets::product_roots(&s));
        let p = pieces(&s, &nodes);
        eprintln!("{} pezzi diversi su {} nodi con mesh", p.len(), nodes.len());
        for (page, chunk) in (0..p.len()).collect::<Vec<_>>().chunks(12).enumerate() {
            let t = std::time::Instant::now();
            let png = contact_sheet(&s, &p, chunk, chunk[0] + 1, view_direction("tre_quarti").unwrap());
            eprintln!("pagina {}: {:.2} s, {} KB", page + 1, t.elapsed().as_secs_f32(), png.len() / 1024);
            std::fs::write(out.join(format!("inspect_pagina_{}.png", page + 1)), png).unwrap();
        }
    }
}
