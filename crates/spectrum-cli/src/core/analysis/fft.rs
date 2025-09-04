use crate::core::analysis::windowing;
use crate::core::config::SpectrogramConfig;
use crate::error::SpectrumError;
use rustfft::{FftPlanner, num_complex::Complex};

/// Simple FFT processor for spectrogram generation
pub struct FFTProcessor {
    config: SpectrogramConfig,
    window: Vec<f32>,
}

impl FFTProcessor {
    /// Create a new FFT processor
    pub fn new(config: SpectrogramConfig) -> Self {
        let window = windowing::generate_hanning_window(config.window_size);
        Self { config, window }
    }

    /// Process a single frame of audio samples
    pub fn process_frame(&self, samples: &[f32]) -> Result<Vec<f32>, SpectrumError> {
        if samples.len() != self.config.window_size {
            return Err(SpectrumError::new(format!(
                "Expected {} samples, got {}",
                self.config.window_size,
                samples.len()
            )));
        }

        // Apply window function and convert to complex
        let mut buffer: Vec<Complex<f32>> = samples
            .iter()
            .zip(self.window.iter())
            .map(|(&sample, &window_val)| Complex::new(sample * window_val, 0.0))
            .collect();

        // Perform FFT
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(self.config.window_size);
        fft.process(&mut buffer);

        // Convert to magnitude spectrum
        Ok(self.compute_magnitude_spectrum(&buffer))
    }

    /// Compute magnitude spectrum from FFT output with proper frequency mapping
    fn compute_magnitude_spectrum(&self, fft_output: &[Complex<f32>]) -> Vec<f32> {
        let mut spectrum = Vec::with_capacity(self.config.freq_bins());

        // Adaptive window compensation based on window size
        let window_compensation = match self.config.window_size {
            w if w <= 256 => 1.5,
            w if w <= 512 => 1.75,
            _ => 2.0,
        };

        for bin in 0..self.config.freq_bins() {
            let db_value = if bin < fft_output.len() {
                let magnitude = fft_output[bin].norm() / self.config.window_size as f32;
                let adjusted = magnitude * window_compensation;
                20.0 * adjusted.max(1e-12).log10().max(-120.0)
            } else {
                -120.0
            };
            spectrum.push(db_value);
        }

        spectrum
    }

    /// Process entire audio signal and return spectrogram with high resolution
    pub fn process_signal(&self, samples: &[f32]) -> Result<Vec<Vec<f32>>, SpectrumError> {
        if samples.len() < self.config.window_size {
            return Err(SpectrumError::new("Signal too short for window size"));
        }

        let mut spectrogram = Vec::new();
        let mut pos = 0;

        // Use smaller hop size for smoother time resolution
        let effective_hop_size = self.config.hop_size;

        while pos + self.config.window_size <= samples.len() {
            let frame_samples = &samples[pos..pos + self.config.window_size];
            let spectrum = self.process_frame(frame_samples)?;
            spectrogram.push(spectrum);
            pos += effective_hop_size;
        }

        // Add partial frame processing for the end if there's remaining data
        if pos < samples.len() && samples.len() - pos > self.config.window_size / 2 {
            let remaining = samples.len() - pos;
            let mut padded_frame = vec![0.0; self.config.window_size];
            padded_frame[..remaining].copy_from_slice(&samples[pos..]);

            if let Ok(spectrum) = self.process_frame(&padded_frame) {
                spectrogram.push(spectrum);
            }
        }

        Ok(spectrogram)
    }

    /// Process signal with zero padding for short audio
    pub fn process_signal_with_padding(
        &self,
        samples: &[f32],
        padding_ratio: f32,
    ) -> Result<Vec<Vec<f32>>, SpectrumError> {
        // Calculate padding size
        let padding_samples = (samples.len() as f32 * padding_ratio) as usize;

        // Create padded signal
        let mut padded_samples = vec![0.0; padding_samples];
        padded_samples.extend_from_slice(samples);
        padded_samples.extend(vec![0.0; padding_samples]);

        // Process with padding
        let mut spectrogram = Vec::new();
        let mut pos = 0;
        let effective_hop_size = self.config.hop_size;

        while pos + self.config.window_size <= padded_samples.len() {
            let frame_samples = &padded_samples[pos..pos + self.config.window_size];
            let spectrum = self.process_frame(frame_samples)?;
            spectrogram.push(spectrum);
            pos += effective_hop_size;
        }

        // Process remaining samples if any
        if pos < padded_samples.len() && padded_samples.len() - pos > self.config.window_size / 4 {
            let remaining = padded_samples.len() - pos;
            let mut padded_frame = vec![0.0; self.config.window_size];
            padded_frame[..remaining].copy_from_slice(&padded_samples[pos..]);

            if let Ok(spectrum) = self.process_frame(&padded_frame) {
                spectrogram.push(spectrum);
            }
        }

        Ok(spectrogram)
    }

