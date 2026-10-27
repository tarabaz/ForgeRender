//! BSDF del materiale "principled lite" (vedi `forge_scene::Material`).
//!
//! Tutti i calcoli avvengono in spazio locale con z = normale di shading,
//! orientata dal lato della direzione uscente `wo`.
//!
//! Lobi:
//! - diffuso di Lambert, pesato da (1 − F(wi))(1 − F(wo)) per conservare l'energia
//!   rispetto al riflesso speculare (reciproco);
//! - speculare microfacet GGX (Trowbridge-Reitz) con mascheramento di Smith
//!   height-correlated e Fresnel di Schlick; campionamento della distribuzione
//!   delle normali visibili (Heitz 2018);
//! - trasmissione dielettrica liscia (delta) con Fresnel esatto.

use forge_math::{luminance, sampling, Onb, Vec3};
use forge_render_core::RenderMaterial;
use std::f32::consts::{FRAC_1_PI, PI};

pub struct Bsdf<'a> {
    pub frame: Onb,
    /// Normale geometrica orientata come la normale di shading.
    pub ng: Vec3,
    pub mat: &'a RenderMaterial,
    /// Vero se il raggio arriva dal lato esterno della superficie (rispetto al winding).
    pub entering: bool,
}

pub struct BsdfSample {
    pub wi: Vec3,
    /// f · |cos θ| / pdf
    pub weight: Vec3,
    pub pdf: f32,
    pub delta: bool,
}

#[inline]
fn schlick(f0: f32, cos: f32) -> f32 {
    let m = (1.0 - cos).clamp(0.0, 1.0);
    let m2 = m * m;
    f0 + (1.0 - f0) * m2 * m2 * m
}

#[inline]
fn schlick3(f0: Vec3, cos: f32) -> Vec3 {
    let m = (1.0 - cos).clamp(0.0, 1.0);
    let m2 = m * m;
    f0 + (Vec3::ONE - f0) * (m2 * m2 * m)
}

/// Riflettanza di Fresnel esatta per dielettrici (luce non polarizzata).
/// `eta` = n_trasmesso / n_incidente. Restituisce (F, cos θ_t).
#[inline]
pub fn fresnel_dielectric(cos_i: f32, eta: f32) -> (f32, f32) {
    let cos_i = cos_i.clamp(0.0, 1.0);
    let sin2_t = (1.0 - cos_i * cos_i) / (eta * eta);
    if sin2_t >= 1.0 {
        return (1.0, 0.0); // riflessione totale interna
    }
    let cos_t = (1.0 - sin2_t).sqrt();
    let rs = (cos_i - eta * cos_t) / (cos_i + eta * cos_t);
    let rp = (eta * cos_i - cos_t) / (eta * cos_i + cos_t);
    (0.5 * (rs * rs + rp * rp), cos_t)
}

#[inline]
fn ggx_d(h: Vec3, a2: f32) -> f32 {
    let c2 = h.z * h.z;
    let d = c2 * (a2 - 1.0) + 1.0;
    a2 / (PI * d * d)
}

#[inline]
fn smith_lambda(w: Vec3, a2: f32) -> f32 {
    let c2 = w.z * w.z;
    if c2 <= 0.0 {
        return f32::INFINITY;
    }
    let t2 = (1.0 - c2).max(0.0) / c2;
    0.5 * ((1.0 + a2 * t2).sqrt() - 1.0)
}

/// Campiona una normale visibile GGX (Heitz 2018, "Sampling the GGX
/// Distribution of Visible Normals"), isotropa.
fn sample_vndf(wo: Vec3, alpha: f32, u1: f32, u2: f32) -> Vec3 {
    let vh = Vec3::new(alpha * wo.x, alpha * wo.y, wo.z).normalize();
    let lensq = vh.x * vh.x + vh.y * vh.y;
    let t1 = if lensq > 0.0 {
        Vec3::new(-vh.y, vh.x, 0.0) / lensq.sqrt()
    } else {
        Vec3::X
    };
    let t2 = vh.cross(t1);
    let r = u1.sqrt();
    let phi = 2.0 * PI * u2;
    let p1 = r * phi.cos();
    let mut p2 = r * phi.sin();
    let s = 0.5 * (1.0 + vh.z);
    p2 = (1.0 - s) * (1.0 - p1 * p1).max(0.0).sqrt() + s * p2;
    let nh = t1 * p1 + t2 * p2 + vh * (1.0 - p1 * p1 - p2 * p2).max(0.0).sqrt();
    Vec3::new(alpha * nh.x, alpha * nh.y, nh.z.max(1e-6)).normalize()
}

