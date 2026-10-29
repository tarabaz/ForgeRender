//! Primitive procedurali per scene di test (nessun asset esterno da scaricare).

use crate::mesh::Mesh;
use forge_math::{Vec2, Vec3};
use std::f32::consts::{PI, TAU};

/// Parallelepipedo centrato nell'origine con dimensioni `size`.
pub fn cuboid(size: Vec3) -> Mesh {
    let h = size * 0.5;
    let mut m = Mesh {
        name: "Box".into(),
        ..Default::default()
    };
    let faces: [(Vec3, Vec3, Vec3); 6] = [
        (Vec3::X, Vec3::Y, Vec3::Z),
        (-Vec3::X, Vec3::Y, -Vec3::Z),
        (Vec3::Y, Vec3::Z, Vec3::X),
        (-Vec3::Y, Vec3::Z, -Vec3::X),
        (Vec3::Z, Vec3::Y, -Vec3::X),
        (-Vec3::Z, Vec3::Y, Vec3::X),
    ];
    for (n, u, v) in faces {
        // u × v = n per avere il winding antiorario visto da fuori.
        let (u, v) = if u.cross(v).dot(n) > 0.0 { (u, v) } else { (v, u) };
        let base = m.positions.len() as u32;
        for (a, b) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            m.positions.push((n + u * a + v * b) * h);
            m.normals.push(n);
            m.uvs.push(Vec2::new((a + 1.0) * 0.5, (b + 1.0) * 0.5));
        }
        m.indices.push([base, base + 1, base + 2]);
        m.indices.push([base, base + 2, base + 3]);
    }
    m
}

/// Sfera UV con normali analitiche.
pub fn uv_sphere(radius: f32, segments: u32, rings: u32) -> Mesh {
    let mut m = Mesh {
        name: "Sfera".into(),
        ..Default::default()
    };
    for r in 0..=rings {
        let v = r as f32 / rings as f32;
        let theta = v * PI;
        for s in 0..=segments {
            let u = s as f32 / segments as f32;
            let phi = u * TAU;
            let n = Vec3::new(theta.sin() * phi.cos(), theta.cos(), -theta.sin() * phi.sin());
            m.positions.push(n * radius);
            m.normals.push(n);
            m.uvs.push(Vec2::new(u, v));
        }
    }
    let row = segments + 1;
    for r in 0..rings {
        for s in 0..segments {
            let a = r * row + s;
            let b = a + row;
            if r != 0 {
                m.indices.push([a, b, a + 1]);
            }
            if r != rings - 1 {
                m.indices.push([a + 1, b, b + 1]);
            }
        }
    }
    m
}

/// Piano orizzontale (normale +Y) centrato nell'origine.
pub fn plane(width: f32, depth: f32) -> Mesh {
    let (w, d) = (width * 0.5, depth * 0.5);
    Mesh {
        name: "Piano".into(),
        positions: vec![
            Vec3::new(-w, 0.0, d),
            Vec3::new(w, 0.0, d),
            Vec3::new(w, 0.0, -d),
            Vec3::new(-w, 0.0, -d),
        ],
        normals: vec![Vec3::Y; 4],
        uvs: vec![Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0), Vec2::new(1.0, 1.0), Vec2::new(0.0, 1.0)],
        indices: vec![[0, 1, 2], [0, 2, 3]],
    }
}

/// Cilindro chiuso lungo Y, base a y=0.
pub fn cylinder(radius: f32, height: f32, segments: u32) -> Mesh {
    let mut m = Mesh {
        name: "Cilindro".into(),
        ..Default::default()
    };
    // Mantello.
    for s in 0..=segments {
        let phi = s as f32 / segments as f32 * TAU;
        let n = Vec3::new(phi.cos(), 0.0, -phi.sin());
        for y in [0.0, height] {
            m.positions.push(n * radius + Vec3::Y * y);
            m.normals.push(n);
            m.uvs.push(Vec2::new(s as f32 / segments as f32, y / height));
        }
    }
    for s in 0..segments {
        let a = s * 2;
        m.indices.push([a, a + 2, a + 1]);
        m.indices.push([a + 1, a + 2, a + 3]);
    }
    // Tappi.
    for (y, n) in [(0.0, -Vec3::Y), (height, Vec3::Y)] {
        let c = m.positions.len() as u32;
        m.positions.push(Vec3::Y * y);
        m.normals.push(n);
        m.uvs.push(Vec2::splat(0.5));
        for s in 0..=segments {
            let phi = s as f32 / segments as f32 * TAU;
            m.positions.push(Vec3::new(phi.cos() * radius, y, -phi.sin() * radius));
            m.normals.push(n);
            m.uvs.push(Vec2::new(phi.cos() * 0.5 + 0.5, phi.sin() * 0.5 + 0.5));
        }
        for s in 0..segments {
            let (a, b) = (c + 1 + s, c + 2 + s);
            if n.y > 0.0 {
                m.indices.push([c, a, b]);
            } else {
                m.indices.push([c, b, a]);
            }
        }
    }
    m
}

