use rayon::prelude::*;

/// Buffer di accumulo: somma dei campioni di radianza per pixel.
/// L'immagine corrente è `sum / samples`. Ogni passata aggiunge esattamente
/// un campione a **tutti** i pixel, quindi il conteggio è globale.
#[derive(Clone, Debug)]
pub struct Accumulator {
    pub width: u32,
    pub height: u32,
    pub samples: u32,
    sum: Vec<[f32; 3]>,
}

impl Accumulator {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            samples: 0,
            sum: vec![[0.0; 3]; (width as usize) * (height as usize)],
        }
    }

    /// Ricostruisce lo stato da una somma salvata (ripresa del render).
    pub fn from_parts(width: u32, height: u32, samples: u32, sum: Vec<[f32; 3]>) -> Option<Self> {
        (sum.len() == width as usize * height as usize).then_some(Self {
            width,
            height,
            samples,
            sum,
        })
    }

    pub fn pixel_count(&self) -> usize {
        self.sum.len()
    }

    pub fn sum(&self) -> &[[f32; 3]] {
        &self.sum
    }

    /// Aggiunge una passata completa (1 campione per pixel).
    pub fn add_pass(&mut self, pass: &[[f32; 3]]) {
        assert_eq!(pass.len(), self.sum.len());
        self.sum.par_iter_mut().zip(pass.par_iter()).for_each(|(s, p)| {
            // Un campione non finito (NaN/Inf) corromperebbe il pixel per sempre.
            if p.iter().all(|v| v.is_finite()) {
                s[0] += p[0];
                s[1] += p[1];
                s[2] += p[2];
            }
        });
        self.samples += 1;
    }

    /// Media corrente (radianza lineare).
    pub fn resolve(&self) -> Vec<[f32; 3]> {
        let inv = if self.samples > 0 { 1.0 / self.samples as f32 } else { 0.0 };
        self.sum.par_iter().map(|s| [s[0] * inv, s[1] * inv, s[2] * inv]).collect()
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
    }
}
