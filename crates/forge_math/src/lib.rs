//! Primitive matematiche condivise dai crate di ForgeRender.
//!
//! Tutto ciò che sta qui è indipendente dal backend: lo stesso layout di
//! [`Aabb`] e gli stessi algoritmi di campionamento verranno replicati negli
//! shader WGSL del backend GPU.

pub use glam;
pub use glam::{Mat3, Mat4, Quat, Vec2, Vec3, Vec3A, Vec4};

mod aabb;
mod onb;
mod ray;
mod rng;
pub mod sampling;

pub use aabb::Aabb;
pub use onb::Onb;
pub use ray::{offset_ray_origin, Ray};
pub use rng::Pcg32;

/// Luminanza Rec.709 di un colore lineare.
#[inline]
pub fn luminance(c: Vec3) -> f32 {
    c.dot(Vec3::new(0.2126, 0.7152, 0.0722))
}
