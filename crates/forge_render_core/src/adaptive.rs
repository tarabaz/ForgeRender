//! Campionamento adattivo: i riquadri dell'immagine già abbastanza puliti smettono di
//! ricevere campioni, così il tempo va alle zone difficili (vetro, ombre morbide, nebbia).
//!
//! L'errore di un pixel è stimato dall'accumulatore ([`Accumulator::pixel_error`]):
//! differenza tra la media di tutti i campioni e quella della metà "B", relativa alla
//! luminosità. L'errore di un riquadro è la media sui suoi pixel; un riquadro si ferma
//! quando sta sotto la soglia e lo stanno anche i vicini (bordi senza scalini). Ogni
//! pixel resta una media non distorta dei suoi campioni: cambia solo quanti ne riceve.

use crate::Accumulator;
use rayon::prelude::*;

/// Lato dei riquadri in pixel.
pub const TILE: u32 = 16;

/// Riquadri ancora attivi (che ricevono campioni).
#[derive(Clone, Debug)]
pub struct TileMask {
    pub tiles_x: u32,
    pub tiles_y: u32,
    pub active: Vec<bool>,
}

impl TileMask {
    /// Tutti i riquadri attivi.
    pub fn all(width: u32, height: u32) -> Self {
        let (tiles_x, tiles_y) = (width.div_ceil(TILE).max(1), height.div_ceil(TILE).max(1));
        Self {
            tiles_x,
            tiles_y,
            active: vec![true; (tiles_x * tiles_y) as usize],
        }
    }

    #[inline]
    pub fn pixel_active(&self, x: usize, y: usize) -> bool {
        self.active[(y / TILE as usize) * self.tiles_x as usize + x / TILE as usize]
    }

    pub fn active_count(&self) -> usize {
        self.active.iter().filter(|&&a| a).count()
    }

    /// Frazione dei riquadri ancora attivi (0 = immagine finita).
    pub fn active_fraction(&self) -> f32 {
        self.active_count() as f32 / self.active.len().max(1) as f32
    }

    pub fn all_done(&self) -> bool {
        !self.active.iter().any(|&a| a)
    }

    /// Un bit per riquadro (per la GPU), a gruppi di 32.
    pub fn bits(&self) -> Vec<u32> {
        let mut w = vec![0u32; self.active.len().div_ceil(32).max(1)];
        for (i, &a) in self.active.iter().enumerate() {
            if a {
                w[i / 32] |= 1 << (i % 32);
            }
        }
        w
    }

    /// Ricalcola i riquadri attivi dall'errore stimato. Un riquadro fermo non riparte.
    pub fn update(&mut self, acc: &Accumulator, threshold: f32) {
        let (tx, ty) = (self.tiles_x as usize, self.tiles_y as usize);
        let (w, h) = (acc.width as usize, acc.height as usize);
        let t = TILE as usize;
        // Errore di ogni riquadro ancora attivo (∞ se non stimabile: resta attivo).
        let err: Vec<f32> = (0..tx * ty)
            .into_par_iter()
            .map(|k| {
                if !self.active[k] {
                    return 0.0;
                }
                let (bx, by) = (k % tx, k / tx);
                let mut sum = 0.0f64;
                let mut n = 0usize;
                for y in by * t..((by + 1) * t).min(h) {
                    for x in bx * t..((bx + 1) * t).min(w) {
                        match acc.pixel_error(y * w + x) {
                            Some(e) => sum += e as f64,
                            None => return f32::INFINITY,
                        }
                        n += 1;
                    }
                }
                if n == 0 {
                    0.0
                } else {
                    (sum / n as f64) as f32
                }
            })
            .collect();
        let noisy: Vec<bool> = err.iter().map(|&e| e > threshold).collect();
        // Resta attivo un riquadro rumoroso o confinante con uno rumoroso.
        for k in 0..tx * ty {
            if !self.active[k] {
                continue;
            }
            let (bx, by) = ((k % tx) as i64, (k / tx) as i64);
            let mut keep = false;
            for dy in -1..=1i64 {
                for dx in -1..=1i64 {
                    let (nx, ny) = (bx + dx, by + dy);
                    if nx >= 0 && ny >= 0 && (nx as usize) < tx && (ny as usize) < ty && noisy[ny as usize * tx + nx as usize] {
                        keep = true;
                    }
                }
            }
            self.active[k] = keep;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noisy_tiles_and_their_neighbours_stay_active() {
        // 64×16 pixel = 4 riquadri: il primo rumoroso (valori alterni), gli altri costanti.
        let (w, h) = (64u32, 16u32);
        let mut acc = Accumulator::new(w, h);
        for k in 0..32 {
            let pass: Vec<[f32; 3]> = (0..w * h)
                .map(|i| {
                    let x = i % w;
                    if x < 16 {
                        [if (k + i) % 2 == 0 { 2.0 } else { 0.0 }; 3]
                    } else {
                        [0.5; 3]
                    }
                })
                .collect();
            acc.add_pass(&pass);
        }
        let mut m = TileMask::all(w, h);
        m.update(&acc, 0.01);
        assert_eq!(m.active, vec![true, true, false, false], "il secondo resta attivo come vicino");
        // Una volta fermo un riquadro non riparte.
        m.active[1] = false;
        m.update(&acc, 0.01);
        assert!(!m.active[1]);
        assert_eq!(m.bits(), vec![0b0001]);
        assert!(m.pixel_active(3, 3) && !m.pixel_active(40, 3));
    }
}
