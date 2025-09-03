use crate::command::spectrum::core::config::SpectrogramConfig;
use crate::command::spectrum::error::SpectrumError;

pub fn validate_config(config: &mut SpectrogramConfig) -> Result<(), SpectrumError> {
    // Validate window size
    if config.window_size == 0 || !config.window_size.is_power_of_two() {
        return Err(SpectrumError::new("Window size must be a power of 2"));
    }

    // Validate hop size
    if config.hop_size == 0 || config.hop_size > config.window_size {
        return Err(SpectrumError::new("Invalid hop size"));
    }

    // Validate sample rate
    if config.sample_rate <= 0.0 {
        return Err(SpectrumError::new("Sample rate must be positive"));
    }

    // Validate frequency range
    if config.min_freq <= 0.0 || config.max_freq <= config.min_freq {
        return Err(SpectrumError::new("Invalid frequency range"));
    }

    // Auto-adjust max frequency if needed
    let nyquist = config.sample_rate / 2.0;
    if config.max_freq > nyquist {
        config.max_freq = nyquist;
    }

    // Validate analysis duration
    if config.analysis_duration_ms < 0.0 {
        return Err(SpectrumError::new("Analysis duration must be non-negative"));
    }

    Ok(())
}
