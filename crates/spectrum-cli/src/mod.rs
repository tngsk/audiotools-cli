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
