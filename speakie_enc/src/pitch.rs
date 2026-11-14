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
                best_period as f64 + delta
            } else {
                best_period as f64
            }
        }
    }

    /// Up to `k` strongest local maxima of the normalized autocorrelation,
    /// as (period, strength), with parabolic interpolation of the period.
    pub fn candidates(&self, k: usize) -> Vec<(f64, f64)> {
        let mut peaks = Vec::new();
        for p in self.min_period..=self.max_period {
            let c = self.coeffs[p];
            if c > 0.0 && c >= self.coeffs[p - 1] && c > self.coeffs[p + 1] {
                peaks.push((self.interpolated(p), c));
            }
        }
        peaks.sort_by(|a, b| b.1.total_cmp(&a.1));
        peaks.truncate(k);
        peaks
    }
}

/// Parameters for `track`. Costs are per frame and follow Boersma's
/// (Praat) formulation, scaled for the frame rate.
pub struct TrackConfig {
    pub min_period: usize,
    pub max_period: usize,
    pub window: usize,
    pub hop: usize,
    /// Strength of the unvoiced candidate; voiced candidates must beat this.
    pub voicing_threshold: f64,
    /// Cost per octave of pitch change between adjacent frames.
    pub octave_jump_cost: f64,
    /// Strength bonus per octave above the lowest pitch, so a doubled
    /// period doesn't win on a near tie.
    pub octave_cost: f64,
    /// Cost of switching between voiced and unvoiced.
    pub voiced_unvoiced_cost: f64,
    /// Frames whose rms is below this fraction of the loudest frame's rms
    /// get a boost to the unvoiced candidate.
    pub silence_threshold: f64,
    pub max_candidates: usize,
}

impl Default for TrackConfig {
    fn default() -> Self {
        // Praat defaults are 0.35 / 0.14 at a 10 ms step; with 25 ms
        // frames, scale them by 0.4 so the cost per unit time is the same.
        Self {
            min_period: 16,
            max_period: 160,
            window: 300,
            hop: 200,
            voicing_threshold: 0.45,
            octave_jump_cost: 0.14,
            octave_cost: 0.01,
            voiced_unvoiced_cost: 0.056,
            silence_threshold: 0.03,
            max_candidates: 6,
        }
    }
}

/// Track pitch over frames starting at multiples of `cfg.hop`, using
/// dynamic programming over autocorrelation candidates. `x` should be the
/// low-passed signal. Returns a period per frame (0 for unvoiced).
pub fn track(x: &[f64], cfg: &TrackConfig) -> Vec<f64> {
    let n_frames = x.len().div_ceil(cfg.hop);
    let slice = |start: usize| -> Vec<f64> {
        (0..cfg.window)
            .map(|j| x.get(start + j).copied().unwrap_or_default())
            .collect()
    };
    let rms: Vec<f64> = (0..n_frames)
        .map(|i| {
            let s = slice(i * cfg.hop);
            (s.iter().map(|v| v * v).sum::<f64>() / s.len() as f64).sqrt()
        })
        .collect();
    let global = rms.iter().cloned().fold(0.0, f64::max).max(1e-9);
    // Candidate 0 in each frame is unvoiced (period 0).
    let cands: Vec<Vec<(f64, f64)>> = (0..n_frames)
        .map(|i| {
            let local = rms[i] / global;
            let unvoiced = cfg.voicing_threshold
                + (2.0 - local / (cfg.silence_threshold / (1.0 + cfg.voicing_threshold))).max(0.0);
            let mut c = vec![(0.0, unvoiced)];
            if rms[i] > 0.0 {
                let s = slice(i * cfg.hop);
                let est = PitchEstimator::new(&s, cfg.min_period, cfg.max_period);
                c.extend(
                    est.candidates(cfg.max_candidates)
                        .into_iter()
                        .map(|(p, r)| {
                            (p, r + cfg.octave_cost * (cfg.max_period as f64 / p).log2())
                        }),
                );
            }
            c
        })
        .collect();
    let trans = |a: f64, b: f64| -> f64 {
        match (a > 0.0, b > 0.0) {
            (false, false) => 0.0,
            (true, true) => cfg.octave_jump_cost * (a / b).log2().abs(),
            _ => cfg.voiced_unvoiced_cost,
        }
    };
    // Viterbi, maximizing strength minus transition cost.
    let mut score: Vec<f64> = cands[0].iter().map(|c| c.1).collect();
    let mut back: Vec<Vec<usize>> = vec![vec![0; cands[0].len()]];
    for i in 1..n_frames {
        let mut new_score = Vec::with_capacity(cands[i].len());
        let mut new_back = Vec::with_capacity(cands[i].len());
        for c in &cands[i] {
            let (best_j, best) = cands[i - 1]
                .iter()
                .enumerate()
                .map(|(j, p)| (j, score[j] - trans(p.0, c.0)))
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .unwrap();
            new_score.push(best + c.1);
            new_back.push(best_j);
        }
        score = new_score;
        back.push(new_back);
    }
    let mut j = score
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .unwrap()
        .0;
    let mut out = vec![0.0; n_frames];
    for i in (0..n_frames).rev() {
        out[i] = cands[i][j].0;
        j = back[i][j];
    }
    out
}
