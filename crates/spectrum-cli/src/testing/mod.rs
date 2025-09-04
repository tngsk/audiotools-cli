pub mod fixtures;
pub mod mocks;

#[cfg(test)]
pub mod integration;

// Re-export commonly used test utilities
pub use fixtures::{
    common_annotations, configs, create_chirp_audio, create_harmonic_audio, create_noise_audio,
    create_pattern_spectrogram, create_test_audio, create_test_request, create_test_spectrogram,
    durations, sample_rates, TestPattern,
};

pub use mocks::{
    FailingAudioLoader, MockAudioLoader, MockSpectralAnalyzer, MockSpectrogramRenderer,
};
