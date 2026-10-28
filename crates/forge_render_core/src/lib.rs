//! Interfacce comuni ai backend di rendering.
//!
//! - [`RenderSettings`]: parametri serializzabili nel progetto.
//! - [`RenderScene`]: scena "compilata" (triangoli in spazio mondo, BVH,
//!   materiali e luci pronti per l'integratore), indipendente dal backend.
//! - [`RenderBackend`]: un backend calcola una passata da 1 campione per pixel.
//! - [`RenderSession`]: thread di lavoro con avvio/pausa/ripresa/annullamento
//!   e accumulo progressivo dei campioni.

mod accum;
mod backend;
mod denoise;
pub mod displace;
pub mod env_map;
mod render_scene;
mod session;
mod settings;
pub mod texture;
mod tonemap;

pub use accum::Accumulator;
pub use backend::RenderBackend;
pub use denoise::{mix_denoised, Denoiser, Guides};
pub use env_map::EnvironmentMap;
pub use render_scene::{pick, EmissiveSet, MaterialTextures, RenderGeometry, RenderLight, RenderLightKind, RenderMaterial, RenderScene};
pub use session::{Frame, Progress, RenderSession, SessionState};
pub use settings::{BackendKind, DenoiseMode, DenoiseQuality, DenoiseSettings, RenderSettings, SamplesPreset, ToneMapping};
pub use tonemap::{tonemap_pixel, tonemap_to_rgba8};
