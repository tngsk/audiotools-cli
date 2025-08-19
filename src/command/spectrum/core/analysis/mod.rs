pub mod fft;
pub mod windowing;

use crate::command::spectrum::error::SpectrumError;
use crate::command::spectrum::core::SpectralAnalyzer;
use crate::command::spectrum::core::config::SpectrogramConfig;

pub struct DefaultSpectralAnalyzer {
    config: SpectrogramConfig,
}

impl DefaultSpectralAnalyzer {
    pub fn new(config: SpectrogramConfig) -> Self {
        Self { config }
    }
}

impl SpectralAnalyzer for DefaultSpectralAnalyzer {
    // Returns (spectrogram_data, time_axis, freq_axis)
    fn analyze(&self, samples: &[f32]) -> Result<(Vec<Vec<f32>>, Vec<f32>, Vec<f32>), SpectrumError> {
        let fft_processor = fft::FFTProcessor::new(self.config.clone());
        let spectrogram_data = if self.config.analysis_duration_ms < 150.0 {
            fft_processor.process_signal_with_padding(samples, 0.3)?
        } else if self.config.analysis_duration_ms < 300.0 {
            fft_processor.process_signal_with_padding(samples, 0.2)?
        } else if self.config.analysis_duration_ms < 500.0 {
            fft_processor.process_signal_with_padding(samples, 0.1)?
        } else {
            fft_processor.process_signal(samples)?
        };

        let time_axis = (0..spectrogram_data.len())
            .map(|i| i as f32 * self.config.time_resolution())
            .collect();
        let freq_axis = (0..self.config.freq_bins())
            .map(|i| i as f32 * self.config.freq_resolution())
            .collect();

        Ok((spectrogram_data, time_axis, freq_axis))
    }
}