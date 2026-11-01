//! Fondamenta del rendering spettrale (modalità "Spettrale", accanto a quella RGB).
//!
//! - **Lunghezze d'onda**: 4 per percorso tra 360 e 830 nm, con la distribuzione "visibile" di
//!   PBRT v4 (∝ 1/cosh²(0,0072·(λ − 538))) stratificata a passi di 1/4 (Wilkie et al. 2014,
//!   "Hero Wavelength Spectral Sampling"; PBRT v4, `SampledWavelengths::SampleVisible`). La prima
//!   è la "hero": quando un evento dipende dalla lunghezza d'onda (vetro con dispersione) le altre
//!   si fermano e la sua pdf si divide per 4 (`TerminateSecondary` di PBRT v4), così la stima
//!   resta non distorta.
//! - **Da RGB a spettro**: sigmoide di un polinomio di secondo grado, s(x) = ½ + x / (2√(1 + x²))
//!   (Jakob e Hanika 2019, "A Low-Dimensional Function Space for Efficient Spectral Upsampling",
//!   come in PBRT v4). I coefficienti stanno in una tabella 3 × 64³ calcolata una volta per
//!   processo con un'ottimizzazione di Gauss-Newton (scritta qui, nessun file esterno).
//!   Riflettanze in [0, 1] così come sono; emissioni senza limite con la scala 2·max (PBRT v4).
//! - **Colore**: funzioni colorimetriche CIE 1931 (2°) con l'approssimazione analitica a gaussiane
//!   a tratti di Wyman, Sloan e Shirley (2013); da XYZ a RGB lineare (primari sRGB) con un
//!   adattamento di Bradford dal bianco a energia uguale. Il bianco di riferimento è lo spettro
//!   costante: una luce RGB (1, 1, 1) resta bianca e una riflettanza RGB torna (quasi) uguale,
//!   come nel render RGB. (PBRT moltiplica le luci per il D65; qui i colori delle luci sono già
//!   espressi nello spazio RGB della scena, che ha il bianco in (1, 1, 1).)
//! - **Dispersione**: equazione di Cauchy n(λ) = A + B/λ² dal numero di Abbe del materiale.
//!
//! La conversione da XYZ a RGB è lineare: accumulare RGB equivale ad accumulare XYZ e convertire
//! alla fine, quindi il resto della pipeline (accumulo, denoiser, tone mapping, EXR) non cambia.

use forge_math::Vec3;
use rayon::prelude::*;
use std::sync::OnceLock;

/// Lunghezze d'onda per percorso.
pub const N_LAMBDA: usize = 4;
pub const LAMBDA_MIN: f32 = 360.0;
pub const LAMBDA_MAX: f32 = 830.0;

/// Distribuzione "visibile" di PBRT v4: pdf ∝ 1/cosh²(A·(λ − B)) su [360, 830] nm.
const VIS_A: f64 = 0.0072;
const VIS_B: f64 = 538.0;

fn vis_tanh_range() -> (f64, f64) {
    (
        (VIS_A * (LAMBDA_MIN as f64 - VIS_B)).tanh(),
        (VIS_A * (LAMBDA_MAX as f64 - VIS_B)).tanh(),
    )
}

/// Densità della distribuzione visibile (per nm).
pub fn visible_pdf(lambda: f32) -> f32 {
    if !(LAMBDA_MIN..=LAMBDA_MAX).contains(&lambda) {
        return 0.0;
    }
    let (t0, t1) = vis_tanh_range();
    let c = (VIS_A * (lambda as f64 - VIS_B)).cosh();
    (VIS_A / (t1 - t0) / (c * c)) as f32
}

/// Lunghezza d'onda con la distribuzione visibile (inversione esatta della CDF).
pub fn sample_visible(u: f32) -> f32 {
    let (t0, t1) = vis_tanh_range();
    let t = (t0 + u as f64 * (t1 - t0)).clamp(-0.999_999_999, 0.999_999_999);
    ((VIS_B + t.atanh() / VIS_A) as f32).clamp(LAMBDA_MIN, LAMBDA_MAX)
}

