use crate::core::{AudioLoader, SpectralAnalyzer, SpectrogramRenderer};
use crate::domain::audio_data::AudioData;
use crate::domain::spectrogram::Spectrogram;
use crate::error::SpectrumError;
use std::path::Path;

/// Mock audio loader for testing
pub struct MockAudioLoader {
    sample_rate: f32,
    duration: f32,
    channels: u32,
    frequency: Option<f32>, // Generate sine wave if specified
}

impl MockAudioLoader {
    /// Create a new mock loader with default settings
    pub fn new() -> Self {
        Self {
            sample_rate: 44100.0,
            duration: 1.0,
            channels: 1,
            frequency: None,
        }
    }

    /// Set sample rate
    pub fn with_sample_rate(mut self, rate: f32) -> Self {
        self.sample_rate = rate;
        self
    }

    /// Set duration in seconds
    pub fn with_duration(mut self, duration: f32) -> Self {
        self.duration = duration;
        self
    }

    /// Set number of channels
    pub fn with_channels(mut self, channels: u32) -> Self {
        self.channels = channels;
        self
    }

    /// Generate sine wave at specified frequency
    pub fn with_sine_wave(mut self, frequency: f32) -> Self {
        self.frequency = Some(frequency);
        self
    }

    /// Generate test samples
    fn generate_samples(&self) -> Vec<f32> {
        let num_samples = (self.sample_rate * self.duration) as usize;

        match self.frequency {
            Some(freq) => {
                // Generate sine wave
                (0..num_samples)
                    .map(|i| {
                        let t = i as f32 / self.sample_rate;
                        (2.0 * std::f32::consts::PI * freq * t).sin()
                    })
                    .collect()
            }
            None => {
                // Generate silence
                vec![0.0; num_samples]
            }
        }
    }
}

impl AudioLoader for MockAudioLoader {
    fn load(&self, _path: &Path) -> Result<AudioData, SpectrumError> {
        let samples = self.generate_samples();

        Ok(AudioData {
            samples,
            sample_rate: self.sample_rate,
            duration: self.duration,
            channels: self.channels,
        })
    }
}

/// Mock audio loader that always fails
pub struct FailingAudioLoader {
    error_message: String,
}

impl FailingAudioLoader {
    pub fn new(error_message: impl Into<String>) -> Self {
        Self {
            error_message: error_message.into(),
        }
    }
}

impl AudioLoader for FailingAudioLoader {
    fn load(&self, _path: &Path) -> Result<AudioData, SpectrumError> {
        Err(SpectrumError::new(&self.error_message))
    }
}

/// Mock spectral analyzer for testing
pub struct MockSpectralAnalyzer {
    time_bins: usize,
    freq_bins: usize,
    generate_data: bool,
}

impl MockSpectralAnalyzer {
    pub fn new() -> Self {
        Self {
            time_bins: 100,
            freq_bins: 512,
            generate_data: true,
        }
    }

    pub fn with_dimensions(mut self, time_bins: usize, freq_bins: usize) -> Self {
        self.time_bins = time_bins;
        self.freq_bins = freq_bins;
        self
    }

    pub fn with_empty_data(mut self) -> Self {
        self.generate_data = false;
        self
    }
}

impl SpectralAnalyzer for MockSpectralAnalyzer {
    fn analyze(
        &self,
        _samples: &[f32],
    ) -> Result<(Vec<Vec<f32>>, Vec<f32>, Vec<f32>), SpectrumError> {
        let spectrogram_data = if self.generate_data {
            // Generate test pattern (gradient)
            (0..self.time_bins)
                .map(|t| {
                    (0..self.freq_bins)
                        .map(|f| {
                            // Create a diagonal gradient pattern
                            (t + f) as f32 / (self.time_bins + self.freq_bins) as f32
                        })
                        .collect()
                })
                .collect()
        } else {
            vec![vec![0.0; self.freq_bins]; self.time_bins]
        };

        let time_axis = (0..self.time_bins)
            .map(|i| i as f32 * 0.01) // 10ms per bin
            .collect();

        let freq_axis = (0..self.freq_bins)
            .map(|i| i as f32 * 20.0) // 20 Hz per bin
            .collect();

        Ok((spectrogram_data, time_axis, freq_axis))
    }
}

/// Mock spectrogram renderer for testing
pub struct MockSpectrogramRenderer {
    should_fail: bool,
    error_message: String,
    rendered_count: std::cell::RefCell<usize>,
}

impl MockSpectrogramRenderer {
    pub fn new() -> Self {
        Self {
            should_fail: false,
            error_message: String::new(),
            rendered_count: std::cell::RefCell::new(0),
        }
    }

    pub fn with_failure(mut self, error_message: impl Into<String>) -> Self {
        self.should_fail = true;
        self.error_message = error_message.into();
        self
    }

    pub fn render_count(&self) -> usize {
        *self.rendered_count.borrow()
    }
}

