use crate::command::spectrum::core::config::{SpectrogramConfig, ConfigError};
use crate::command::spectrum::error::SpectrumError;

pub fn validate_config(config: &mut SpectrogramConfig) -> Result<(), SpectrumError> {
    // Validate window size
    if config.window_size == 0 || !config.window_size.is_power_of_two() {
        return Err(ConfigError::InvalidWindowSize(
            "Window size must be a power of 2".to_string(),
        ).into());
    }

    // Validate hop size
    if config.hop_size == 0 || config.hop_size > config.window_size {
        return Err(ConfigError::InvalidHopSize("Invalid hop size".to_string()).into());
    }

    // Validate sample rate
    if config.sample_rate <= 0.0 {
        return Err(ConfigError::InvalidSampleRate(
            "Sample rate must be positive".to_string(),
        ).into());
    }

    // Validate frequency range
    if config.min_freq <= 0.0 || config.max_freq <= config.min_freq {
        return Err(ConfigError::InvalidFrequencyRange(
            "Invalid frequency range".to_string(),
        ).into());
    }

    // Auto-adjust max frequency to Nyquist frequency if it exceeds
    let nyquist = config.sample_rate / 2.0;
    if config.max_freq > nyquist {
        eprintln!("Warning: Max frequency {:.1} Hz exceeds Nyquist frequency {:.1} Hz. Auto-adjusting to {:.1} Hz.",
                 config.max_freq, nyquist, nyquist);
        config.max_freq = nyquist;
    }

    Ok(())
}
