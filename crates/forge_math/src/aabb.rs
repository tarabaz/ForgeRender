use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};

/// Bounding box allineata agli assi.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Default for Aabb {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl Aabb {
    /// Box vuota: l'unione con qualsiasi punto restituisce quel punto.
    pub const EMPTY: Aabb = Aabb {
        min: Vec3::splat(f32::INFINITY),
        max: Vec3::splat(f32::NEG_INFINITY),
    };

    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn from_points<'a>(points: impl IntoIterator<Item = &'a Vec3>) -> Self {
        let mut b = Self::EMPTY;
        for p in points {
            b.grow_point(*p);
        }
        b
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.min.x > self.max.x || self.min.y > self.max.y || self.min.z > self.max.z
    }

    #[inline]
    pub fn grow_point(&mut self, p: Vec3) {
        self.min = self.min.min(p);
        self.max = self.max.max(p);
    }

    #[inline]
    pub fn grow(&mut self, other: &Aabb) {
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
    }

    #[inline]
    pub fn union(mut self, other: &Aabb) -> Aabb {
        self.grow(other);
        self
    }

    #[inline]
    pub fn extent(&self) -> Vec3 {
        if self.is_empty() {
            Vec3::ZERO
        } else {
            self.max - self.min
        }
    }

    #[inline]
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Metà dell'area superficiale (sufficiente per la SAH, che usa rapporti).
    #[inline]
    pub fn half_area(&self) -> f32 {
        let e = self.extent();
        e.x * e.y + e.y * e.z + e.z * e.x
    }

    #[inline]
    pub fn largest_axis(&self) -> usize {
        let e = self.extent();
        if e.x >= e.y && e.x >= e.z {
            0
        } else if e.y >= e.z {
            1
        } else {
            2
        }
    }

    /// Raggio della sfera circoscritta.
    pub fn radius(&self) -> f32 {
        self.extent().length() * 0.5
    }

    /// Box degli 8 vertici trasformati.
    pub fn transformed(&self, m: &Mat4) -> Aabb {
        if self.is_empty() {
            return *self;
        }
        let mut out = Aabb::EMPTY;
        for i in 0..8 {
            let p = Vec3::new(
                if i & 1 == 0 { self.min.x } else { self.max.x },
                if i & 2 == 0 { self.min.y } else { self.max.y },
                if i & 4 == 0 { self.min.z } else { self.max.z },
            );
            out.grow_point(m.transform_point3(p));
        }
        out
    }

    /// Test raggio-box (slab test). `inv_dir` è il reciproco della direzione.
    /// Restituisce la distanza di ingresso se l'intervallo `[t_min, t_max]` interseca la box.
    #[inline(always)]
    pub fn intersect(&self, origin: Vec3, inv_dir: Vec3, t_min: f32, t_max: f32) -> Option<f32> {
        let t0 = (self.min - origin) * inv_dir;
        let t1 = (self.max - origin) * inv_dir;
        let tn = t0.min(t1);
        let tf = t0.max(t1);
        let enter = tn.max_element().max(t_min);
        let exit = tf.min_element().min(t_max);
        if enter <= exit {
            Some(enter)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_union() {
        let mut b = Aabb::EMPTY;
        assert!(b.is_empty());
        b.grow_point(Vec3::ONE);
        assert_eq!(b.min, Vec3::ONE);
        assert_eq!(b.max, Vec3::ONE);
    }

    #[test]
    fn slab_hit_and_miss() {
        let b = Aabb::new(Vec3::splat(-1.0), Vec3::splat(1.0));
        let o = Vec3::new(0.0, 0.0, -5.0);
        let d = Vec3::Z;
        assert_eq!(b.intersect(o, d.recip(), 0.0, 100.0), Some(4.0));
        let d2 = Vec3::new(0.0, 1.0, 0.0);
        assert!(b.intersect(o, d2.recip(), 0.0, 100.0).is_none());
    }
}
