#[derive(Default)]
pub struct Reflector {
    ks: [f64; 11],
    rms: f64,
}

fn get_correlations(buf: &[f64]) -> [f64; 11] {
    core::array::from_fn(|lag| {
        let mut sum = 0.0;
        for i in 0..buf.len() - lag {
            sum += buf[i] * buf[i + lag]
        }
        sum
    })
}

impl Reflector {
    pub fn new(buf: &[f64]) -> Self {
        let coeffs = get_correlations(buf);
        let mut result = Self::default();
        result.translate_coeffs(&coeffs, buf.len());
        result
    }

    fn translate_coeffs(&mut self, coeffs: &[f64], n_samples: usize) {
        let mut b = [0.0; 11];
        let mut d = [0.0; 12];
        self.ks[1] = -coeffs[1] / coeffs[0];
        d[1] = coeffs[1];
        d[2] = coeffs[0] + (self.ks[1] * coeffs[1]);

        for i in 2..=10 {
            let mut y = coeffs[i];
            b[1] = y;
            for j in 1..i {
                b[j + 1] = d[j] + self.ks[j] * y;
                y += self.ks[j] * d[j];
                d[j] = b[j];
            }
            self.ks[i] = -y / d[i];
            d[i + 1] = d[i] + self.ks[i] * y;
            d[i] = b[i];
        }
        self.rms = d[11] / n_samples as f64 * 32768.0;
    }

    pub fn is_unvoiced(&self) -> bool {
        const UNVOICED_THRESHOLD: f64 = 0.3;
        self.ks[1] > UNVOICED_THRESHOLD
    }

    pub fn ks(&self) -> &[f64] {
        &self.ks
    }
}
