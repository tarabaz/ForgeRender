use rayon::prelude::*;

/// Buffer di accumulo: somma dei campioni di radianza per pixel, con il numero di
/// campioni di ogni pixel (col campionamento adattivo i pixel già puliti si fermano).
/// L'immagine corrente è `sum / count` pixel per pixel.
///
/// Per stimare il rumore si tiene anche una seconda somma "B" con le sole passate di
/// indice pari (circa metà dei campioni, indipendente dal resto): la differenza tra la
/// media di B e la media complessiva misura quanto l'immagine è ancora rumorosa.
#[derive(Clone, Debug)]
pub struct Accumulator {
    pub width: u32,
    pub height: u32,
    /// Campioni dei pixel ancora attivi (il massimo per pixel).
    pub samples: u32,
    sum: Vec<[f32; 3]>,
    count: Vec<u32>,
    sum_b: Vec<[f32; 3]>,
    count_b: Vec<u32>,
    passes: u32,
}

impl Accumulator {
    pub fn new(width: u32, height: u32) -> Self {
        let n = (width as usize) * (height as usize);
        Self {
            width,
            height,
            samples: 0,
            sum: vec![[0.0; 3]; n],
            count: vec![0; n],
            sum_b: vec![[0.0; 3]; n],
            count_b: vec![0; n],
            passes: 0,
        }
    }

    /// Ricostruisce lo stato da una somma salvata (ripresa del render): tutti i pixel con
    /// `samples` campioni, senza la stima del rumore (si ricostruisce con le passate nuove).
    pub fn from_parts(width: u32, height: u32, samples: u32, sum: Vec<[f32; 3]>) -> Option<Self> {
        let n = width as usize * height as usize;
        (sum.len() == n).then(|| Self {
            width,
            height,
            samples,
            sum,
            count: vec![samples; n],
            sum_b: vec![[0.0; 3]; n],
            count_b: vec![0; n],
            passes: 0,
        })
    }

    pub fn pixel_count(&self) -> usize {
        self.sum.len()
    }

    pub fn sum(&self) -> &[[f32; 3]] {
        &self.sum
    }

    /// Campioni di ogni pixel.
    pub fn counts(&self) -> &[u32] {
        &self.count
    }

    /// Campioni calcolati in totale (somma su tutti i pixel).
    pub fn total_samples(&self) -> u64 {
        self.count.iter().map(|&c| c as u64).sum()
    }

    /// Aggiunge una passata completa (1 campione per pixel).
    pub fn add_pass(&mut self, pass: &[[f32; 3]]) {
        self.add_samples(pass, 1);
    }

    /// Aggiunge la somma di `n` campioni per pixel a tutti i pixel.
    pub fn add_samples(&mut self, pass: &[[f32; 3]], n: u32) {
        self.add_samples_masked(pass, n, None);
    }

    /// Aggiunge la somma di `n` campioni per pixel ai soli pixel attivi della maschera
    /// (tutti se `mask` è `None`); i pixel fermi restano come sono.
    pub fn add_samples_masked(&mut self, pass: &[[f32; 3]], n: u32, mask: Option<&crate::adaptive::TileMask>) {
        assert_eq!(pass.len(), self.sum.len());
        if n == 0 {
            return;
        }
        let w = self.width as usize;
        let to_b = self.passes.is_multiple_of(2);
        self.sum
            .par_iter_mut()
            .zip(self.count.par_iter_mut())
            .zip(self.sum_b.par_iter_mut().zip(self.count_b.par_iter_mut()))
            .zip(pass.par_iter())
            .enumerate()
            .for_each(|(i, (((s, c), (sb, cb)), p))| {
                if mask.is_some_and(|m| !m.pixel_active(i % w, i / w)) {
                    return;
                }
                // Un campione non finito (NaN/Inf) corromperebbe il pixel per sempre.
                if p.iter().all(|v| v.is_finite()) {
                    s[0] += p[0];
                    s[1] += p[1];
                    s[2] += p[2];
                    if to_b {
                        sb[0] += p[0];
                        sb[1] += p[1];
                        sb[2] += p[2];
                    }
                }
                *c += n;
                if to_b {
                    *cb += n;
                }
            });
        self.samples += n;
        self.passes += 1;
    }

    /// Media corrente (radianza lineare).
    pub fn resolve(&self) -> Vec<[f32; 3]> {
        self.sum
            .par_iter()
            .zip(self.count.par_iter())
            .map(|(s, &c)| {
                let inv = if c > 0 { 1.0 / c as f32 } else { 0.0 };
                [s[0] * inv, s[1] * inv, s[2] * inv]
            })
            .collect()
    }

    /// Errore relativo stimato di ogni pixel: |media − media di B| (somma dei canali)
    /// divisa per la radice della luminosità, come in Cycles (il rumore si vede di più
    /// nelle zone scure). `None` per i pixel senza campioni in B.
    pub fn pixel_error(&self, i: usize) -> Option<f32> {
        let (c, cb) = (self.count[i], self.count_b[i]);
        if c == 0 || cb == 0 || cb >= c {
            return None;
        }
        let (s, sb) = (self.sum[i], self.sum_b[i]);
        let a = [s[0] / c as f32, s[1] / c as f32, s[2] / c as f32];
        let b = [sb[0] / cb as f32, sb[1] / cb as f32, sb[2] / cb as f32];
        let diff = (a[0] - b[0]).abs() + (a[1] - b[1]).abs() + (a[2] - b[2]).abs();
        Some(diff / (1e-4 + (a[0] + a[1] + a[2]).max(0.0)).sqrt())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mean_is_exact() {
        let mut a = Accumulator::new(2, 1);
        a.add_pass(&[[1.0, 2.0, 3.0], [0.0; 3]]);
        a.add_pass(&[[3.0, 2.0, 1.0], [f32::NAN, 0.0, 0.0]]);
        let r = a.resolve();
        assert_eq!(r[0], [2.0, 2.0, 2.0]);
        assert_eq!(r[1], [0.0, 0.0, 0.0]);
        assert_eq!(a.samples, 2);
        assert_eq!(a.counts(), &[2, 2]);
    }
}
