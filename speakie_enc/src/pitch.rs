pub struct PitchEstimator {
    min_period: usize,
    max_period: usize,
    coeffs: Vec<f64>,
}

fn get_normalized_coefficients(buf: &[f64], min_period: usize, max_period: usize) -> Vec<f64> {
    let mut coefficients = vec![0.0; max_period + 1];
    for lag in min_period..=max_period {
        let mut sos_beginning = 0.0;
        let mut sos_end = 0.0;
        let mut sum = 0.0;
        let samples = buf.len() - lag;
        for i in 0..samples {
            sum += buf[i] * buf[i + lag];
            sos_beginning += buf[i].powi(2);
            sos_end += buf[i + lag].powi(2);
        }
        coefficients[lag] = sum / (sos_beginning * sos_end).sqrt();
    }
    coefficients
}

impl PitchEstimator {
    pub fn new(buf: &[f64], min_period: usize, max_period: usize) -> Self {
        let coeffs = get_normalized_coefficients(buf, min_period - 1, max_period + 1);
        Self {
            coeffs,
            min_period,
            max_period,
        }
    }

    fn best_period(&self) -> usize {
        let mut best_period = self.min_period;
        for period in best_period + 1..self.max_period {
            if self.coeffs[period] > self.coeffs[best_period] {
                best_period = period;
            }
        }
        best_period
    }

    fn interpolated(&self, best_period: usize) -> f64 {
        let middle = self.coeffs[best_period];
        let left = self.coeffs[best_period - 1];
        let right = self.coeffs[best_period + 1];
        let dd = 2. * middle - left - right;
        if dd == 0.0 {
            best_period as f64
        } else {
            let delta = 0.5 * (right - left) / dd;
            if delta.abs() < 0.5 {
                best_period as f64 + dd
            } else {
                best_period as f64
            }
        }
    }

    pub fn estimate(&self) -> f64 {
        let best_period = self.best_period();
        let best = self.coeffs[best_period];
        if best < self.coeffs[best_period - 1] || best < self.coeffs[best_period + 1] {
            return 0.0;
        }
        let mut max_multiple = best_period / self.min_period;
        let estimate = self.interpolated(best_period);
        const SUB_MULTIPLE_THRESHOLD: f64 = 0.9;
        let thresh = SUB_MULTIPLE_THRESHOLD * self.coeffs[best_period];
        while max_multiple >= 1 {
            let candidate = estimate / max_multiple as f64;
            let mut sub_multiples_strong = true;
            for i in 0..max_multiple {
                let sub_multiple_period = ((i + 1) as f64 * candidate).round() as usize;
                if self.coeffs[sub_multiple_period] != 0.0
                    && self.coeffs[sub_multiple_period] < thresh
                {
                    sub_multiples_strong = false;
                }
            }
            if sub_multiples_strong {
                return candidate;
            }
            max_multiple -= 1;
        }
        estimate
    }
}
