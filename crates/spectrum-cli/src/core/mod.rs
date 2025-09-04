pub mod analysis;
pub mod audio;
pub mod config;

use crate::domain::audio_data::AudioData;
use crate::domain::spectrogram::Spectrogram;
use crate::error::SpectrumError;
use std::path::Path;

pub trait AudioLoader {
    fn load(&self, path: &Path) -> Result<AudioData, SpectrumError>;
}

pub trait SpectralAnalyzer {
    fn analyze(
        &self,
        samples: &[f32],
    ) -> Result<(Vec<Vec<f32>>, Vec<f32>, Vec<f32>), SpectrumError>;
}

pub trait SpectrogramRenderer {
    fn render(&self, spectrogram: &Spectrogram, output: &Path) -> Result<(), SpectrumError>;
}
