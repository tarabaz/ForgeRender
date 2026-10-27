use glam::Vec3;

/// Base ortonormale costruita attorno a una normale (Duff et al. 2017,
/// "Building an Orthonormal Basis, Revisited"): nessuna singolarità.
#[derive(Clone, Copy, Debug)]
pub struct Onb {
    pub t: Vec3,
    pub b: Vec3,
    pub n: Vec3,
}

impl Onb {
    pub fn from_normal(n: Vec3) -> Self {
        let sign = 1.0f32.copysign(n.z);
        let a = -1.0 / (sign + n.z);
        let b = n.x * n.y * a;
        let t = Vec3::new(1.0 + sign * n.x * n.x * a, sign * b, -sign * n.x);
        let bt = Vec3::new(b, sign + n.y * n.y * a, -n.y);
        Self { t, b: bt, n }
    }

    /// Da spazio locale (z = normale) a spazio mondo.
    #[inline]
    pub fn to_world(&self, v: Vec3) -> Vec3 {
        self.t * v.x + self.b * v.y + self.n * v.z
    }

    /// Da spazio mondo a spazio locale.
    #[inline]
    pub fn to_local(&self, v: Vec3) -> Vec3 {
        Vec3::new(v.dot(self.t), v.dot(self.b), v.dot(self.n))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orthonormal() {
        for n in [Vec3::Z, -Vec3::Z, Vec3::X, Vec3::new(0.3, -0.4, 0.866).normalize()] {
            let o = Onb::from_normal(n);
            assert!((o.t.length() - 1.0).abs() < 1e-5);
            assert!((o.b.length() - 1.0).abs() < 1e-5);
            assert!(o.t.dot(o.b).abs() < 1e-5);
            assert!(o.t.dot(o.n).abs() < 1e-5);
            assert!(o.b.dot(o.n).abs() < 1e-5);
            let v = Vec3::new(0.2, 0.5, 0.7);
            assert!((o.to_local(o.to_world(v)) - v).length() < 1e-5);
        }
    }
}