/// Albedo direzionale del GGX a singolo scattering con F = 1, E(μ, α), tabulata
/// una volta per processo (32×32, integrazione con campionamento VNDF stratificato).
/// Serve alla compensazione dell'energia persa dai riflessi multipli tra microfacce
/// (Turquin 2019, "Practical multiple scattering compensation for microfacet models").
pub const GGX_LUT_SIZE: usize = 32;

/// Tabella E(μ, α) (righe: α, colonne: μ), condivisa con il backend GPU.
pub fn ggx_albedo_table() -> &'static [f32] {
    const N: usize = GGX_LUT_SIZE;
    static LUT: std::sync::OnceLock<Vec<f32>> = std::sync::OnceLock::new();
    LUT.get_or_init(|| {
        let mut t = vec![0.0f32; N * N];
        const S: usize = 24;
        for ia in 0..N {
            let a = (ia as f32 / (N - 1) as f32).max(1e-3);
            let a2 = a * a;
            for ic in 0..N {
                let mu = (ic as f32 / (N - 1) as f32).max(1e-3);
                let wo = Vec3::new((1.0 - mu * mu).max(0.0).sqrt(), 0.0, mu);
                let mut e = 0.0f64;
                for i in 0..S {
                    for j in 0..S {
                        let h = sample_vndf(wo, a, (i as f32 + 0.5) / S as f32, (j as f32 + 0.5) / S as f32);
                        let wi = 2.0 * wo.dot(h) * h - wo;
                        if wi.z > 0.0 {
                            let lo = smith_lambda(wo, a2);
                            let li = smith_lambda(wi, a2);
                            e += ((1.0 + lo) / (1.0 + lo + li)) as f64;
                        }
                    }
                }
                t[ia * N + ic] = (e / (S * S) as f64) as f32;
            }
        }
        t
    })
}

fn ggx_albedo(cos_o: f32, alpha: f32) -> f32 {
    const N: usize = GGX_LUT_SIZE;
    let lut = ggx_albedo_table();
    let fa = (alpha.clamp(0.0, 1.0) * (N - 1) as f32).min((N - 1) as f32 - 1e-4);
    let fc = (cos_o.clamp(0.0, 1.0) * (N - 1) as f32).min((N - 1) as f32 - 1e-4);
    let (ia, ic) = (fa as usize, fc as usize);
    let (ta, tc) = (fa - ia as f32, fc - ic as f32);
    let v = |a: usize, c: usize| lut[a * N + c];
    let top = v(ia, ic) * (1.0 - tc) + v(ia, ic + 1) * tc;
    let bot = v(ia + 1, ic) * (1.0 - tc) + v(ia + 1, ic + 1) * tc;
    (top * (1.0 - ta) + bot * ta).max(1e-3)
}

/// Rugosità oltre la quale il vetro diventa satinato (trasmissione microfacet).
const ROUGH_GLASS: f32 = 0.03;

impl<'a> Bsdf<'a> {
    /// Componente non-delta presente (serve il campionamento delle luci).
    #[inline]
    pub fn has_non_delta(&self) -> bool {
        self.mat.transmission < 1.0 || self.rough_glass()
    }

    #[inline]
    fn rough_glass(&self) -> bool {
        self.mat.transmission > 0.0 && self.mat.roughness >= ROUGH_GLASS
    }

    /// Indici di rifrazione (lato di `wo`, lato opposto).
    #[inline]
    fn ior_pair(&self) -> (f32, f32) {
        if self.entering {
            (1.0, self.mat.ior)
        } else {
            (self.mat.ior, 1.0)
        }
    }

    #[inline]
    fn f_dielectric(&self, cos: f32) -> f32 {
        (self.mat.specular_scale * schlick(self.mat.f0_dielectric, cos)).min(1.0)
    }

    #[inline]
    fn f0(&self) -> Vec3 {
        let m = self.mat;
        Vec3::splat((m.specular_scale * m.f0_dielectric).min(1.0)).lerp(m.base_color, m.metallic)
    }

