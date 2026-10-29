use crate::{RenderScene, RenderSettings};
use std::sync::atomic::AtomicBool;

/// Un backend di rendering. Ogni chiamata a [`render_pass`](Self::render_pass)
/// calcola `n ≥ 1` campioni per pixel (al massimo `max_samples`) per l'intera
/// immagine e scrive in `out` la **somma** delle radianze di quei campioni
/// (riga per riga dall'alto). Il campione `k` della chiamata ha indice globale
/// `first_sample + k`, così i campioni restano intercambiabili tra backend.
/// L'accumulo è gestito dalla sessione.
pub trait RenderBackend: Send {
    fn name(&self) -> String;

    /// Descrizione delle risorse usate (thread, dispositivo...).
    fn device_info(&self) -> String;

    /// Restituisce il numero di campioni calcolati, oppure `0` se la passata è
    /// stata interrotta da `cancel` (in tal caso `out` va scartato).
    fn render_pass(
        &mut self,
        scene: &RenderScene,
        settings: &RenderSettings,
        first_sample: u32,
        max_samples: u32,
        out: &mut [[f32; 3]],
        cancel: &AtomicBool,
    ) -> anyhow::Result<u32>;

    /// Come [`render_pass`](Self::render_pass), ma solo i pixel dei riquadri attivi della
    /// maschera contano (campionamento adattivo): un backend può saltare gli altri e
    /// scriverci 0. Il comportamento predefinito calcola tutta l'immagine (corretto, la
    /// sessione ignora i pixel fermi, ma senza risparmio di tempo).
    #[allow(clippy::too_many_arguments)]
    fn render_pass_masked(
        &mut self,
        scene: &RenderScene,
        settings: &RenderSettings,
        first_sample: u32,
        max_samples: u32,
        out: &mut [[f32; 3]],
        cancel: &AtomicBool,
        _mask: &crate::adaptive::TileMask,
    ) -> anyhow::Result<u32> {
        self.render_pass(scene, settings, first_sample, max_samples, out, cancel)
    }
}
