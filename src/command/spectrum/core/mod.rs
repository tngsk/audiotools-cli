pub mod audio;
pub mod analysis;
pub mod config;

use std::path::Path;
use crate::command::spectrum::domain::audio_data::AudioData;
use crate::command::spectrum::domain::spectrogram::Spectrogram;
use crate::command::spectrum::error::SpectrumError;

pub trait AudioLoader {
    fn load(&self, path: &Path) -> Result<AudioData, SpectrumError>;
}

pub trait SpectralAnalyzer {
    fn analyze(&self, samples: &[f32]) -> Result<(Vec<Vec<f32>>, Vec<f32>, Vec<f32>), SpectrumError>;
}

pub trait SpectrogramRenderer {
    fn render(&self, spectrogram: &Spectrogram, output: &Path) -> Result<(), SpectrumError>;
}