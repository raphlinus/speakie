use std::io::stdin;

use clap::Parser;
use speakie::{BitStream, Speakie};

#[derive(Parser)]
struct Args {
    hex: Option<String>,
    #[arg(short, long)]
    input_file: Option<String>,
    #[arg(short, long)]
    output_file: String,
}

fn parse_hex(inp: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut result = vec![];
    let inp1 = inp.trim();
    let inp2 = inp1.replace(',', " ");
    let inp3 = inp2.strip_prefix("[").unwrap_or(&inp2);
    let inp4 = inp3.strip_suffix("]").unwrap_or(&inp3);
    for word in inp4.split_ascii_whitespace() {
        if word.is_empty() {
            continue;
        }
        let word2 = word.strip_prefix("0x").unwrap_or(&word);
        let byte = u8::from_str_radix(word2, 16)?;
        result.push(byte);
    }
    Ok(result)
}

impl Args {
    fn get_hex(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        if let Some(hex) = &self.hex {
            parse_hex(hex)
        } else if let Some(input_file) = &self.input_file {
            let hex = std::fs::read_to_string(input_file)?;
            parse_hex(&hex)
        } else {
            let mut hex = String::new();
            stdin().read_line(&mut hex)?;
            parse_hex(&hex)
        }
    }
}

fn main() {
    let args = Args::parse();
    let lpc_encoded =args.get_hex().expect("error parsing hex");

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 8000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(&args.output_file, spec).unwrap();
    let mut bs = BitStream::new(&lpc_encoded);
    let mut speakie = Speakie::new();
    while !speakie.process_frame(&mut bs) {
        for _ in 0..200 {
            let sample = speakie.get_sample();
            writer.write_sample(sample).unwrap();
        }
    }
    writer.finalize().unwrap();
}
