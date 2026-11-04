//! Contorni a inchiostro sull'immagine finale: sagome, confini tra oggetti diversi, salti di
//! profondità e (se attivi) spigoli vivi. Si ricavano una volta dalla geometria, con 3×3 raggi
//! per pixel dal centro della camera (bordi senza scalini), e si disegnano sopra l'immagine
//! (anche ripulita dal denoiser) in [`crate::Frame::display`]: valgono per CPU e GPU.

use crate::{OutlineSettings, RenderScene, RenderSettings};
use forge_math::Vec3;
use rayon::prelude::*;

/// Raggi per lato di ogni pixel.
const SUB: usize = 3;
const MISS: u32 = u32::MAX;

/// Copertura della linea per pixel (0–1) e colore.
#[derive(Clone, Debug)]
pub struct Outline {
    pub width: u32,
    pub height: u32,
    pub coverage: Vec<f32>,
    pub color: Vec3,
}

impl Outline {
    /// L'immagine con la linea sopra.
    pub fn apply(&self, img: &[[f32; 3]]) -> Vec<[f32; 3]> {
        let c = self.color.to_array();
        img.iter()
            .zip(&self.coverage)
            .map(|(p, &k)| [p[0] + (c[0] - p[0]) * k, p[1] + (c[1] - p[1]) * k, p[2] + (c[2] - p[2]) * k])
            .collect()
    }
}

/// Primo punto colpito da un raggio: oggetto (nodo), distanza lungo l'asse di vista, normale.
#[derive(Clone, Copy)]
struct Sample {
    node: u32,
    depth: f32,
    normal: Vec3,
}

