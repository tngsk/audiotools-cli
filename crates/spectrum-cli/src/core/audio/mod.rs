pub mod processor;

use crate::core::AudioLoader;
use crate::domain::audio_data::AudioData;
use crate::error::SpectrumError;
use hound::WavReader;
use std::path::Path;

pub struct DefaultAudioLoader;

impl AudioLoader for DefaultAudioLoader {
    fn load(&self, input: &Path) -> Result<AudioData, SpectrumError> {
        let mut reader = WavReader::open(input)?;
        let spec = reader.spec();
        let sample_rate = spec.sample_rate as f32;
        let duration = reader.duration() as f32 / sample_rate;
        let channels = spec.channels as u32;

        // Convert to mono f32 samples
        let inv_channels = 1.0 / spec.channels as f32;
        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Float => {
                let samples: std::result::Result<Vec<f32>, hound::Error> =
                    reader.samples::<f32>().collect();
                samples?
                    .chunks(spec.channels as usize)
                    .map(|chunk| chunk.iter().sum::<f32>() * inv_channels)
                    .collect()
            }
            hound::SampleFormat::Int => {
                let bits = spec.bits_per_sample;
                let max_value = (1 << (bits - 1)) as f32;
                let inv_max_channels = 1.0 / (spec.channels as f32 * max_value);

                let samples: std::result::Result<Vec<i32>, hound::Error> =
                    reader.samples::<i32>().collect();
                samples?
                    .chunks(spec.channels as usize)
                    .map(|chunk| chunk.iter().sum::<i32>() as f32 * inv_max_channels)
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