/// Lunghezze d'onda di un percorso e le loro densità.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wavelengths {
    pub lambda: [f32; N_LAMBDA],
    pub pdf: [f32; N_LAMBDA],
}

impl Wavelengths {
    /// Quattro lunghezze d'onda stratificate: u, u + ¼, u + ½, u + ¾ (modulo 1).
    pub fn sample(u: f32) -> Self {
        let mut lambda = [0.0; N_LAMBDA];
        let mut pdf = [0.0; N_LAMBDA];
        for i in 0..N_LAMBDA {
            let mut up = u + i as f32 / N_LAMBDA as f32;
            if up >= 1.0 {
                up -= 1.0;
            }
            lambda[i] = sample_visible(up);
            pdf[i] = visible_pdf(lambda[i]);
        }
        Self { lambda, pdf }
    }

    pub fn hero(&self) -> f32 {
        self.lambda[0]
    }

    pub fn secondary_terminated(&self) -> bool {
        self.pdf[1..].iter().all(|&p| p == 0.0)
    }

    /// Resta solo la hero: la sua pdf include la probabilità 1/4 di essere quella scelta.
    pub fn terminate_secondary(&mut self) {
        if self.secondary_terminated() {
            return;
        }
        for p in &mut self.pdf[1..] {
            *p = 0.0;
        }
        self.pdf[0] /= N_LAMBDA as f32;
    }

    /// Radianza campionata → RGB lineare della scena (stima a un campione dell'integrale con le
    /// funzioni colorimetriche, normalizzata: uno spettro costante 1 dà (1, 1, 1)).
    pub fn to_rgb(&self, radiance: [f32; N_LAMBDA]) -> Vec3 {
        let mut xyz = [0.0f32; 3];
        for ((&l, &p), &r) in self.lambda.iter().zip(&self.pdf).zip(&radiance) {
            if p <= 0.0 || r == 0.0 {
                continue;
            }
            let c = cie_xyz(l);
            let w = r / p;
            for k in 0..3 {
                xyz[k] += c[k] * w;
            }
        }
        let t = tables();
        let s = 1.0 / (N_LAMBDA as f32 * t.y_integral);
        let m = &t.xyz_to_rgb;
        let (x, y, z) = (xyz[0] * s, xyz[1] * s, xyz[2] * s);
        Vec3::new(
            m[0][0] * x + m[0][1] * y + m[0][2] * z,
            m[1][0] * x + m[1][1] * y + m[1][2] * z,
            m[2][0] * x + m[2][1] * y + m[2][2] * z,
        )
    }
}

/// RGB lineare della scena di un valore spettrale `value` (per nm) alla lunghezza d'onda `lambda`:
/// il contributo di un solo campione (la divisione per la pdf della lunghezza d'onda la fa chi
/// chiama). Serve ai fotoni, che portano una lunghezza d'onda ciascuno.
pub fn lambda_to_rgb(lambda: f32, value: f32) -> Vec3 {
    let t = tables();
    let c = cie_xyz(lambda);
    let s = value / t.y_integral;
    let m = &t.xyz_to_rgb;
    let (x, y, z) = (c[0] * s, c[1] * s, c[2] * s);
    Vec3::new(
        m[0][0] * x + m[0][1] * y + m[0][2] * z,
        m[1][0] * x + m[1][1] * y + m[1][2] * z,
        m[2][0] * x + m[2][1] * y + m[2][2] * z,
    )
}

/// Gaussiana a tratti (σ diverse a sinistra e a destra del picco).
#[inline]
fn g(l: f64, mu: f64, s1: f64, s2: f64) -> f64 {
    let t = (l - mu) / if l < mu { s1 } else { s2 };
    (-0.5 * t * t).exp()
}

/// Funzioni colorimetriche CIE 1931 2° (approssimazione di Wyman, Sloan e Shirley 2013, errore
/// tipico sotto l'1% del picco).
pub fn cie_xyz(lambda: f32) -> [f32; 3] {
    let l = lambda as f64;
    let x = 1.056 * g(l, 599.8, 37.9, 31.0) + 0.362 * g(l, 442.0, 16.0, 26.7) - 0.065 * g(l, 501.1, 20.4, 26.2);
    let y = 0.821 * g(l, 568.8, 46.9, 40.5) + 0.286 * g(l, 530.9, 16.3, 31.1);
    let z = 1.217 * g(l, 437.0, 11.8, 36.0) + 0.681 * g(l, 459.0, 26.0, 13.8);
    [x as f32, y as f32, z as f32]
}

