pub mod processor;

use std::path::Path;
use hound::WavReader;
use crate::command::spectrum::domain::audio_data::AudioData;
use crate::command::spectrum::error::SpectrumError;
use crate::command::spectrum::core::AudioLoader;

pub struct DefaultAudioLoader;

impl AudioLoader for DefaultAudioLoader {
    fn load(&self, input: &Path) -> Result<AudioData, SpectrumError> {
        let mut reader = WavReader::open(input).map_err(|e| SpectrumError::AudioLoad(format!("Failed to open audio file: {}", e)))?;
        let spec = reader.spec();
        let sample_rate = spec.sample_rate as f32;
        let duration = reader.duration() as f32 / sample_rate;
        let channels = spec.channels as u32;

        // Convert to mono f32 samples
        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float => {
                let samples: std::result::Result<Vec<f32>, hound::Error> =
                    reader.samples::<f32>().collect();
                samples?
                    .chunks(spec.channels as usize)
                    .map(|chunk| chunk.iter().sum::<f32>() / chunk.len() as f32)
                    .collect()
            }
            hound::SampleFormat::Int => {
                let bits = spec.bits_per_sample;
                let max_value = (1 << (bits - 1)) as f32;

                let samples: std::result::Result<Vec<i32>, hound::Error> =
                    reader.samples::<i32>().collect();
                samples?
                    .chunks(spec.channels as usize)
                    .map(|chunk| chunk.iter().sum::<i32>() as f32 / (chunk.len() as f32 * max_value))
                    .collect()
            }
        };

        Ok(AudioData {
            samples,
            sample_rate,
            duration,
            channels,
        })
    }
}
