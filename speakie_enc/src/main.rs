use std::f64::consts::PI;

use clap::Parser;
use pitch_detection::detector::PitchDetector;

use crate::output::Output;

mod output;

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
    let samples = reader.samples().map(|s| s.unwrap()).collect::<Vec<i16>>();
    let bytes = to_lpc(&samples);
    println!("{:?}", bytes);
}

const FRAME_SIZE: usize = 200;
const WINDOW_SIZE: usize = 400;

fn to_lpc(samples: &[i16]) -> Vec<u8> {
    let mut out = Output::default();
    let hw = hamming_window();
    let n_frames = samples.len().div_ceil(FRAME_SIZE);
    for i in 0..n_frames {
        let base = i * FRAME_SIZE;
        let windowed = (0..WINDOW_SIZE).map(|i|
            samples.get(base + i).cloned().unwrap_or_default() as f64 * hw[i]
        ).collect::<Vec<_>>();
        let rms = windowed.iter().map(|x| x * x).sum::<f64>().sqrt();
        let filtered = lpf(&windowed, 0.1);
        let pitch = pitch_detection::detector::mcleod::McLeodDetector::new(WINDOW_SIZE, 0)
            .get_pitch(&filtered, EXPECTED_SAMPLE_RATE as usize, 1.0, 0.3);
        let period = match pitch {
            Some(pitch) => {
                let period = EXPECTED_SAMPLE_RATE as f64 / pitch.frequency;
                if period >= 16. && period <= 160. {
                    period
                } else {
                    0.0
                }
            }
            _ => 0.0,
        };
        let data = ndarray::Array1::from_iter(preemph(&windowed, 0.9375));
        let lpc = linear_predictive_coding::calc_lpc_by_burg(data.view(), 10).unwrap();
        out.frame(0.01 * rms, period, lpc);
    }
    out.pack(15, 4);
    out.pack(0, 7);
    out.reap()
}

fn hamming_window() -> Vec<f64> {
    (0..WINDOW_SIZE).map(|i|
        0.54 - 0.46 * (2. * PI * i as f64 / (WINDOW_SIZE - 1) as f64).cos()
    ).collect()
}

fn lpf(inp: &[f64], a: f64) -> Vec<f64> {
    let mut y = 0.0;
    inp.iter().map(|x| {
        y = (1. - a) * y + a * x;
        y
    }).collect()
}

fn preemph(inp: &[f64], a: f64) -> Vec<f64> {
    (0..inp.len()).map(|i|
        inp[i] - inp.get(i.wrapping_sub(1)).cloned().unwrap_or_default() * a
    ).collect()
}