/// Passo e numero dei punti di integrazione (1 nm, regola dei trapezi).
const INT_STEP: f64 = 1.0;
const INT_N: usize = ((LAMBDA_MAX - LAMBDA_MIN) as usize) + 1;

fn int_weight(i: usize) -> f64 {
    if i == 0 || i == INT_N - 1 {
        0.5 * INT_STEP
    } else {
        INT_STEP
    }
}

/// Matrici e costanti del colore, calcolate una volta.
struct ColorTables {
    /// ∫ ȳ dλ.
    y_integral: f32,
    xyz_to_rgb: [[f32; 3]; 3],
    rgb_to_xyz: [[f64; 3]; 3],
    /// XYZ del bianco (spettro costante), normalizzato a Y = 1.
    white: [f64; 3],
    /// Funzioni colorimetriche ai punti di integrazione, già pesate e normalizzate (Y del bianco = 1).
    cmf: Vec<[f64; 3]>,
}

fn mat_mul(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut r = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    r
}

fn mat_vec(a: &[[f64; 3]; 3], v: [f64; 3]) -> [f64; 3] {
    [0, 1, 2].map(|i| a[i][0] * v[0] + a[i][1] * v[1] + a[i][2] * v[2])
}

fn mat_inv(m: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let det = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1]) - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
    let inv = 1.0 / det;
    [
        [
            (m[1][1] * m[2][2] - m[1][2] * m[2][1]) * inv,
            (m[0][2] * m[2][1] - m[0][1] * m[2][2]) * inv,
            (m[0][1] * m[1][2] - m[0][2] * m[1][1]) * inv,
        ],
        [
            (m[1][2] * m[2][0] - m[1][0] * m[2][2]) * inv,
            (m[0][0] * m[2][2] - m[0][2] * m[2][0]) * inv,
            (m[0][2] * m[1][0] - m[0][0] * m[1][2]) * inv,
        ],
        [
            (m[1][0] * m[2][1] - m[1][1] * m[2][0]) * inv,
            (m[0][1] * m[2][0] - m[0][0] * m[2][1]) * inv,
            (m[0][0] * m[1][1] - m[0][1] * m[1][0]) * inv,
        ],
    ]
}

fn tables() -> &'static ColorTables {
    static T: OnceLock<ColorTables> = OnceLock::new();
    T.get_or_init(|| {
        let mut sum = [0.0f64; 3];
        let mut raw = Vec::with_capacity(INT_N);
        for i in 0..INT_N {
            let c = cie_xyz((LAMBDA_MIN as f64 + i as f64 * INT_STEP) as f32).map(|v| v as f64);
            let w = int_weight(i);
            for k in 0..3 {
                sum[k] += c[k] * w;
            }
            raw.push(c.map(|v| v * w));
        }
        let y_int = sum[1];
        let white = sum.map(|v| v / y_int);
        let cmf = raw.into_iter().map(|c| c.map(|v| v / y_int)).collect();
        // XYZ → sRGB lineare (bianco D65) e adattamento di Bradford dal bianco a energia uguale.
        const SRGB: [[f64; 3]; 3] = [
            [3.2404542, -1.5371385, -0.4985314],
            [-0.9692660, 1.8760108, 0.0415560],
            [0.0556434, -0.2040259, 1.0572252],
        ];
        const BRADFORD: [[f64; 3]; 3] = [[0.8951, 0.2664, -0.1614], [-0.7502, 1.7135, 0.0367], [0.0389, -0.0685, 1.0296]];
        const D65: [f64; 3] = [0.95047, 1.0, 1.08883];
        let src = mat_vec(&BRADFORD, white);
        let dst = mat_vec(&BRADFORD, D65);
        let scale = [
            [dst[0] / src[0], 0.0, 0.0],
            [0.0, dst[1] / src[1], 0.0],
            [0.0, 0.0, dst[2] / src[2]],
        ];
        let adapt = mat_mul(&mat_inv(&BRADFORD), &mat_mul(&scale, &BRADFORD));
        let mut m = mat_mul(&SRGB, &adapt);
        // Il bianco va esattamente in (1, 1, 1).
        let w = mat_vec(&m, white);
        for (i, row) in m.iter_mut().enumerate() {
            for v in row.iter_mut() {
                *v /= w[i];
            }
        }
        ColorTables {
            y_integral: y_int as f32,
            xyz_to_rgb: m.map(|r| r.map(|v| v as f32)),
            rgb_to_xyz: mat_inv(&m),
            white,
            cmf,
        }
    })
}