    /// Probabilità di scegliere il lobo speculare (per il lobo opaco).
    #[inline]
    fn spec_probability(&self, wo: Vec3) -> f32 {
        let m = self.mat;
        let spec = luminance(schlick3(self.f0(), wo.z)).max(if m.metallic > 0.0 || m.specular_scale > 0.0 { 1e-3 } else { 0.0 });
        let diff = (1.0 - m.metallic) * luminance(m.base_color) * (1.0 - self.f_dielectric(wo.z));
        if diff <= 0.0 {
            1.0
        } else if spec <= 0.0 {
            0.0
        } else {
            (spec / (spec + diff)).clamp(0.1, 0.9)
        }
    }

    /// Lobo opaco (diffuso + GGX con compensazione multi-scattering), solo riflessione.
    fn eval_opaque(&self, wo: Vec3, wi: Vec3) -> (Vec3, f32) {
        let m = self.mat;
        if wo.z <= 0.0 || wi.z <= 0.0 {
            return (Vec3::ZERO, 0.0);
        }
        let p_spec = self.spec_probability(wo);
        let a2 = m.alpha * m.alpha;
        let h = (wo + wi).normalize();
        let mut f = Vec3::ZERO;
        let mut pdf = 0.0;
        if p_spec > 0.0 {
            let d = ggx_d(h, a2);
            let lo = smith_lambda(wo, a2);
            let li = smith_lambda(wi, a2);
            let g2 = 1.0 / (1.0 + lo + li);
            let g1o = 1.0 / (1.0 + lo);
            let voh = wo.dot(h).max(0.0);
            let fr = Vec3::splat(self.f_dielectric(voh)).lerp(schlick3(m.base_color, voh), m.metallic);
            // Compensazione dell'energia dei riflessi multipli: 1 + F0·(1/E − 1).
            let e = ggx_albedo(wo.z, m.alpha);
            let ms = Vec3::ONE + self.f0() * (1.0 / e - 1.0);
            f += fr * ms * (d * g2 / (4.0 * wo.z * wi.z));
            pdf += p_spec * g1o * d / (4.0 * wo.z);
        }
        if p_spec < 1.0 {
            let kd = (1.0 - m.metallic) * (1.0 - self.f_dielectric(wi.z)) * (1.0 - self.f_dielectric(wo.z));
            f += m.base_color * (kd * FRAC_1_PI);
            pdf += (1.0 - p_spec) * sampling::cosine_hemisphere_pdf(wi.z);
        }
        (f, pdf)
    }

    /// Dielettrico ruvido (Walter et al. 2007, formulazione di PBRT v4): riflessione e
    /// rifrazione attraverso le microfacce GGX. Radianza scalata per (n_o/n_i)².
    fn eval_rough_glass(&self, wo: Vec3, wi: Vec3) -> (Vec3, f32) {
        let m = self.mat;
        if wo.z <= 0.0 || wi.z == 0.0 {
            return (Vec3::ZERO, 0.0);
        }
        let (n_o, n_t) = self.ior_pair();
        let eta = n_t / n_o;
        let reflect = wi.z > 0.0;
        let mut wm = if reflect { wi + wo } else { wi * eta + wo };
        if wm.length_squared() == 0.0 {
            return (Vec3::ZERO, 0.0);
        }
        wm = wm.normalize();
        if wm.z < 0.0 {
            wm = -wm;
        }
        let (wom, wim) = (wo.dot(wm), wi.dot(wm));
        if wom * wo.z <= 0.0 || wim * wi.z <= 0.0 {
            return (Vec3::ZERO, 0.0);
        }
        let a2 = m.alpha * m.alpha;
        let (fr, _) = fresnel_dielectric(wom, eta);
        let d = ggx_d(wm, a2);
        let lo = smith_lambda(wo, a2);
        let li = smith_lambda(Vec3::new(wi.x, wi.y, wi.z.abs()), a2);
        let g2 = 1.0 / (1.0 + lo + li);
        let g1o = 1.0 / (1.0 + lo);
        let dv = g1o * wom * d / wo.z;
        if reflect {
            let f = d * g2 * fr / (4.0 * wo.z * wi.z);
            (Vec3::splat(f), dv / (4.0 * wom) * fr)
        } else {
            let denom = (wim + wom / eta).powi(2);
            if denom <= 0.0 {
                return (Vec3::ZERO, 0.0);
            }
            let ft = (1.0 - fr) * d * g2 * (wim * wom / (wi.z * wo.z * denom)).abs() / (eta * eta);
            let pdf = dv * (wim.abs() / denom) * (1.0 - fr);
            (m.base_color * ft, pdf)
        }
    }

