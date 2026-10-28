//! Interfaccia dei denoiser (riduzione del rumore sull'immagine accumulata).
//!
//! La sessione di rendering non conosce l'implementazione (Open Image Denoise
//! è caricato a runtime da `forge_denoise`): riceve un `Box<dyn Denoiser>`.

use crate::{DenoiseQuality, RenderScene, RenderSettings};
use std::sync::atomic::AtomicBool;

/// Buffer guida per il denoiser, calcolati dalla camera con pochi campioni.
#[derive(Clone, Debug, Default)]
pub struct Guides {
    pub width: u32,
    pub height: u32,
    /// Colore base della prima superficie non speculare (attraverso vetri e specchi).
    pub albedo: Vec<[f32; 3]>,
    /// Normale di shading in spazio mondo, componenti in [-1, 1].
    pub normal: Vec<[f32; 3]>,
}

pub trait Denoiser: Send {
    /// Descrizione del filtro e del dispositivo usato l'ultima volta (es. "OIDN 2.3.3 · CUDA").
    fn name(&self) -> String;

    /// Calcola albedo e normali per la scena; `None` se `cancel` interrompe il calcolo.
    fn guides(&mut self, scene: &RenderScene, settings: &RenderSettings, cancel: &AtomicBool) -> Option<Guides>;

    /// Restituisce l'immagine ripulita (radianza lineare, stesse dimensioni).
    fn denoise(
        &mut self,
        color: &[[f32; 3]],
        guides: Option<&Guides>,
        width: u32,
        height: u32,
        quality: DenoiseQuality,
        use_cuda: bool,
    ) -> anyhow::Result<Vec<[f32; 3]>>;
}

/// Miscela lineare tra immagine grezza e ripulita (`strength` 0 = grezza, 1 = ripulita).
pub fn mix_denoised(raw: &[[f32; 3]], denoised: &[[f32; 3]], strength: f32) -> Vec<[f32; 3]> {
    let t = strength.clamp(0.0, 1.0);
    raw.iter()
        .zip(denoised)
        .map(|(r, d)| [r[0] + (d[0] - r[0]) * t, r[1] + (d[1] - r[1]) * t, r[2] + (d[2] - r[2]) * t])
        .collect()
}
