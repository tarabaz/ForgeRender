use crate::{RenderScene, RenderSettings};
use std::sync::atomic::AtomicBool;

/// Un backend di rendering. Ogni chiamata a [`render_pass`](Self::render_pass)
/// calcola **un campione per pixel** per l'intera immagine e lo scrive in `out`
/// (radianza lineare, riga per riga dall'alto). L'accumulo è gestito dalla
/// sessione, così ogni backend produce campioni intercambiabili.
pub trait RenderBackend: Send {
    fn name(&self) -> String;

    /// Descrizione delle risorse usate (thread, dispositivo...).
    fn device_info(&self) -> String;

    /// Restituisce `Ok(true)` se la passata è completa, `Ok(false)` se è stata
    /// interrotta da `cancel` (in tal caso `out` va scartato).
    fn render_pass(
        &mut self,
        scene: &RenderScene,
        settings: &RenderSettings,
        sample_index: u32,
        out: &mut [[f32; 3]],
        cancel: &AtomicBool,
    ) -> anyhow::Result<bool>;
}
