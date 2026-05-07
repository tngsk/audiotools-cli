use anyhow::{Context, Result};
use hound::{WavReader, WavSpec};
use rodio::{Decoder, Source};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

pub fn load_audio(path: &Path) -> Result<(Vec<f32>, u32)> {
    if let Ok(reader) = WavReader::open(path) {
        return load_wav(reader);
    }
    load_generic(path)
}

fn load_wav(reader: WavReader<BufReader<File>>) -> Result<(Vec<f32>, u32)> {
    let spec = reader.spec();
    let sample_rate = spec.sample_rate;
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .into_samples::<f32>()
            .collect::<Result<Vec<_>, _>>()?,
        hound::SampleFormat::Int => {
            let bits = spec.bits_per_sample;
            let max_val = 2.0_f32.powi(bits as i32 - 1);
            let inv_max_val = 1.0 / max_val;
            reader
                .into_samples::<i32>()
                .map(|s| s.map(|x| x as f32 * inv_max_val))
                .collect::<Result<Vec<_>, _>>()?
        }
    };

    let channels = spec.channels as usize;
    if channels > 1 {
        let inv_channels = 1.0 / channels as f32;
        let mono_samples = samples
            .chunks(channels)
            .map(|chunk| chunk.iter().sum::<f32>() * inv_channels)
            .collect();
        return Ok((mono_samples, sample_rate));
    }

    Ok((samples, sample_rate))
}

fn load_generic(path: &Path) -> Result<(Vec<f32>, u32)> {
    let file = File::open(path).with_context(|| format!("Failed to open file: {:?}", path))?;
    let source = Decoder::new(BufReader::new(file)).with_context(|| "Failed to decode audio")?;

    let sample_rate = source.sample_rate();
    let channels = source.channels();
    let samples: Vec<f32> = source.convert_samples().collect();

    if channels > 1 {
        let inv_channels = 1.0 / channels as f32;
        let mono_samples = samples
            .chunks(channels as usize)
            .map(|chunk| chunk.iter().sum::<f32>() * inv_channels)
            .collect();
        return Ok((mono_samples, sample_rate));
    }

    Ok((samples, sample_rate))
}

pub fn save_wav(path: &Path, samples: &[f32], sample_rate: u32) -> Result<()> {
    let spec = WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut writer = hound::WavWriter::create(path, spec)?;
    for &sample in samples {
        writer.write_sample(sample)?;
    }
    writer.finalize()?;
    Ok(())
}
