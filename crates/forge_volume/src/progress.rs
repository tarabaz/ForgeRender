//! Avanzamento del calcolo della nebbia, per la barra nell'interfaccia: fase in corso e frazione
//! fatta. Il calcolo gira su un thread a sé; l'interfaccia legge con [`get`].

use std::sync::Mutex;

/// Fase del calcolo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Celle occupate dagli oggetti (collisioni).
    Obstacles,
    /// Simulazione fluida.
    Simulation,
    /// Dettaglio sulla griglia fine.
    Detail,
}

/// Fase, frazione fatta (0–1) e numero della nebbia (1 = prima) su quante.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Progress {
    pub phase: Phase,
    pub fraction: f32,
    pub fog: usize,
    pub fogs: usize,
}

static STATE: Mutex<Option<Progress>> = Mutex::new(None);

/// Stato attuale (None = nessun calcolo in corso).
pub fn get() -> Option<Progress> {
    *STATE.lock().unwrap()
}

/// Nuova fase della nebbia `fog` su `fogs`.
pub fn begin(phase: Phase, fog: usize, fogs: usize) {
    *STATE.lock().unwrap() = Some(Progress {
        phase,
        fraction: 0.0,
        fog,
        fogs,
    });
}

/// Avanzamento della fase in corso.
pub fn set(phase: Phase, fraction: f32) {
    let mut s = STATE.lock().unwrap();
    match s.as_mut() {
        Some(p) => {
            p.phase = phase;
            p.fraction = fraction.clamp(0.0, 1.0);
        }
        None => {
            *s = Some(Progress {
                phase,
                fraction: fraction.clamp(0.0, 1.0),
                fog: 1,
                fogs: 1,
            })
        }
    }
}

/// Calcolo finito.
pub fn clear() {
    *STATE.lock().unwrap() = None;
}
