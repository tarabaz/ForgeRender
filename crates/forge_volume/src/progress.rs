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
    /// Foschia d'ambiente (interno degli oggetti, poi densità).
    Haze,
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

/// Avanzamento della fase in corso, solo se c'è un calcolo in corso (un calcolo annullato
/// non fa ricomparire la barra). Si può chiamare da qualsiasi thread.
pub fn update(fraction: f32) {
    if let Some(p) = STATE.lock().unwrap().as_mut() {
        p.fraction = fraction.clamp(0.0, 1.0);
    }
}

/// Calcolo finito.
pub fn clear() {
    *STATE.lock().unwrap() = None;
}

/// Numero del calcolo attuale: annullare lo fa avanzare, e un calcolo che aveva il numero di
/// prima si ferma. Così un calcolo nuovo partito subito dopo non viene fermato.
static GEN: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

thread_local! {
    static MINE: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Il calcolo che parte su questo thread è quello attuale.
pub fn start_job() {
    MINE.with(|m| m.set(GEN.load(std::sync::atomic::Ordering::SeqCst)));
}

/// Ferma il calcolo in corso (annullato dall'utente, o la nebbia è cambiata): la simulazione si
/// ferma al passo successivo e il risultato non si conserva.
pub fn cancel() {
    GEN.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    *STATE.lock().unwrap() = None;
}

/// Il calcolo di questo thread è stato annullato?
pub fn cancelled() -> bool {
    MINE.with(|m| m.get()) != GEN.load(std::sync::atomic::Ordering::SeqCst)
}