/// Toro sul piano XZ.
pub fn torus(major: f32, minor: f32, seg_major: u32, seg_minor: u32) -> Mesh {
    let mut m = Mesh {
        name: "Toro".into(),
        ..Default::default()
    };
    for i in 0..=seg_major {
        let u = i as f32 / seg_major as f32 * TAU;
        let center = Vec3::new(u.cos(), 0.0, -u.sin()) * major;
        for j in 0..=seg_minor {
            let v = j as f32 / seg_minor as f32 * TAU;
            let n = Vec3::new(u.cos() * v.cos(), v.sin(), -u.sin() * v.cos());
            m.positions.push(center + n * minor);
            m.normals.push(n);
            m.uvs.push(Vec2::new(i as f32 / seg_major as f32, j as f32 / seg_minor as f32));
        }
    }
    let row = seg_minor + 1;
    for i in 0..seg_major {
        for j in 0..seg_minor {
            let a = i * row + j;
            let b = a + row;
            m.indices.push([a, b, a + 1]);
            m.indices.push([a + 1, b, b + 1]);
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifica che il winding sia coerente con le normali (facce orientate verso l'esterno).
    fn assert_outward(m: &Mesh) {
        for tri in &m.indices {
            let [a, b, c] = tri.map(|i| m.positions[i as usize]);
            let nf = (b - a).cross(c - a);
            if nf.length_squared() < 1e-12 {
                continue;
            }
            let ns = m.normals[tri[0] as usize] + m.normals[tri[1] as usize] + m.normals[tri[2] as usize];
            assert!(nf.dot(ns) > 0.0, "{} ha un triangolo invertito", m.name);
        }
        assert!(m.validate().invalid_indices == 0);
    }

    #[test]
    fn primitives_are_consistent() {
        assert_outward(&cuboid(Vec3::new(1.0, 2.0, 3.0)));
        assert_outward(&uv_sphere(1.0, 24, 12));
        assert_outward(&plane(2.0, 2.0));
        assert_outward(&cylinder(0.5, 1.0, 24));
        assert_outward(&torus(1.0, 0.3, 24, 12));
    }
}

/// Tazza (cilindro cavo aperto in alto) appoggiata su y = 0: raggio esterno, altezza,
/// spessore della parete e del fondo. Normali calcolate con spigoli netti a 40°.
pub fn cup(radius: f32, height: f32, wall: f32, segments: u32) -> Mesh {
    let mut m = Mesh {
        name: "Tazza".into(),
        ..Default::default()
    };
    let s = segments.max(8);
    let ri = (radius - wall).max(radius * 0.05);
    let ring = |r: f32, y: f32| -> Vec<Vec3> {
        (0..s)
            .map(|k| {
                let a = k as f32 / s as f32 * TAU;
                Vec3::new(r * a.cos(), y, r * a.sin())
            })
            .collect()
    };
    let quad_strip = |m: &mut Mesh, a: &[Vec3], b: &[Vec3]| {
        // Quadrilateri tra due anelli; il verso è quello dato dall'ordine (a, b).
        for k in 0..s as usize {
            let k1 = (k + 1) % s as usize;
            let base = m.positions.len() as u32;
            m.positions.extend([a[k], a[k1], b[k1], b[k]]);
            m.indices.push([base, base + 2, base + 1]);
            m.indices.push([base, base + 3, base + 2]);
        }
    };
    let floor = wall.min(height * 0.5);
    let (o0, o1) = (ring(radius, 0.0), ring(radius, height));
    let (i0, i1) = (ring(ri, floor), ring(ri, height));
    quad_strip(&mut m, &o0, &o1); // parete esterna
    quad_strip(&mut m, &i1, &i0); // parete interna
    quad_strip(&mut m, &o1, &i1); // bordo
                                  // Fondo: esterno (verso il basso) e interno (verso l'alto), a ventaglio.
    let fan = |m: &mut Mesh, r: &[Vec3], center: Vec3, up: bool| {
        let c = m.positions.len() as u32;
        m.positions.push(center);
        let first = m.positions.len() as u32;
        m.positions.extend_from_slice(r);
        for k in 0..s {
            let (a, b) = (first + k, first + (k + 1) % s);
            m.indices.push(if up { [c, b, a] } else { [c, a, b] });
        }
    };
    fan(&mut m, &o0, Vec3::ZERO, false);
    fan(&mut m, &i0, Vec3::new(0.0, floor, 0.0), true);
    m.compute_normals(40.0);
    m
}

#[cfg(test)]
mod cup_tests {
    use super::*;

    #[test]
    fn cup_is_closed_and_faces_outward() {
        let m = cup(0.05, 0.08, 0.004, 32);
        let b = m.bounds();
        assert!((b.max.y - 0.08).abs() < 1e-6 && b.min.y.abs() < 1e-6);
        // Le normali delle facce della parete esterna puntano in fuori.
        let mut out = 0;
        for t in &m.indices {
            let [a, b, c] = t.map(|i| m.positions[i as usize]);
            let n = (b - a).cross(c - a);
            let ctr = (a + b + c) / 3.0;
            let r = Vec3::new(ctr.x, 0.0, ctr.z).length();
            if (r - 0.05).abs() < 1e-3 && ctr.y > 0.001 && ctr.y < 0.079 && n.dot(Vec3::new(ctr.x, 0.0, ctr.z)) > 0.0 {
                out += 1;
            }
        }
        assert_eq!(out, 64);
        // Fondo interno verso l'alto, fondo esterno verso il basso.
        for t in &m.indices {
            let [a, b, c] = t.map(|i| m.positions[i as usize]);
            let n = (b - a).cross(c - a).normalize_or_zero();
            if a.y == b.y && b.y == c.y && n.y.abs() > 0.9 {
                if a.y.abs() < 1e-6 {
                    assert!(n.y < 0.0, "fondo esterno");
                } else if (a.y - 0.004).abs() < 1e-6 {
                    assert!(n.y > 0.0, "fondo interno");
                }
            }
        }
    }
}

/// Fondale da studio (cyclorama): pavimento che sale in una curva senza spigolo fino a
/// una parete verticale. Larghezza lungo X, pavimento da z = `depth` a z = 0, curva di
/// raggio `radius`, parete alta `height` a z = −`radius`. Base a y = 0.
pub fn backdrop(width: f32, depth: f32, height: f32, radius: f32, segments: u32) -> Mesh {
    let mut m = Mesh {
        name: "Fondale".into(),
        ..Default::default()
    };
    let r = radius.clamp(0.01, height.max(0.02));
    // Profilo nel piano (z, y): pavimento, quarto di cerchio, parete.
    let mut profile: Vec<(f32, f32)> = vec![(depth, 0.0), (0.0, 0.0)];
    let segs = segments.max(4);
    for k in 1..segs {
        let a = k as f32 / segs as f32 * std::f32::consts::FRAC_PI_2;
        profile.push((-r * a.sin(), r - r * a.cos()));
    }
    profile.push((-r, r));
    profile.push((-r, height.max(r)));
    let w = width * 0.5;
    // Lunghezza del profilo per le UV (la texture non si stira nella curva).
    let mut len = vec![0.0f32];
    for k in 1..profile.len() {
        let (a, b) = (profile[k - 1], profile[k]);
        len.push(len[k - 1] + ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt());
    }
    for (k, &(z, y)) in profile.iter().enumerate() {
        for x in [-w, w] {
            m.positions.push(Vec3::new(x, y, z));
            m.uvs
                .push(Vec2::new((x + w) / width.max(1e-6), len[k] / len[len.len() - 1].max(1e-6)));
        }
    }
    for k in 0..profile.len() as u32 - 1 {
        let (a, b, c, d) = (2 * k, 2 * k + 1, 2 * k + 2, 2 * k + 3);
        // Normali verso +Y sul pavimento e verso +Z sulla parete (lato della scena).
        m.indices.push([a, b, d]);
        m.indices.push([a, d, c]);
    }
    m.compute_normals(30.0);
    m
}

/// Fondale fotografico in tela: pavimento lungo `depth` davanti, raccordo di raggio `radius`
/// e telo che sale **inclinato all'indietro** di `tilt_deg` rispetto alla verticale (come un
/// fondale appeso al supporto e tirato verso la camera) fino all'altezza `height`.
/// Larghezza `width` lungo X, origine al piede del raccordo; normali verso la scena (+Y, +Z).
pub fn studio_sweep(width: f32, depth: f32, height: f32, radius: f32, tilt_deg: f32, segments: u32) -> Mesh {
    let mut m = Mesh {
        name: "Fondale fotografico".into(),
        ..Default::default()
    };
    let r = radius.clamp(0.01, height.max(0.02));
    // L'arco si ferma dove la tangente è inclinata di `tilt` rispetto alla verticale.
    let end = std::f32::consts::FRAC_PI_2 - tilt_deg.clamp(0.0, 45.0).to_radians();
    let mut profile: Vec<(f32, f32)> = vec![(depth, 0.0), (0.0, 0.0)];
    let segs = segments.max(4);
    for k in 1..=segs {
        let a = k as f32 / segs as f32 * end;
        profile.push((-r * a.sin(), r - r * a.cos()));
    }
    // Tratto dritto lungo la tangente di fine arco, fino all'altezza.
    let (z0, y0) = *profile.last().expect("profilo");
    let dir = (-end.cos(), end.sin());
    if height > y0 {
        let t = (height - y0) / dir.1.max(1e-4);
        profile.push((z0 + dir.0 * t, height));
    }
    let w = width * 0.5;
    let mut len = vec![0.0f32];
    for k in 1..profile.len() {
        let (a, b) = (profile[k - 1], profile[k]);
        len.push(len[k - 1] + ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt());
    }
    for (k, &(z, y)) in profile.iter().enumerate() {
        for x in [-w, w] {
            m.positions.push(Vec3::new(x, y, z));
            m.uvs
                .push(Vec2::new((x + w) / width.max(1e-6), len[k] / len[len.len() - 1].max(1e-6)));
        }
    }
    for k in 0..profile.len() as u32 - 1 {
        let (a, b, c, d) = (2 * k, 2 * k + 1, 2 * k + 2, 2 * k + 3);
        m.indices.push([a, b, d]);
        m.indices.push([a, d, c]);
    }
    m.compute_normals(30.0);
    m
}

#[cfg(test)]
mod backdrop_tests {
    use super::*;

    #[test]
    fn studio_sweep_leans_back_and_faces_the_scene() {
        let (h, r, tilt) = (2.5, 0.5, 12.0f32);
        let m = studio_sweep(4.0, 3.0, h, r, tilt, 16);
        let b = m.bounds();
        assert!((b.max.y - h).abs() < 1e-4 && b.min.y.abs() < 1e-6, "{b:?}");
        // La sommità del telo è più indietro del raccordo: inclinato all'indietro.
        let top_z = m
            .positions
            .iter()
            .filter(|p| (p.y - h).abs() < 1e-4)
            .map(|p| p.z)
            .fold(f32::MAX, f32::min);
        let expected = -r * (90.0 - tilt).to_radians().sin() - (h - r * (1.0 - (90.0 - tilt).to_radians().cos())) * tilt.to_radians().tan();
        assert!((top_z - expected).abs() < 1e-3, "{top_z} invece di {expected}");
        for t in &m.indices {
            let [a, b, c] = t.map(|i| m.positions[i as usize]);
            let n = (b - a).cross(c - a).normalize();
            // Tutto il telo guarda la scena: in su o verso +Z, mai di spalle.
            assert!(n.y > -1e-3 && n.z > -1e-3, "{n}");
            let ctr = (a + b + c) / 3.0;
            if ctr.y > r + 0.05 {
                // Tratto inclinato: normale inclinata verso l'alto di `tilt`.
                assert!((n.y - tilt.to_radians().sin()).abs() < 0.02, "{n}");
            }
        }
    }

    #[test]
    fn backdrop_faces_the_scene() {
        let m = backdrop(4.0, 3.0, 2.5, 0.6, 12);
        let b = m.bounds();
        assert!((b.max.y - 2.5).abs() < 1e-5 && b.min.y.abs() < 1e-6);
        for t in &m.indices {
            let [a, b, c] = t.map(|i| m.positions[i as usize]);
            let n = (b - a).cross(c - a).normalize();
            let ctr = (a + b + c) / 3.0;
            // Pavimento: normale in su; parete: verso +Z (dove sta la scena).
            if ctr.y < 1e-4 {
                assert!(n.y > 0.99, "pavimento {n}");
            } else if ctr.y > 0.61 {
                assert!(n.z > 0.99, "parete {n}");
            } else {
                assert!(n.y > -1e-3 && n.z > -1e-3, "curva {n}");
            }
        }
    }
}