/// XYZ (Y del bianco = 1) di uno spettro dato per punti, con le funzioni colorimetriche.
fn spectrum_xyz(f: impl Fn(f64) -> f64) -> [f64; 3] {
    let t = tables();
    let mut xyz = [0.0; 3];
    for (i, c) in t.cmf.iter().enumerate() {
        let v = f(LAMBDA_MIN as f64 + i as f64 * INT_STEP);
        for k in 0..3 {
            xyz[k] += c[k] * v;
        }
    }
    xyz
}

/// RGB lineare della scena di uno spettro (integrazione numerica): per i test e le verifiche.
pub fn spectrum_to_rgb(f: impl Fn(f32) -> f32) -> Vec3 {
    let xyz = spectrum_xyz(|l| f(l as f32) as f64);
    let m = &tables().xyz_to_rgb;
    let r = [0, 1, 2].map(|i| m[i][0] * xyz[0] as f32 + m[i][1] * xyz[1] as f32 + m[i][2] * xyz[2] as f32);
    Vec3::from_array(r)
}

// ---------------------------------------------------------------------------------------------
// Da RGB a spettro: sigmoide di un polinomio.
// ---------------------------------------------------------------------------------------------

/// Lunghezza d'onda normalizzata in [0, 1] (coefficienti meglio condizionati).
#[inline]
fn norm_lambda(l: f64) -> f64 {
    (l - LAMBDA_MIN as f64) / (LAMBDA_MAX - LAMBDA_MIN) as f64
}

#[inline]
fn sigmoid(x: f64) -> f64 {
    if x.is_infinite() {
        return if x > 0.0 { 1.0 } else { 0.0 };
    }
    0.5 + x / (2.0 * (1.0 + x * x).sqrt())
}

/// Spettro liscio s(c₀t² + c₁t + c₂), t = λ normalizzata, moltiplicato per `scale`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RgbSpectrum {
    pub c: [f32; 3],
    pub scale: f32,
}

impl RgbSpectrum {
    pub const ZERO: Self = Self {
        c: [0.0, 0.0, -1e30],
        scale: 0.0,
    };

    #[inline]
    pub fn eval(&self, lambda: f32) -> f32 {
        if self.scale == 0.0 {
            return 0.0;
        }
        let t = norm_lambda(lambda as f64) as f32;
        let x = (self.c[0] * t + self.c[1]) * t + self.c[2];
        self.scale * sigmoid(x as f64) as f32
    }

    /// Valori alle quattro lunghezze d'onda del percorso.
    #[inline]
    pub fn sample(&self, w: &Wavelengths) -> [f32; N_LAMBDA] {
        w.lambda.map(|l| self.eval(l))
    }

    /// Riflettanza (valori in [0, 1]): lo spettro liscio con quel colore.
    pub fn albedo(rgb: Vec3) -> Self {
        let rgb = rgb.clamp(Vec3::ZERO, Vec3::ONE);
        Self {
            c: rgb_table().coefficients(rgb.to_array()),
            scale: 1.0,
        }
    }