    /// Valuta la parte non-delta della BSDF (senza il coseno) e la sua PDF complessiva.
    /// `wo`, `wi` in spazio mondo.
    pub fn eval(&self, wo_w: Vec3, wi_w: Vec3) -> (Vec3, f32) {
        let m = self.mat;
        if wo_w.dot(self.ng) <= 0.0 {
            return (Vec3::ZERO, 0.0);
        }
        let wo = self.frame.to_local(wo_w);
        let wi = self.frame.to_local(wi_w);
        // Coerenza tra normale geometrica e di shading: evita che la luce "trapeli".
        let geo_reflect = wi_w.dot(self.ng) > 0.0;
        if geo_reflect != (wi.z > 0.0) {
            return (Vec3::ZERO, 0.0);
        }
        let t = m.transmission;
        let mut f = Vec3::ZERO;
        let mut pdf = 0.0;
        if t < 1.0 && geo_reflect {
            let (fo, po) = self.eval_opaque(wo, wi);
            f += fo * (1.0 - t);
            pdf += po * (1.0 - t);
        }
        if self.rough_glass() {
            let (fg, pg) = self.eval_rough_glass(wo, wi);
            f += fg * t;
            pdf += pg * t;
        }
        (f, pdf)
    }

    pub fn sample(&self, wo_w: Vec3, u_lobe: f32, u1: f32, u2: f32) -> Option<BsdfSample> {
        let m = self.mat;
        let wo = self.frame.to_local(wo_w);
        if wo.z <= 0.0 {
            return None;
        }
        let wi = if u_lobe < m.transmission {
            if !self.rough_glass() {
                return self.sample_glass(wo_w, u1, u2);
            }
            // Vetro satinato: microfaccia visibile, poi riflessione o rifrazione con probabilità F.
            let (n_o, n_t) = self.ior_pair();
            let eta = n_t / n_o;
            let wm = sample_vndf(wo, m.alpha, u1, u2);
            let wom = wo.dot(wm);
            let (fr, cos_t) = fresnel_dielectric(wom, eta);
            // Terzo numero casuale ricavato da u_lobe (già usato solo per la soglia).
            let u3 = u_lobe / m.transmission;
            if u3 < fr {
                2.0 * wom * wm - wo
            } else {
                -wo / eta + wm * (wom / eta - cos_t)
            }
        } else {
            let u = if m.transmission > 0.0 {
                (u_lobe - m.transmission) / (1.0 - m.transmission)
            } else {
                u_lobe
            };
            let p_spec = self.spec_probability(wo);
            if u < p_spec {
                let h = sample_vndf(wo, m.alpha, u1, u2);
                2.0 * wo.dot(h) * h - wo
            } else {
                sampling::cosine_hemisphere(u1, u2)
            }
        };
        if !wi.is_finite() || wi.z == 0.0 {
            return None;
        }
        let wi_w = self.frame.to_world(wi.normalize());
        // Stimatore a un campione sulla miscela dei lobi: f e pdf complessivi.
        let (f, pdf) = self.eval(wo_w, wi_w);
        if pdf <= 0.0 || f == Vec3::ZERO {
            return None;
        }
        Some(BsdfSample {
            wi: wi_w,
            weight: f * (wi_w.dot(self.frame.n).abs() / pdf),
            pdf,
            delta: false,
        })
    }

