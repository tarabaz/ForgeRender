use glam::Vec3;

#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub origin: Vec3,
    pub dir: Vec3,
}

impl Ray {
    #[inline]
    pub fn new(origin: Vec3, dir: Vec3) -> Self {
        Self { origin, dir }
    }

    #[inline]
    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.dir * t
    }
}

/// Sposta l'origine di un raggio secondario fuori dalla superficie per evitare
/// l'auto-intersezione ("shadow acne"), lungo la normale geometrica `ng`
/// orientata verso il lato in cui parte il raggio.
///
/// Implementazione di Wächter & Binder, "A Fast and Robust Method for Avoiding
/// Self-Intersection" (Ray Tracing Gems, cap. 6): l'offset è espresso in ULP
/// del float, quindi è corretto sia per scene millimetriche sia chilometriche.
#[inline]
pub fn offset_ray_origin(p: Vec3, ng: Vec3) -> Vec3 {
    const ORIGIN: f32 = 1.0 / 32.0;
    const FLOAT_SCALE: f32 = 1.0 / 65536.0;
    const INT_SCALE: f32 = 256.0;

    let of_i = [(INT_SCALE * ng.x) as i32, (INT_SCALE * ng.y) as i32, (INT_SCALE * ng.z) as i32];
    let pa = [p.x, p.y, p.z];
    let na = [ng.x, ng.y, ng.z];
    let mut out = [0.0f32; 3];
    for i in 0..3 {
        let bits = pa[i].to_bits() as i32;
        let delta = if pa[i] < 0.0 { -of_i[i] } else { of_i[i] };
        let p_i = f32::from_bits(bits.wrapping_add(delta) as u32);
        out[i] = if pa[i].abs() < ORIGIN { pa[i] + FLOAT_SCALE * na[i] } else { p_i };
    }
    Vec3::from_array(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offset_moves_along_normal() {
        for p in [Vec3::ZERO, Vec3::splat(1000.0), Vec3::new(-0.5, 3.0, -2000.0)] {
            let n = Vec3::new(0.0, 1.0, 0.0);
            let q = offset_ray_origin(p, n);
            assert!(q.y > p.y, "{p:?} -> {q:?}");
            assert_eq!(q.x, p.x);
            let q2 = offset_ray_origin(p, -n);
            assert!(q2.y < p.y);
        }
    }
}