    /// Emissione o coefficiente senza limite superiore (≥ 0): forma dalla tabella, intensità nella
    /// scala 2·max (PBRT v4, `RGBUnboundedSpectrum`).
    pub fn unbounded(rgb: Vec3) -> Self {
        let rgb = rgb.max(Vec3::ZERO);
        let m = rgb.max_element();
        if m <= 0.0 || !m.is_finite() {
            return Self::ZERO;
        }
        let scale = 2.0 * m;
        Self {
            c: rgb_table().coefficients((rgb / scale).to_array()),
            scale,
        }
    }
}

/// Lato della tabella dei coefficienti.
const RES: usize = 64;

/// Tabella dei coefficienti: per ogni canale massimo (r, g, b), z = valore massimo (nodi più fitti
/// vicino a 0 e a 1), x, y = gli altri due canali divisi per il massimo.
pub struct RgbToSpectrumTable {
    z_nodes: Vec<f32>,
    /// [max][z][y][x] → 3 coefficienti.
    coeffs: Vec<[f32; 3]>,
}

fn smoothstep(x: f64) -> f64 {
    x * x * (3.0 - 2.0 * x)
}

impl RgbToSpectrumTable {
    #[inline]
    fn idx(m: usize, z: usize, y: usize, x: usize) -> usize {
        ((m * RES + z) * RES + y) * RES + x
    }

    /// Coefficienti per un RGB in [0, 1] (interpolazione trilineare).
    pub fn coefficients(&self, rgb: [f32; 3]) -> [f32; 3] {
        let rgb = rgb.map(|v| v.clamp(0.0, 1.0));
        if rgb[0] == rgb[1] && rgb[1] == rgb[2] {
            // Grigio: spettro costante (c₀ = c₁ = 0).
            let v = rgb[0].clamp(1e-6, 1.0 - 1e-6);
            return [0.0, 0.0, (v - 0.5) / (v * (1.0 - v)).sqrt()];
        }
        let m = if rgb[0] >= rgb[1] && rgb[0] >= rgb[2] {
            0
        } else if rgb[1] >= rgb[2] {
            1
        } else {
            2
        };
        let z = rgb[m];
        let x = rgb[(m + 1) % 3] / z * (RES - 1) as f32;
        let y = rgb[(m + 2) % 3] / z * (RES - 1) as f32;
        let zi = match self.z_nodes.binary_search_by(|n| n.partial_cmp(&z).unwrap()) {
            Ok(i) => i.min(RES - 2),
            Err(i) => i.saturating_sub(1).min(RES - 2),
        };
        let xi = (x as usize).min(RES - 2);
        let yi = (y as usize).min(RES - 2);
        let dx = x - xi as f32;
        let dy = y - yi as f32;
        let dz = ((z - self.z_nodes[zi]) / (self.z_nodes[zi + 1] - self.z_nodes[zi])).clamp(0.0, 1.0);
        let mut out = [0.0f32; 3];
        for (k, o) in out.iter_mut().enumerate() {
            let c = |zz: usize, yy: usize, xx: usize| self.coeffs[Self::idx(m, zz, yy, xx)][k];
            let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
            let c00 = lerp(c(zi, yi, xi), c(zi, yi, xi + 1), dx);
            let c01 = lerp(c(zi, yi + 1, xi), c(zi, yi + 1, xi + 1), dx);
            let c10 = lerp(c(zi + 1, yi, xi), c(zi + 1, yi, xi + 1), dx);
            let c11 = lerp(c(zi + 1, yi + 1, xi), c(zi + 1, yi + 1, xi + 1), dx);
            *o = lerp(lerp(c00, c01, dy), lerp(c10, c11, dy), dz);
        }
        out
    }

