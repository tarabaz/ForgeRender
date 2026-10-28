//! Griglie regolari a celle cubiche con campionamento trilineare.

use forge_math::Vec3;

/// Geometria di una griglia: origine, numero di celle per asse e lato della cella.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridSpec {
    pub min: Vec3,
    pub res: [usize; 3],
    pub cell: f32,
}

impl GridSpec {
    /// Griglia che copre [min, max] con `longest` celle sul lato più lungo.
    pub fn covering(min: Vec3, max: Vec3, longest: usize) -> Self {
        let ext = (max - min).max(Vec3::splat(1e-4));
        let cell = ext.max_element() / longest.max(1) as f32;
        let res = [0, 1, 2].map(|a| ((ext[a] / cell).ceil() as usize).max(2));
        Self { min, res, cell }
    }

    pub fn len(&self) -> usize {
        self.res[0] * self.res[1] * self.res[2]
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn max(&self) -> Vec3 {
        self.min + Vec3::new(self.res[0] as f32, self.res[1] as f32, self.res[2] as f32) * self.cell
    }

    /// Indice lineare: x più veloce, poi z, poi y (strati orizzontali contigui).
    #[inline]
    pub fn index(&self, x: usize, y: usize, z: usize) -> usize {
        (y * self.res[2] + z) * self.res[0] + x
    }

    #[inline]
    pub fn coords(&self, i: usize) -> (usize, usize, usize) {
        let x = i % self.res[0];
        let r = i / self.res[0];
        (x, r / self.res[2], r % self.res[2])
    }

    #[inline]
    pub fn center(&self, x: usize, y: usize, z: usize) -> Vec3 {
        self.min + (Vec3::new(x as f32, y as f32, z as f32) + Vec3::splat(0.5)) * self.cell
    }

    /// Cella che contiene `p` (None se fuori).
    pub fn cell_of(&self, p: Vec3) -> Option<(usize, usize, usize)> {
        let q = (p - self.min) / self.cell;
        if q.x < 0.0 || q.y < 0.0 || q.z < 0.0 {
            return None;
        }
        let (x, y, z) = (q.x as usize, q.y as usize, q.z as usize);
        (x < self.res[0] && y < self.res[1] && z < self.res[2]).then_some((x, y, z))
    }

    /// Campionamento trilineare (valori ai centri delle celle, bordo esteso; 0 fuori dalla griglia).
    pub fn sample(&self, data: &[f32], p: Vec3) -> f32 {
        let q = (p - self.min) / self.cell;
        let size = Vec3::new(self.res[0] as f32, self.res[1] as f32, self.res[2] as f32);
        if q.x < 0.0 || q.y < 0.0 || q.z < 0.0 || q.x > size.x || q.y > size.y || q.z > size.z {
            return 0.0;
        }
        let q = q - Vec3::splat(0.5);
        let f = q.floor();
        let t = q - f;
        let i0 = [f.x as i64, f.y as i64, f.z as i64];
        let at = |dx: i64, dy: i64, dz: i64| {
            let c = |v: i64, a: usize| v.clamp(0, self.res[a] as i64 - 1) as usize;
            data[self.index(c(i0[0] + dx, 0), c(i0[1] + dy, 1), c(i0[2] + dz, 2))]
        };
        let l = |a: f32, b: f32, t: f32| a + (b - a) * t;
        let x00 = l(at(0, 0, 0), at(1, 0, 0), t.x);
        let x10 = l(at(0, 1, 0), at(1, 1, 0), t.x);
        let x01 = l(at(0, 0, 1), at(1, 0, 1), t.x);
        let x11 = l(at(0, 1, 1), at(1, 1, 1), t.x);
        l(l(x00, x10, t.y), l(x01, x11, t.y), t.z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_roundtrip_and_trilinear() {
        let g = GridSpec::covering(Vec3::ZERO, Vec3::new(2.0, 1.0, 1.0), 4);
        assert_eq!(g.res, [4, 2, 2]);
        for i in 0..g.len() {
            let (x, y, z) = g.coords(i);
            assert_eq!(g.index(x, y, z), i);
        }
        // Campo lineare in x: il trilineare lo riproduce esattamente all'interno.
        let data: Vec<f32> = (0..g.len()).map(|i| g.center(g.coords(i).0, 0, 0).x).collect();
        for x in [0.3f32, 0.9, 1.33, 1.7] {
            let v = g.sample(&data, Vec3::new(x, 0.5, 0.5));
            assert!((v - x).abs() < 1e-5, "{x}: {v}");
        }
        assert_eq!(g.sample(&data, Vec3::new(-0.1, 0.5, 0.5)), 0.0);
    }
}