/// Contorni per le impostazioni del render (None se spenti o se l'immagine è vuota).
pub fn compute(scene: &RenderScene, s: &RenderSettings) -> Option<Outline> {
    let o: &OutlineSettings = &s.outlines;
    if !o.enabled || s.width == 0 || s.height == 0 {
        return None;
    }
    let t0 = std::time::Instant::now();
    let (w, h) = (s.width as usize, s.height as usize);
    let (sw, sh) = (w * SUB, h * SUB);
    let cam = &scene.camera;
    let forward = cam.forward();
    let aspect = w as f32 / h as f32;
    let geo = &scene.geometry;
    // La polvere non ha contorni.
    let solid = |prim: u32| !scene.materials[scene.material_index(prim as usize)].dust;
    let samples: Vec<Sample> = (0..sw * sh)
        .into_par_iter()
        .map(|i| {
            let (x, y) = (i % sw, i / sw);
            let nx = 2.0 * (x as f32 + 0.5) / sw as f32 - 1.0 + 2.0 * cam.lens.shift_x;
            let ny = 1.0 - 2.0 * (y as f32 + 0.5) / sh as f32 + 2.0 * cam.lens.shift_y;
            let (ro, rd) = cam.ray(nx, ny, aspect);
            let hit = if scene.has_dust {
                geo.accel.intersect_filtered(ro, rd, 0.0, f32::INFINITY, |p, _, _, _| solid(p))
            } else {
                geo.accel.intersect(ro, rd, 0.0, f32::INFINITY)
            };
            match hit {
                Some(hit) => Sample {
                    node: geo.tri_node[hit.prim as usize],
                    depth: hit.t * rd.dot(forward),
                    normal: geo.geometric_normals[hit.prim as usize],
                },
                None => Sample {
                    node: MISS,
                    depth: f32::INFINITY,
                    normal: Vec3::ZERO,
                },
            }
        })
        .collect();
    let crease_cos = if o.creases { o.crease_deg.to_radians().cos() } else { -2.0 };
    // Bordo tra due raggi vicini: si segna quello più vicino alla camera (la linea sta
    // sull'oggetto davanti, come la disegnerebbe un illustratore).
    let edge_between = |a: &Sample, b: &Sample| -> bool {
        if a.node == MISS && b.node == MISS {
            return false;
        }
        if a.node != b.node {
            return true;
        }
        let near = a.depth.min(b.depth).max(1e-4);
        if (a.depth - b.depth).abs() > 0.03 * near {
            return true;
        }
        // Spigolo vivo: facce dello stesso oggetto con normali molto diverse (non le facce
        // viste da dietro, che hanno la normale rovesciata).
        a.normal.dot(b.normal).abs() < crease_cos
    };
    let mut edge: Vec<bool> = (0..sw * sh)
        .into_par_iter()
        .map(|i| {
            let (x, y) = (i % sw, i / sw);
            let a = &samples[i];
            let mut e = false;
            for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                if nx < 0 || ny < 0 || nx >= sw as i32 || ny >= sh as i32 {
                    continue;
                }
                let b = &samples[ny as usize * sw + nx as usize];
                if edge_between(a, b) && a.depth <= b.depth {
                    e = true;
                    break;
                }
            }
            e
        })
        .collect();
    // Spessore: allargamento separabile (righe, poi colonne) di r raggi per parte.
    let r = ((o.width_px.max(0.5) * SUB as f32 - 1.0) * 0.5).round().max(0.0) as i32;
    if r > 0 {
        let rows: Vec<bool> = (0..sw * sh)
            .into_par_iter()
            .map(|i| {
                let (x, y) = ((i % sw) as i32, i / sw);
                (-r..=r).any(|k| {
                    let xx = x + k;
                    xx >= 0 && (xx as usize) < sw && edge[y * sw + xx as usize]
                })
            })
            .collect();
        edge = (0..sw * sh)
            .into_par_iter()
            .map(|i| {
                let (x, y) = (i % sw, (i / sw) as i32);
                (-r..=r).any(|k| {
                    let yy = y + k;
                    yy >= 0 && (yy as usize) < sh && rows[yy as usize * sw + x]
                })
            })
            .collect();
    }
    let coverage: Vec<f32> = (0..w * h)
        .into_par_iter()
        .map(|i| {
            let (px, py) = (i % w, i / w);
            let mut n = 0;
            for sy in 0..SUB {
                for sx in 0..SUB {
                    if edge[(py * SUB + sy) * sw + px * SUB + sx] {
                        n += 1;
                    }
                }
            }
            n as f32 / (SUB * SUB) as f32
        })
        .collect();
    tracing::info!("contorni: {}×{} in {:.0} ms", w, h, t0.elapsed().as_secs_f64() * 1e3);
    Some(Outline {
        width: s.width,
        height: s.height,
        coverage,
        color: Vec3::from_array(o.color),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_math::Mat4;
    use forge_scene::{primitives, Camera, ComponentId, Scene};

    /// Un cubo davanti alla camera su fondo vuoto: linea sul bordo del cubo, niente al centro
    /// né sullo sfondo.
    #[test]
    fn outline_follows_the_silhouette() {
        let mut s = Scene::default();
        let m = s.add_mesh(primitives::cuboid(Vec3::splat(0.5)));
        s.add_node(None, ComponentId("cubo".into()), "cubo", Mat4::IDENTITY, Some(m));
        s.camera = Camera {
            position: Vec3::new(0.0, 0.0, 4.0),
            target: Vec3::ZERO,
            up: Vec3::Y,
            ..Default::default()
        };
        let rs = RenderScene::compile(&s);
        let st = RenderSettings {
            width: 64,
            height: 64,
            outlines: OutlineSettings {
                enabled: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let o = compute(&rs, &st).expect("contorni");
        let at = |x: usize, y: usize| o.coverage[y * 64 + x];
        assert_eq!(at(32, 32), 0.0, "centro del cubo");
        assert_eq!(at(1, 1), 0.0, "sfondo");
        let row: f32 = (0..64).map(|x| at(x, 32)).sum();
        assert!(row >= 1.0, "il bordo del cubo attraversa la riga centrale: {row}");
        let img = o.apply(&vec![[1.0; 3]; 64 * 64]);
        assert!(img.iter().any(|p| p[0] < 0.5), "la linea scurisce l'immagine");
    }
}