    /// Calcola la tabella (qualche secondo su tutti i core).
    fn build() -> Self {
        let z_nodes: Vec<f32> = (0..RES)
            .map(|k| smoothstep(smoothstep(k as f64 / (RES - 1) as f64)) as f32)
            .collect();
        let t0 = std::time::Instant::now();
        // Una colonna (m, y, x) alla volta: si parte da z ≈ 1/5 con coefficienti nulli e si sale e
        // si scende riusando la soluzione precedente come punto di partenza (continuazione).
        // Pool dedicato: il calcolo (qualche secondo su tutti i core) non blocca chi usa il pool
        // globale (accumulo della sessione, interfaccia) né il pool del backend CPU.
        let compute = || -> Vec<Vec<[f32; 3]>> {
            (0..3 * RES * RES)
                .into_par_iter()
                .map(|col| {
                    let m = col / (RES * RES);
                    let y = (col / RES) % RES;
                    let x = col % RES;
                    let mut out = vec![[0.0f32; 3]; RES];
                    let start = RES / 5;
                    let solve_at = |zk: usize, c: &mut [f64; 3]| {
                        let z = z_nodes[zk] as f64;
                        let xv = x as f64 / (RES - 1) as f64 * z;
                        let yv = y as f64 / (RES - 1) as f64 * z;
                        let mut rgb = [0.0; 3];
                        rgb[m] = z;
                        rgb[(m + 1) % 3] = xv;
                        rgb[(m + 2) % 3] = yv;
                        gauss_newton(rgb, c);
                    };
                    let mut c = [0.0f64; 3];
                    for (zk, o) in out.iter_mut().enumerate().skip(start) {
                        solve_at(zk, &mut c);
                        *o = c.map(|v| v as f32);
                    }
                    let mut c = out[start].map(|v| v as f64);
                    for zk in (0..start).rev() {
                        solve_at(zk, &mut c);
                        out[zk] = c.map(|v| v as f32);
                    }
                    out
                })
                .collect()
        };
        let columns = match rayon::ThreadPoolBuilder::new()
            .thread_name(|i| format!("forge-rgb2spec-{i}"))
            .build()
        {
            Ok(pool) => pool.install(compute),
            Err(_) => compute(),
        };
        let mut coeffs = vec![[0.0f32; 3]; 3 * RES * RES * RES];
        for (col, data) in columns.into_iter().enumerate() {
            let m = col / (RES * RES);
            let y = (col / RES) % RES;
            let x = col % RES;
            for (z, c) in data.into_iter().enumerate() {
                coeffs[Self::idx(m, z, y, x)] = c;
            }
        }
        tracing::info!("tabella RGB → spettro calcolata in {:.2} s", t0.elapsed().as_secs_f64());
        Self { z_nodes, coeffs }
    }

    /// Dati grezzi per la GPU: nodi z, poi coefficienti (3 float per voce, ordine [m][z][y][x]).
    pub fn raw(&self) -> (&[f32], &[[f32; 3]]) {
        (&self.z_nodes, &self.coeffs)
    }
}

/// CIELAB rispetto al bianco della scena.
fn lab(xyz: [f64; 3], white: [f64; 3]) -> [f64; 3] {
    let f = |t: f64| {
        let d = 6.0 / 29.0;
        if t > d * d * d {
            t.cbrt()
        } else {
            t / (3.0 * d * d) + 4.0 / 29.0
        }
    };
    let (fx, fy, fz) = (f(xyz[0] / white[0]), f(xyz[1] / white[1]), f(xyz[2] / white[2]));
    [116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz)]
}

/// Residuo in CIELAB tra lo spettro di coefficienti `c` e il colore `target` (RGB della scena).
fn residual(c: &[f64; 3], target_lab: &[f64; 3]) -> [f64; 3] {
    let xyz = spectrum_xyz(|l| {
        let t = norm_lambda(l);
        sigmoid((c[0] * t + c[1]) * t + c[2])
    });
    let l = lab(xyz, tables().white);
    [l[0] - target_lab[0], l[1] - target_lab[1], l[2] - target_lab[2]]
}

