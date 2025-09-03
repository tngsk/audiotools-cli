//! Simplified spectrum analysis and spectrogram generation module

pub mod command;
pub mod core;
pub mod domain;
pub mod error;
pub mod render;
pub mod testing;

// Re-export main types from new modules
pub use core::analysis::fft::FFTProcessor;
pub use core::config::{FrequencyPreset, QualityLevel, SpectrogramConfig, WindowType};
pub use error::{ResultExt, SpectrumError as SpectrogramError};

// Constants (consider moving to config or render modules)
// These constants are now used within the render module, so they can be removed from here.
// const FONT_FAMILY: &str = "Fira Code";
// const BACKGROUND_COLOR: RGBColor = RGBColor(4, 20, 36);

// TODO: Remove existing functions that have been moved or are no longer needed
// parse_frequency_annotation, get_frequency_preset, create_spectrograms, create_spectrogram
