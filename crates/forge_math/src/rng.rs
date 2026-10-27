/// Generatore PCG32 (O'Neill 2014). Piccolo, veloce e deterministico:
/// lo stesso seed produce sempre la stessa sequenza, indipendentemente da
/// quale thread esegue il calcolo. Questo rende i render riproducibili.
#[derive(Clone, Debug)]
pub struct Pcg32 {
    state: u64,
    inc: u64,
}

impl Pcg32 {
    const MULT: u64 = 6364136223846793005;

    pub fn new(seed: u64, stream: u64) -> Self {
        let mut rng = Self {
            state: 0,
            inc: (stream << 1) | 1,
        };
        rng.next_u32();
        rng.state = rng.state.wrapping_add(seed);
        rng.next_u32();
        rng
    }

    /// RNG per un pixel e un indice di campione: la sequenza dipende solo da
    /// `(seed, x, y, sample)`, non dall'ordine di esecuzione.
    pub fn for_pixel(seed: u64, x: u32, y: u32, sample: u32) -> Self {
        let pixel = ((y as u64) << 32) | x as u64;
        let s = splitmix64(seed ^ splitmix64(pixel ^ splitmix64(sample as u64)));
        Self::new(s, splitmix64(s ^ 0x9E37_79B9_7F4A_7C15))
    }

    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(Self::MULT).wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// Float uniforme in [0, 1).
    #[inline]
    pub fn next_f32(&mut self) -> f32 {
        // 24 bit di mantissa: risultato sempre < 1.0.
        (self.next_u32() >> 8) as f32 * (1.0 / 16_777_216.0)
    }

    #[inline]
    pub fn next_2d(&mut self) -> (f32, f32) {
        (self.next_f32(), self.next_f32())
    }
}

#[inline]
pub(crate) fn splitmix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_in_range() {
        let mut a = Pcg32::for_pixel(7, 10, 20, 3);
        let mut b = Pcg32::for_pixel(7, 10, 20, 3);
        let mut sum = 0.0f64;
        for _ in 0..10_000 {
            let x = a.next_f32();
            assert_eq!(x, b.next_f32());
            assert!((0.0..1.0).contains(&x));
            sum += x as f64;
        }
        let mean = sum / 10_000.0;
        assert!((mean - 0.5).abs() < 0.02, "media {mean}");
    }

    #[test]
    fn different_pixels_differ() {
        let mut a = Pcg32::for_pixel(7, 10, 20, 3);
        let mut b = Pcg32::for_pixel(7, 11, 20, 3);
        assert_ne!(a.next_u32(), b.next_u32());
    }
}