/// Gauss-Newton sui tre coefficienti (Jacobiano per differenze finite), come rgb2spec.
fn gauss_newton(rgb: [f64; 3], c: &mut [f64; 3]) {
    let t = tables();
    let target = lab(mat_vec(&t.rgb_to_xyz, rgb), t.white);
    for _ in 0..30 {
        let r = residual(c, &target);
        let norm = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
        if norm < 1e-6 {
            break;
        }
        let mut j = [[0.0f64; 3]; 3];
        for k in 0..3 {
            let eps = 1e-5;
            let mut cp = *c;
            cp[k] += eps;
            let rp = residual(&cp, &target);
            for i in 0..3 {
                j[i][k] = (rp[i] - r[i]) / eps;
            }
        }
        let det = j[0][0] * (j[1][1] * j[2][2] - j[1][2] * j[2][1]) - j[0][1] * (j[1][0] * j[2][2] - j[1][2] * j[2][0])
            + j[0][2] * (j[1][0] * j[2][1] - j[1][1] * j[2][0]);
        if !det.is_finite() || det.abs() < 1e-15 {
            break;
        }
        let ji = mat_inv(&j);
        let d = mat_vec(&ji, r);
        for k in 0..3 {
            c[k] -= d[k];
        }
        // Coefficienti enormi: si riportano in un intervallo ragionevole (come rgb2spec).
        let max = c.iter().fold(0.0f64, |a, v| a.max(v.abs()));
        if max > 200.0 {
            for v in c.iter_mut() {
                *v *= 200.0 / max;
            }
        }
    }
}

/// Posizioni nei dati spettrali per la GPU (vedi [`gpu_data`]).
pub const GPU_MATRIX: usize = 0;
pub const GPU_Y_INTEGRAL: usize = 9;
pub const GPU_Z_NODES: usize = 10;
pub const GPU_COEFFS: usize = GPU_Z_NODES + RES;
pub const GPU_RES: usize = RES;

/// Dati del render spettrale per lo shader GPU, in un solo array di float: matrice XYZ → RGB
/// (9, per righe), ∫ȳ, nodi z (64), coefficienti (3 × 64³ × 3, ordine [canale max][z][y][x]).
pub fn gpu_data() -> Vec<f32> {
    let t = tables();
    let table = rgb_table();
    let (z, c) = table.raw();
    let mut v = Vec::with_capacity(GPU_COEFFS + c.len() * 3);
    v.extend(t.xyz_to_rgb.iter().flatten());
    v.push(t.y_integral);
    v.extend_from_slice(z);
    v.extend(c.iter().flatten());
    debug_assert_eq!(v.len(), GPU_COEFFS + 3 * RES * RES * RES * 3);
    v
}

/// Tabella condivisa (calcolata alla prima richiesta).
pub fn rgb_table() -> &'static RgbToSpectrumTable {
    static T: OnceLock<RgbToSpectrumTable> = OnceLock::new();
    T.get_or_init(RgbToSpectrumTable::build)
}

