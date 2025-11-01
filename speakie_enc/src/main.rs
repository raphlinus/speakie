use std::f64::consts::PI;

use clap::Parser;

use crate::{output::Output, pitch::PitchEstimator, reflector::Reflector};

mod filter;
mod output;
mod pitch;
mod reflector;

#[derive(Parser, Debug)]
struct Args {
    filename: String,
}

const EXPECTED_SAMPLE_RATE: u32 = 8000;

fn main() {
    let args = Args::parse();
    let mut reader = hound::WavReader::open(&args.filename).expect("error opening input file");
    let spec = reader.spec();
    if spec.sample_rate != EXPECTED_SAMPLE_RATE {
        println!("Warning: sample rate is not {EXPECTED_SAMPLE_RATE}");
    }
    if spec.channels != 1 {
        panic!("Warning: can't handle stereo file yet");
    }
    if spec.sample_format != hound::SampleFormat::Int {
        panic!("Input WAV file must be in integer format");
    }
    let samples = reader
        .samples::<i16>()
        .map(|s| s.unwrap() as f64)
        .collect::<Vec<_>>();
    let bytes = to_lpc(&samples);
    println!("{:?}", bytes);
}

const FRAME_SIZE: usize = 200;
const WINDOW_SIZE: usize = 300;

fn to_lpc(samples: &[f64]) -> Vec<u8> {
    let mut out = Output::default();
    let hw = hamming_window();
    let n_frames = samples.len().div_ceil(FRAME_SIZE);
    let filtered = filter::lowpass(samples);
    for i in 0..n_frames {
        let base = i * FRAME_SIZE;
        let filtered_slice = (0..WINDOW_SIZE)
            .map(|i| filtered.get(base + i).cloned().unwrap_or_default())
            .collect::<Vec<_>>();
        let mut period = PitchEstimator::new(&filtered_slice, 16, 160).estimate();
        const VOICED_THRESH: f64 = 0.25;
        if reflector::confidence(&filtered_slice, period.round() as usize) < VOICED_THRESH {
            period = 0.0;
        }
        let alpha = if period == 0.0 { 0.0 } else { 0.9375 };
        let windowed = (0..WINDOW_SIZE)
            .map(|i| samples.get(base + i).cloned().unwrap_or_default() * hw[i])
            .collect::<Vec<_>>();
        let preemphed = preemph(&windowed, alpha);
        let reflector = Reflector::new(&preemphed);
        let mut rms = reflector.rms();
        if reflector.is_unvoiced() {
            period = 0.0;
        }
        if period == 0.0 {
            rms *= 0.25;
        }
        out.frame(0.01 * rms, period, &reflector.ks()[1..]);
    }
    out.pack(15, 4);
    out.pack(0, 7);
    out.reap()
}

fn hamming_window() -> Vec<f64> {
    (0..WINDOW_SIZE)
        .map(|i| 0.54 - 0.46 * (2. * PI * i as f64 / (WINDOW_SIZE - 1) as f64).cos())
        .collect()
}

fn preemph(inp: &[f64], a: f64) -> Vec<f64> {
    (0..inp.len())
        .map(|i| inp[i] - inp.get(i.wrapping_sub(1)).cloned().unwrap_or_default() * a)
        .collect()
}