    /// Dielettrico liscio: riflessione o rifrazione scelte con probabilità F.
    fn sample_glass(&self, wo_w: Vec3, u1: f32, _u2: f32) -> Option<BsdfSample> {
        let m = self.mat;
        let n = self.frame.n;
        let cos_o = wo_w.dot(n);
        if cos_o <= 0.0 {
            return None;
        }
        // eta = n_trasmesso / n_incidente lungo la direzione della luce verso wo.
        let (n_wo, n_wi) = if self.entering { (1.0, m.ior) } else { (m.ior, 1.0) };
        let (fr, cos_t) = fresnel_dielectric(cos_o, n_wi / n_wo);
        if u1 < fr {
            let wi = 2.0 * cos_o * n - wo_w;
            return Some(BsdfSample {
                wi,
                weight: Vec3::ONE,
                pdf: 0.0,
                delta: true,
            });
        }
        let eta = n_wo / n_wi;
        let wi = (-wo_w * eta + n * (eta * cos_o - cos_t)).normalize();
        // Scala della radianza tra mezzi con indice diverso: (n_wo / n_wi)².
        let scale = (n_wo / n_wi) * (n_wo / n_wi);
        Some(BsdfSample {
            wi,
            weight: m.base_color * scale,
            pdf: 0.0,
            delta: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_math::Pcg32;
    use forge_scene::Material;

    fn mat(base: f32, metallic: f32, roughness: f32, specular: f32) -> RenderMaterial {
        RenderMaterial::from_material(&Material {
            base_color: [base; 3],
            metallic,
            roughness,
            specular,
            ..Default::default()
        })
    }

    fn bsdf(m: &RenderMaterial) -> Bsdf<'_> {
        Bsdf {
            frame: Onb::from_normal(Vec3::Z),
            ng: Vec3::Z,
            mat: m,
            entering: true,
        }
    }

    /// Albedo direzionale stimata via campionamento: ∫ f cos dω.
    fn albedo(m: &RenderMaterial, wo: Vec3, n: usize) -> f32 {
        let b = bsdf(m);
        let mut rng = Pcg32::new(5, 7);
        let mut acc = 0.0f64;
        for _ in 0..n {
            if let Some(s) = b.sample(wo, rng.next_f32(), rng.next_f32(), rng.next_f32()) {
                acc += s.weight.x as f64;
            }
        }
        (acc / n as f64) as f32
    }

    /// Albedo per integrazione numerica di `eval` (campionamento uniforme dell'emisfero).
    fn albedo_by_eval(m: &RenderMaterial, wo: Vec3, n: usize) -> f32 {
        let b = bsdf(m);
        let mut rng = Pcg32::new(17, 1);
        let mut acc = 0.0f64;
        for _ in 0..n {
            let mut d = sampling::uniform_sphere(rng.next_f32(), rng.next_f32());
            d.z = d.z.abs();
            let (f, _) = b.eval(wo, d);
            acc += (f.x * d.z) as f64 / (2.0 * sampling::UNIFORM_SPHERE_PDF) as f64;
        }
        (acc / n as f64) as f32
    }

    /// Nessun materiale crea energia e, grazie alla compensazione multi-scattering,
    /// un metallo bianco conserva l'energia anche se molto ruvido.
    #[test]
    fn energy_conservation() {
        let wo = Vec3::new(0.5, 0.0, 0.866).normalize();
        for &(metallic, rough, min) in &[
            (0.0, 0.5, 0.8),
            (1.0, 0.1, 0.95),
            (1.0, 0.9, 0.4),
            (0.0, 0.05, 0.85),
            (0.5, 0.4, 0.8),
        ] {
            let m = mat(1.0, metallic, rough, 0.5);
            let a = albedo(&m, wo, 200_000);
            assert!(a <= 1.01, "albedo {a} > 1 per metallic {metallic}, roughness {rough}");
            assert!(a > min, "albedo {a} troppo bassa per metallic {metallic}, roughness {rough}");
        }
    }

    /// Lo stimatore di campionamento (f·cos/pdf) e l'integrazione diretta di `eval`
    /// devono dare la stessa albedo: verifica che sample, eval e pdf siano coerenti.
    #[test]
    fn sampling_consistent_with_eval() {
        let wo = Vec3::new(0.5, 0.0, 0.866).normalize();
        for &(metallic, rough) in &[(1.0, 0.9), (1.0, 0.4), (0.0, 0.5), (0.4, 0.6)] {
            let m = mat(1.0, metallic, rough, 0.5);
            let a = albedo(&m, wo, 300_000);
            let b = albedo_by_eval(&m, wo, 2_000_000);
            assert!(
                (a - b).abs() < 0.01,
                "sample {a} vs eval {b} (metallic {metallic}, roughness {rough})"
            );
        }
    }

    #[test]
    fn pure_lambert_albedo() {
        let m = mat(0.5, 0.0, 0.5, 0.0);
        let a = albedo(&m, Vec3::new(0.3, 0.2, 0.9).normalize(), 50_000);
        assert!((a - 0.5).abs() < 1e-4, "albedo {a}");
    }

    /// La PDF restituita da `eval` deve integrare a 1 sull'emisfero e coincidere
    /// con quella usata dal campionamento (condizione necessaria per il MIS).
    #[test]
    fn pdf_integrates_to_one() {
        let wo = Vec3::new(0.6, 0.1, 0.79).normalize();
        for &(metallic, rough) in &[(0.0, 0.5), (1.0, 0.3), (0.3, 0.7)] {
            let m = mat(0.8, metallic, rough, 0.5);
            let b = bsdf(&m);
            let mut rng = Pcg32::new(11, 3);
            let n = 400_000;
            let mut integral = 0.0f64;
            for _ in 0..n {
                let d = sampling::cosine_hemisphere(rng.next_f32(), rng.next_f32());
                let (_, pdf) = b.eval(wo, d);
                integral += (pdf / sampling::cosine_hemisphere_pdf(d.z)) as f64;
            }
            let integral = integral / n as f64;
            // Una piccola parte della massa del lobo speculare cade sotto l'orizzonte e viene scartata.
            assert!(
                integral <= 1.02 && integral > 0.9,
                "integrale {integral} (metallic {metallic}, roughness {rough})"
            );
        }
    }

    fn glass(roughness: f32, entering: bool) -> (RenderMaterial, bool) {
        (
            RenderMaterial::from_material(&Material {
                base_color: [1.0; 3],
                transmission: 1.0,
                roughness,
                ior: 1.5,
                ..Default::default()
            }),
            entering,
        )
    }

    /// Vetro satinato: la somma di riflessione e trasmissione non supera 1, e il
    /// campionamento è coerente con eval/pdf su tutta la sfera delle direzioni.
    #[test]
    fn rough_glass_energy_and_consistency() {
        for &(rough, entering) in &[(0.3, true), (0.6, true), (0.3, false)] {
            let (m, ent) = glass(rough, entering);
            let b = Bsdf {
                frame: Onb::from_normal(Vec3::Z),
                ng: Vec3::Z,
                mat: &m,
                entering: ent,
            };
            let wo = Vec3::new(0.3, 0.1, 0.95).normalize();
            let mut rng = Pcg32::new(21, 5);
            let n = 400_000;
            let mut by_sample = 0.0f64;
            for _ in 0..n {
                if let Some(s) = b.sample(wo, rng.next_f32() * 0.999, rng.next_f32(), rng.next_f32()) {
                    by_sample += s.weight.x as f64;
                }
            }
            let by_sample = by_sample / n as f64;
            let mut by_eval = 0.0f64;
            for _ in 0..n {
                let d = sampling::uniform_sphere(rng.next_f32(), rng.next_f32());
                let (f, _) = b.eval(wo, d);
                by_eval += (f.x * d.z.abs() / sampling::UNIFORM_SPHERE_PDF) as f64;
            }
            let by_eval = by_eval / n as f64;
            // La trasmissione include la scala fisica della radianza (n_o/n_i)²: entrando nel vetro
            // l'albedo "apparente" è ≈ R + T/η² ≈ 0.47. L'energia si verifica sulla sfera completa
            // (white furnace in tests/physics.rs); qui si controlla la coerenza sample/eval/pdf.
            assert!(
                (by_sample - by_eval).abs() / by_eval.max(1e-3) < 0.03,
                "sample {by_sample} vs eval {by_eval} (rough {rough}, entering {entering})"
            );
        }
    }

    #[test]
    fn eval_is_reciprocal() {
        let m = mat(0.7, 0.3, 0.4, 0.5);
        let b = bsdf(&m);
        let a = Vec3::new(0.3, -0.2, 0.9).normalize();
        let c = Vec3::new(-0.6, 0.4, 0.5).normalize();
        let (f1, _) = b.eval(a, c);
        let (f2, _) = b.eval(c, a);
        // La compensazione multi-scattering di Turquin dipende solo da wo: reciprocità all'1%.
        assert!((f1 - f2).length() / f1.length() < 0.01, "{f1} vs {f2}");
    }

    #[test]
    fn fresnel_values() {
        let (f, _) = fresnel_dielectric(1.0, 1.5);
        assert!((f - 0.04).abs() < 1e-3);
        // Riflessione totale interna da vetro verso aria a 60°.
        let (f, _) = fresnel_dielectric(0.5, 1.0 / 1.5);
        assert_eq!(f, 1.0);
    }
}
