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
