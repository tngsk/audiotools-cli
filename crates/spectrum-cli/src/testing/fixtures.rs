use crate::core::config::{SpectrogramConfig, WindowType};
use crate::domain::audio_data::AudioData;
use crate::domain::frequency::FrequencyAnnotation;
use crate::domain::request::SpectrumRequest;
use crate::domain::spectrogram::{Spectrogram, SpectrogramMetadata};
use std::path::PathBuf;

/// Common sample rates for testing
pub mod sample_rates {
    pub const CD_QUALITY: f32 = 44100.0;
    pub const DVD_QUALITY: f32 = 48000.0;
    pub const HIGH_RES: f32 = 96000.0;
    pub const LOW_QUALITY: f32 = 22050.0;
}

/// Common test durations in seconds
pub mod durations {
    pub const VERY_SHORT: f32 = 0.05; // 50ms
    pub const SHORT: f32 = 0.5; // 500ms
    pub const MEDIUM: f32 = 2.0; // 2 seconds
    pub const LONG: f32 = 10.0; // 10 seconds
}

/// Create test audio data with a sine wave
pub fn create_test_audio(frequency: f32, duration: f32, sample_rate: f32) -> AudioData {
    let num_samples = (sample_rate * duration) as usize;
    let samples: Vec<f32> = (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate;
            (2.0 * std::f32::consts::PI * frequency * t).sin()
        })
        .collect();

    AudioData {
        samples,
        sample_rate,
        duration,
        channels: 1,
    }
}

/// Create test audio with multiple sine waves (harmonics)
pub fn create_harmonic_audio(
    fundamental: f32,
    harmonics: &[f32],
    duration: f32,
    sample_rate: f32,
) -> AudioData {
    let num_samples = (sample_rate * duration) as usize;
    let samples: Vec<f32> = (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate;
            let fundamental_wave = (2.0 * std::f32::consts::PI * fundamental * t).sin();

            let harmonic_sum: f32 = harmonics
                .iter()
                .enumerate()
                .map(|(n, &amp)| {
                    let freq = fundamental * (n as f32 + 2.0); // 2nd harmonic, 3rd, etc.
                    amp * (2.0 * std::f32::consts::PI * freq * t).sin()
                })
                .sum();

            // Normalize to prevent clipping
            (fundamental_wave + harmonic_sum) / (1.0 + harmonics.len() as f32)
        })
        .collect();

    AudioData {
        samples,
        sample_rate,
        duration,
        channels: 1,
    }
}

/// Create white noise audio data
pub fn create_noise_audio(duration: f32, sample_rate: f32) -> AudioData {
    use rand::{Rng, thread_rng};

    let num_samples = (sample_rate * duration) as usize;
    let mut rng = thread_rng();
    let samples: Vec<f32> = (0..num_samples).map(|_| rng.gen_range(-1.0..1.0)).collect();

    AudioData {
        samples,
        sample_rate,
        duration,
        channels: 1,
    }
}

/// Create a chirp signal (frequency sweep)
pub fn create_chirp_audio(
    start_freq: f32,
    end_freq: f32,
    duration: f32,
    sample_rate: f32,
) -> AudioData {
    let num_samples = (sample_rate * duration) as usize;
    let samples: Vec<f32> = (0..num_samples)
        .map(|i| {
            let t = i as f32 / sample_rate;
            let normalized_time = t / duration;
            // Linear frequency sweep
            let instantaneous_freq = start_freq + (end_freq - start_freq) * normalized_time;
            (2.0 * std::f32::consts::PI * instantaneous_freq * t).sin()
        })
        .collect();

    AudioData {
        samples,
        sample_rate,
        duration,
        channels: 1,
    }
}

/// Common test configurations
pub mod configs {
    use super::*;