impl SpectrogramRenderer for MockSpectrogramRenderer {
    fn render(&self, _spectrogram: &Spectrogram, _output: &Path) -> Result<(), SpectrumError> {
        if self.should_fail {
            Err(SpectrumError::new(&self.error_message))
        } else {
            *self.rendered_count.borrow_mut() += 1;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::spectrogram::SpectrogramMetadata;
    use std::path::PathBuf;

    #[test]
    fn test_mock_audio_loader_defaults() {
        let loader = MockAudioLoader::new();
        let result = loader.load(&PathBuf::from("test.wav"));

        assert!(result.is_ok());
        let audio = result.unwrap();
        assert_eq!(audio.sample_rate, 44100.0);
        assert_eq!(audio.duration, 1.0);
        assert_eq!(audio.channels, 1);
        assert_eq!(audio.samples.len(), 44100);
    }

    #[test]
    fn test_mock_audio_loader_custom() {
        let loader = MockAudioLoader::new()
            .with_sample_rate(48000.0)
            .with_duration(0.5)
            .with_channels(2);

        let result = loader.load(&PathBuf::from("test.wav"));

        assert!(result.is_ok());
        let audio = result.unwrap();
        assert_eq!(audio.sample_rate, 48000.0);
        assert_eq!(audio.duration, 0.5);
        assert_eq!(audio.channels, 2);
        assert_eq!(audio.samples.len(), 24000);
    }

    #[test]
    fn test_mock_audio_loader_sine_wave() {
        let loader = MockAudioLoader::new()
            .with_sample_rate(1000.0) // Low sample rate for easy testing
            .with_duration(0.01) // 10ms
            .with_sine_wave(100.0); // 100 Hz

        let result = loader.load(&PathBuf::from("test.wav"));

        assert!(result.is_ok());
        let audio = result.unwrap();

        // Check that samples are not all zero
        let non_zero_count = audio.samples.iter().filter(|&&s| s != 0.0).count();
        assert!(non_zero_count > 0);

        // Check that samples are in range [-1, 1]
        assert!(audio.samples.iter().all(|&s| s >= -1.0 && s <= 1.0));
    }

    #[test]
    fn test_failing_audio_loader() {
        let loader = FailingAudioLoader::new("File not found");
        let result = loader.load(&PathBuf::from("test.wav"));

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("File not found"));
    }

    #[test]
    fn test_mock_spectral_analyzer() {
        let analyzer = MockSpectralAnalyzer::new().with_dimensions(50, 256);

        let samples = vec![0.0; 1000];
        let result = analyzer.analyze(&samples);

        assert!(result.is_ok());
        let (data, time_axis, freq_axis) = result.unwrap();

        assert_eq!(data.len(), 50); // time bins
        assert_eq!(data[0].len(), 256); // freq bins
        assert_eq!(time_axis.len(), 50);
        assert_eq!(freq_axis.len(), 256);
    }

    #[test]
    fn test_mock_spectral_analyzer_empty() {
        let analyzer = MockSpectralAnalyzer::new()
            .with_dimensions(10, 10)
            .with_empty_data();

        let samples = vec![0.0; 100];
        let result = analyzer.analyze(&samples);

        assert!(result.is_ok());
        let (data, _, _) = result.unwrap();

        // Check all data is zero
        assert!(data.iter().all(|row| row.iter().all(|&v| v == 0.0)));
    }

    #[test]
    fn test_mock_renderer_success() {
        use crate::core::config::SpectrogramConfig;
        use std::path::PathBuf;

        let renderer = MockSpectrogramRenderer::new();

        let spectrogram = Spectrogram {
            data: vec![vec![0.0; 10]; 10],
            time_axis: vec![0.0; 10],
            freq_axis: vec![0.0; 10],
            metadata: SpectrogramMetadata {
                config: SpectrogramConfig {
                    window_size: 1024,
                    hop_size: 256,
                    sample_rate: 44100.0,
                    min_freq: 20.0,
                    max_freq: 20000.0,
                    image_width: 800,
                    image_height: 600,
                    window_type: crate::core::config::WindowType::Hanning,
                    analysis_duration_ms: 1000.0,
                },
                input_path: PathBuf::from("test.wav"),
                start_time: 0.0,
                end_time: 1.0,
                analysis_duration_ms: 1000.0,
                annotations: vec![],
            },
        };

        let result = renderer.render(&spectrogram, &PathBuf::from("output.png"));
        assert!(result.is_ok());
        assert_eq!(renderer.render_count(), 1);
    }

    #[test]
    fn test_mock_renderer_failure() {
        use crate::core::config::SpectrogramConfig;
        use std::path::PathBuf;

        let renderer = MockSpectrogramRenderer::new().with_failure("Render failed");

        let spectrogram = Spectrogram {
            data: vec![vec![0.0; 10]; 10],
            time_axis: vec![0.0; 10],
            freq_axis: vec![0.0; 10],
            metadata: SpectrogramMetadata {
                config: SpectrogramConfig {
                    window_size: 1024,
                    hop_size: 256,
                    sample_rate: 44100.0,
                    min_freq: 20.0,
                    max_freq: 20000.0,
                    image_width: 800,
                    image_height: 600,
                    window_type: crate::core::config::WindowType::Hanning,
                    analysis_duration_ms: 1000.0,
                },
                input_path: PathBuf::from("test.wav"),
                start_time: 0.0,
                end_time: 1.0,
                analysis_duration_ms: 1000.0,
                annotations: vec![],
            },
        };

        let result = renderer.render(&spectrogram, &PathBuf::from("output.png"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Render failed"));
    }
}