    /// Get configuration reference
    pub fn config(&self) -> &SpectrogramConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::QualityLevel;

    fn create_test_config() -> SpectrogramConfig {
        SpectrogramConfig::new(44100.0, 100.0, 8000.0, 1024, QualityLevel::Standard).unwrap()
    }

    fn generate_sine_wave(freq: f32, duration: f32, sample_rate: f32) -> Vec<f32> {
        let samples = (duration * sample_rate) as usize;
        (0..samples)
            .map(|i| {
                let t = i as f32 / sample_rate;
                (2.0 * std::f32::consts::PI * freq * t).sin()
            })
            .collect()
    }

    #[test]
    fn test_fft_processor_creation() {
        let config = create_test_config();
        let processor = FFTProcessor::new(config);
        assert_eq!(processor.window.len(), processor.config.window_size);
    }

    #[test]
    fn test_window_generation() {
        let window = windowing::generate_hanning_window(1024);
        assert_eq!(window.len(), 1024);
        assert!(window[0] < 0.1);
        assert!(window[window.len() - 1] < 0.1);
        assert!(window[window.len() / 2] > 0.9);
    }

    #[test]
    fn test_sine_wave_detection() {
        let config = create_test_config();
        let processor = FFTProcessor::new(config);
        let samples = generate_sine_wave(1000.0, 0.1, processor.config.sample_rate);

        let spectrum = processor
            .process_frame(&samples[..processor.config.window_size])
            .unwrap();

        // Should detect peak around 1kHz
        let freq_resolution = processor.config.freq_resolution();
        let expected_bin = (1000.0 / freq_resolution) as usize;

        let peak_bin = spectrum
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .unwrap()
            .0;

        assert!((peak_bin as i32 - expected_bin as i32).abs() <= 2);
    }

    #[test]
    fn test_signal_processing() {
        let config = create_test_config();
        let processor = FFTProcessor::new(config);
        let samples = generate_sine_wave(1000.0, 0.5, processor.config.sample_rate);

        let spectrogram = processor.process_signal(&samples).unwrap();
        assert!(!spectrogram.is_empty());

        // With the fixed frequency processing, spectrum length should be freq_bins
        assert_eq!(spectrogram[0].len(), processor.config.freq_bins());
    }

    #[test]
    fn test_short_audio_with_padding() {
        // Create a very short audio sample (50ms)
        let sample_rate = 44100.0;
        let duration = 0.05; // 50ms
        let config =
            SpectrogramConfig::for_short_audio(sample_rate, 100.0, 8000.0, duration * 1000.0)
                .unwrap();
        let processor = FFTProcessor::new(config);

        let samples = generate_sine_wave(1000.0, duration, sample_rate);

        // Process without padding
        let spectrogram_no_padding = processor.process_signal(&samples).unwrap();

        // Process with padding
        let spectrogram_with_padding = processor
            .process_signal_with_padding(&samples, 0.2)
            .unwrap();

        // With padding should produce more frames
        assert!(spectrogram_with_padding.len() > spectrogram_no_padding.len());

        // Both should have the same spectrum length per frame
        if !spectrogram_no_padding.is_empty() {
            assert_eq!(
                spectrogram_no_padding[0].len(),
                spectrogram_with_padding[0].len()
            );
        }
    }

    #[test]
    fn test_very_short_audio_processing() {
        // Test with 20ms audio (very short)
        let sample_rate = 44100.0;
        let duration = 0.02; // 20ms
        let config =
            SpectrogramConfig::for_short_audio(sample_rate, 100.0, 8000.0, duration * 1000.0)
                .unwrap();

        // Should use small window size for short audio
        assert_eq!(config.window_size, 256);
        assert_eq!(config.hop_size, 7); // 97% overlap for very short duration

        let processor = FFTProcessor::new(config);
        let samples = generate_sine_wave(440.0, duration, sample_rate); // A4 note

        // Process with padding to ensure we get frames
        let spectrogram = processor
            .process_signal_with_padding(&samples, 0.3)
            .unwrap();

        // Even very short audio should produce at least some frames with padding
        assert!(!spectrogram.is_empty());

        // Verify frequency detection still works
        if !spectrogram.is_empty() {
            let spectrum = &spectrogram[spectrogram.len() / 2]; // Middle frame
            let freq_resolution = processor.config.freq_resolution();
            let expected_bin = (440.0 / freq_resolution) as usize;

            // Find peak (allowing some tolerance for very short signal)
            let mut max_power = -120.0;
            let mut peak_bin = 0;
            for (bin, &power) in spectrum.iter().enumerate() {
                if power > max_power {
                    max_power = power;
                    peak_bin = bin;
                }
            }

            // Allow more tolerance for very short signals
            assert!((peak_bin as i32 - expected_bin as i32).abs() <= 5);
        }
    }
}