    pub fn default_config() -> SpectrogramConfig {
        SpectrogramConfig {
            window_size: 1024,
            hop_size: 256,
            sample_rate: 44100.0,
            min_freq: 20.0,
            max_freq: 20000.0,
            image_width: 800,
            image_height: 600,
            window_type: WindowType::Hanning,
            analysis_duration_ms: 1000.0,
        }
    }

    pub fn high_resolution_config() -> SpectrogramConfig {
        SpectrogramConfig {
            window_size: 4096,
            hop_size: 512,
            sample_rate: 48000.0,
            min_freq: 20.0,
            max_freq: 24000.0,
            image_width: 1920,
            image_height: 1080,
            window_type: WindowType::Blackman,
            analysis_duration_ms: 1000.0,
        }
    }

    pub fn fast_config() -> SpectrogramConfig {
        SpectrogramConfig {
            window_size: 512,
            hop_size: 256,
            sample_rate: 44100.0,
            min_freq: 100.0,
            max_freq: 10000.0,
            image_width: 640,
            image_height: 480,
            window_type: WindowType::Hamming,
            analysis_duration_ms: 1000.0,
        }
    }

    pub fn speech_config() -> SpectrogramConfig {
        SpectrogramConfig {
            window_size: 512,
            hop_size: 128,
            sample_rate: 16000.0,
            min_freq: 80.0,
            max_freq: 8000.0,
            image_width: 800,
            image_height: 400,
            window_type: WindowType::Hanning,
            analysis_duration_ms: 1000.0,
        }
    }
}

/// Create test spectrogram
pub fn create_test_spectrogram(time_bins: usize, freq_bins: usize) -> Spectrogram {
    let data = vec![vec![0.5; freq_bins]; time_bins];
    let time_axis = (0..time_bins).map(|i| i as f32 * 0.01).collect();
    let freq_axis = (0..freq_bins).map(|i| i as f32 * 20.0).collect();

    Spectrogram {
        data,
        time_axis,
        freq_axis,
        metadata: SpectrogramMetadata {
            config: configs::default_config(),
            input_path: PathBuf::from("test.wav"),
            start_time: 0.0,
            end_time: 1.0,
            analysis_duration_ms: 1000.0,
            annotations: vec![],
        },
    }
}