/// Indice di rifrazione alla lunghezza d'onda `lambda` (nm) con l'equazione di Cauchy a due
/// termini n = A + B/λ², ricavata dall'indice alla riga d (587,6 nm) e dal numero di Abbe
/// V = (n_d − 1)/(n_F − n_C). `abbe` ≤ 0: nessuna dispersione.
pub fn cauchy_ior(n_d: f32, abbe: f32, lambda: f32) -> f32 {
    if abbe <= 0.0 || n_d <= 1.0 {
        return n_d;
    }
    // Lunghezze d'onda delle righe di Fraunhofer in µm.
    const D: f32 = 0.587_56;
    const F: f32 = 0.486_13;
    const C: f32 = 0.656_27;
    let b = (n_d - 1.0) / (abbe * (1.0 / (F * F) - 1.0 / (C * C)));
    let a = n_d - b / (D * D);
    let l = lambda * 1e-3;
    a + b / (l * l)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_pdf_integrates_to_one_and_sampling_matches() {
        let n = 47_000;
        let integral: f64 = (0..n)
            .map(|i| {
                let l = LAMBDA_MIN as f64 + (i as f64 + 0.5) * (LAMBDA_MAX - LAMBDA_MIN) as f64 / n as f64;
                visible_pdf(l as f32) as f64 * (LAMBDA_MAX - LAMBDA_MIN) as f64 / n as f64
            })
            .sum();
        assert!((integral - 1.0).abs() < 1e-4, "{integral}");
        // CDF campionata: la frazione sotto 538 nm coincide con l'integrale della pdf.
        let below: f64 = (0..n)
            .filter_map(|i| {
                let l = LAMBDA_MIN as f64 + (i as f64 + 0.5) * (LAMBDA_MAX - LAMBDA_MIN) as f64 / n as f64;
                (l < 538.0).then(|| visible_pdf(l as f32) as f64 * (LAMBDA_MAX - LAMBDA_MIN) as f64 / n as f64)
            })
            .sum();
        let m = 100_000;
        let frac = (0..m).filter(|i| sample_visible((*i as f32 + 0.5) / m as f32) < 538.0).count() as f64 / m as f64;
        assert!((frac - below).abs() < 1e-3, "{frac} vs {below}");
        // Costante della pdf come in PBRT v4 (0,0039398042 al picco).
        assert!((visible_pdf(538.0) - 0.003_939_804).abs() < 1e-7);
    }

    #[test]
    fn terminating_secondaries_keeps_the_estimate_unbiased() {
        let mut w = Wavelengths::sample(0.3);
        let p0 = w.pdf[0];
        w.terminate_secondary();
        assert_eq!(w.pdf, [p0 / 4.0, 0.0, 0.0, 0.0]);
        // Media su molte lunghezze d'onda: lo spettro costante 1 dà il bianco anche con le sole hero.
        let n = 20_000;
        let mut sum = Vec3::ZERO;
        let mut sum_t = Vec3::ZERO;
        for i in 0..n {
            let mut w = Wavelengths::sample((i as f32 + 0.5) / n as f32);
            sum += w.to_rgb([1.0; 4]);
            w.terminate_secondary();
            sum_t += w.to_rgb([1.0; 4]);
        }
        let (a, b) = (sum / n as f32, sum_t / n as f32);
        assert!((a - Vec3::ONE).abs().max_element() < 2e-3, "{a}");
        assert!((b - Vec3::ONE).abs().max_element() < 2e-3, "{b}");
    }

    #[test]
    fn constant_spectrum_is_white() {
        let w = spectrum_to_rgb(|_| 1.0);
        assert!((w - Vec3::ONE).abs().max_element() < 1e-4, "{w}");
    }

    /// RGB → spettro → RGB: le riflettanze tornano al colore di partenza (errore piccolo anche
    /// sui colori saturi), le emissioni anche con valori grandi.
    #[test]
    fn rgb_round_trip() {
        let mut worst = 0.0f32;
        for &c in &[
            [0.8, 0.2, 0.1],
            [0.1, 0.7, 0.3],
            [0.05, 0.1, 0.9],
            [0.9, 0.9, 0.1],
            [0.5, 0.5, 0.5],
            [0.02, 0.02, 0.02],
            [0.95, 0.6, 0.4],
            [0.3, 0.05, 0.6],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
        ] {
            let rgb = Vec3::from_array(c);
            let s = RgbSpectrum::albedo(rgb);
            let back = spectrum_to_rgb(|l| s.eval(l));
            let err = (back - rgb).abs().max_element();
            worst = worst.max(err);
            assert!(err < 0.02, "{rgb} → {back}");
            assert!(
                (0..=470).all(|i| (0.0..=1.0).contains(&s.eval(360.0 + i as f32))),
                "riflettanza fuori da [0, 1]"
            );
        }
        let e = Vec3::new(12.0, 6.0, 2.0);
        let back = spectrum_to_rgb(|l| RgbSpectrum::unbounded(e).eval(l));
        assert!((back - e).abs().max_element() / 12.0 < 0.01, "{back}");
        assert_eq!(RgbSpectrum::unbounded(Vec3::ZERO).eval(500.0), 0.0);
        eprintln!("errore massimo del giro RGB → spettro → RGB: {worst}");
    }

    #[test]
    fn cauchy_matches_abbe_definition() {
        // BK7: n_d = 1,5168, V = 64,17.
        let (nd, v) = (1.5168, 64.17);
        let n = |l| cauchy_ior(nd, v, l);
        assert!((n(587.56) - nd).abs() < 1e-5);
        let abbe = (n(587.56) - 1.0) / (n(486.13) - n(656.27));
        assert!((abbe - v).abs() < 0.01, "{abbe}");
        assert!(n(400.0) > n(700.0));
        assert_eq!(cauchy_ior(1.5, 0.0, 400.0), 1.5);
    }
}