/// Create test spectrogram with pattern
pub fn create_pattern_spectrogram(
    time_bins: usize,
    freq_bins: usize,
    pattern: TestPattern,
) -> Spectrogram {
    let data = match pattern {
        TestPattern::Diagonal => (0..time_bins)
            .map(|t| {
                (0..freq_bins)
                    .map(|f| {
                        if (t * freq_bins / time_bins) == f {
                            1.0
                        } else {
                            0.0
                        }
                    })
                    .collect()
            })
            .collect(),
        TestPattern::Horizontal => (0..time_bins)
            .map(|_| {
                (0..freq_bins)
                    .map(|f| if f == freq_bins / 2 { 1.0 } else { 0.0 })
                    .collect()
            })
            .collect(),
        TestPattern::Vertical => (0..time_bins)
            .map(|t| {
                (0..freq_bins)
                    .map(|_| if t == time_bins / 2 { 1.0 } else { 0.0 })
                    .collect()
            })
            .collect(),
        TestPattern::Checkerboard => (0..time_bins)
            .map(|t| {
                (0..freq_bins)
                    .map(|f| if (t + f) % 2 == 0 { 1.0 } else { 0.0 })
                    .collect()
            })
            .collect(),
    };

    let time_axis = (0..time_bins).map(|i| i as f32 * 0.01).collect();
    let freq_axis = (0..freq_bins).map(|i| i as f32 * 20.0).collect();

    Spectrogram {
        data,
        time_axis,
        freq_axis,
        metadata: SpectrogramMetadata {
            config: configs::default_config(),
            input_path: PathBuf::from("pattern.wav"),
            start_time: 0.0,
            end_time: time_bins as f32 * 0.01,
            analysis_duration_ms: time_bins as f32 * 10.0,
            annotations: vec![],
        },
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TestPattern {
    Diagonal,
    Horizontal,
    Vertical,
    Checkerboard,
}

/// Common frequency annotations for testing
pub fn common_annotations() -> Vec<FrequencyAnnotation> {
    vec![
        FrequencyAnnotation {
            frequency: 440.0,
            label: "A4".to_string(),
        },
        FrequencyAnnotation {
            frequency: 1000.0,
            label: "1kHz".to_string(),
        },
        FrequencyAnnotation {
            frequency: 10000.0,
            label: "10kHz".to_string(),
        },
    ]
}

/// Create a basic SpectrumRequest for testing
pub fn create_test_request(input: PathBuf, output: PathBuf) -> SpectrumRequest {
    SpectrumRequest {
        input_path: input,
        output_path: output,
        config: configs::default_config(),
        time_range: None,
        auto_start: None,
        annotations: vec![],
        options: Default::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_test_audio() {
        let audio = create_test_audio(440.0, 1.0, 44100.0);
        assert_eq!(audio.samples.len(), 44100);
        assert_eq!(audio.sample_rate, 44100.0);
        assert_eq!(audio.duration, 1.0);

        // Check that it's actually a sine wave
        let max_val = audio.samples.iter().fold(0.0f32, |a, &b| a.max(b));
        let min_val = audio.samples.iter().fold(0.0f32, |a, &b| a.min(b));
        assert!((max_val - 1.0).abs() < 0.01);
        assert!((min_val + 1.0).abs() < 0.01);
    }

    #[test]
    fn test_create_harmonic_audio() {
        let harmonics = vec![0.5, 0.25, 0.125]; // Amplitudes for 2nd, 3rd, 4th harmonics
        let audio = create_harmonic_audio(100.0, &harmonics, 0.1, 10000.0);

        assert_eq!(audio.samples.len(), 1000);
        // Should have more complex waveform than pure sine
        assert!(!audio.samples.is_empty());
    }

    #[test]
    fn test_create_noise_audio() {
        let audio = create_noise_audio(0.1, 10000.0);
        assert_eq!(audio.samples.len(), 1000);

        // Check randomness - very unlikely all values are the same
        let first = audio.samples[0];
        let all_same = audio.samples.iter().all(|&v| v == first);
        assert!(!all_same);
    }

    #[test]
    fn test_create_chirp_audio() {
        let audio = create_chirp_audio(100.0, 1000.0, 1.0, 10000.0);
        assert_eq!(audio.samples.len(), 10000);

        // Chirp should vary over time
        let first_segment = &audio.samples[0..100];
        let last_segment = &audio.samples[9900..10000];

        // Calculate rough "frequency" by zero-crossing rate
        let first_crossings = first_segment
            .windows(2)
            .filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0))
            .count();
        let last_crossings = last_segment
            .windows(2)
            .filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0))
            .count();

        // Higher frequency should have more zero crossings
        assert!(last_crossings > first_crossings);
    }

    #[test]
    fn test_pattern_spectrograms() {
        let diagonal = create_pattern_spectrogram(10, 10, TestPattern::Diagonal);
        assert_eq!(diagonal.data.len(), 10);
        assert_eq!(diagonal.data[0].len(), 10);

        // Check diagonal pattern
        for i in 0..10 {
            assert_eq!(diagonal.data[i][i], 1.0);
        }

        let horizontal = create_pattern_spectrogram(10, 10, TestPattern::Horizontal);
        // Check horizontal line at middle frequency
        for t in 0..10 {
            assert_eq!(horizontal.data[t][5], 1.0);
        }
    }

    #[test]
    fn test_configs() {
        let default = configs::default_config();
        assert_eq!(default.window_size, 1024);

        let high_res = configs::high_resolution_config();
        assert_eq!(high_res.window_size, 4096);

        let fast = configs::fast_config();
        assert_eq!(fast.window_size, 512);

        let speech = configs::speech_config();
        assert_eq!(speech.sample_rate, 16000.0);
    }
}
