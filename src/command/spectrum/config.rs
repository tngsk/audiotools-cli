use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowType {
    Hanning,
    Hamming,
    Blackman,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QualityLevel {
    Fast,     // 50% overlap
    Standard, // 75% overlap
    High,     // 87.5% overlap
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FrequencyPreset {
    Full,        // 20 Hz to Nyquist
    AudioRange,  // 20 Hz to 20 kHz
    SpeechRange, // 80 Hz to 8 kHz
    MusicRange,  // 80 Hz to 12 kHz
    Bass,        // 60-250 Hz
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DurationPreset {
    VeryShort, // < 100ms
    Short,     // 100-500ms
    Medium,    // 500ms-2s
    Long,      // > 2s
}

#[derive(Debug, Clone)]
pub struct SpectrogramConfig {
    pub window_size: usize,
    pub hop_size: usize,
    pub sample_rate: f32,
    pub min_freq: f32,
    pub max_freq: f32,
    pub image_width: u32,
    pub image_height: u32,
    pub window_type: WindowType,
}

#[derive(Debug)]
pub enum ConfigError {
    InvalidWindowSize(String),
    InvalidHopSize(String),
    InvalidFrequencyRange(String),
    InvalidSampleRate(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::InvalidWindowSize(msg) => write!(f, "Invalid window size: {}", msg),
            ConfigError::InvalidHopSize(msg) => write!(f, "Invalid hop size: {}", msg),
            ConfigError::InvalidFrequencyRange(msg) => {
                write!(f, "Invalid frequency range: {}", msg)
            }
            ConfigError::InvalidSampleRate(msg) => write!(f, "Invalid sample rate: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}

impl SpectrogramConfig {
    /// Calculate optimal window size based on audio duration
    ///
    /// This considers the actual analysis duration, which may be shorter than
    /// the total file duration when using --start/--end options.
    ///
    /// # Examples
    ///
    /// ```
    /// // 50ms analysis (very short) - use small window for high time resolution
    /// let window_size = SpectrogramConfig::calculate_optimal_window_size(50.0, 44100.0);
    /// assert_eq!(window_size, 256);
    ///
    /// // 10-second file but analyzing only 80ms section (--start 5.0 --end 5.08)
    /// let window_size = SpectrogramConfig::calculate_optimal_window_size(80.0, 44100.0);
    /// assert_eq!(window_size, 256); // Still uses small window for short analysis
    /// ```
    pub fn calculate_optimal_window_size(duration_ms: f32, _sample_rate: f32) -> usize {
        // Adaptive window size selection based on duration
        let window_size = if duration_ms < 100.0 {
            256 // Very short audio - prioritize time resolution
        } else if duration_ms < 500.0 {
            512 // Short audio
        } else if duration_ms < 2000.0 {
            1024 // Medium audio
        } else {
            2048 // Long audio - prioritize frequency resolution
        };

        window_size
    }

    /// Get duration preset from audio duration
    pub fn get_duration_preset(duration_ms: f32) -> DurationPreset {
        if duration_ms < 100.0 {
            DurationPreset::VeryShort
        } else if duration_ms < 500.0 {
            DurationPreset::Short
        } else if duration_ms < 2000.0 {
            DurationPreset::Medium
        } else {
            DurationPreset::Long
        }
    }

    /// Calculate hop size for short audio (higher overlap)
    pub fn calculate_hop_size_for_short_audio(
        window_size: usize,
        duration_preset: DurationPreset,
    ) -> usize {
        let overlap_ratio = match duration_preset {
            DurationPreset::VeryShort => 0.95, // 95% overlap for very short audio
            DurationPreset::Short => 0.90,     // 90% overlap for short audio
            DurationPreset::Medium => 0.875,   // 87.5% overlap for medium audio
            DurationPreset::Long => 0.75,      // 75% overlap for long audio
        };

        let hop_size = (window_size as f32 * (1.0 - overlap_ratio)) as usize;
        hop_size.max(1)
    }

    /// Create configuration optimized for short audio
    ///
    /// This method is used when window_size is set to 0 (auto-configure mode)
    /// and the analysis duration is short. When window_size is explicitly specified,
    /// that value is always used regardless of audio duration.
    ///
    /// # Examples
    ///
    /// ```
    /// // 80ms analysis duration - uses 256 window with 95% overlap
    /// let config = SpectrogramConfig::for_short_audio(44100.0, 20.0, 8000.0, 80.0)?;
    /// assert_eq!(config.window_size, 256);
    /// assert_eq!(config.hop_size, 12); // 95% overlap
    ///
    /// // 300ms analysis duration - uses 512 window with 90% overlap
    /// let config = SpectrogramConfig::for_short_audio(44100.0, 20.0, 8000.0, 300.0)?;
    /// assert_eq!(config.window_size, 512);
    /// assert_eq!(config.hop_size, 51); // 90% overlap
    /// ```
    pub fn for_short_audio(
        sample_rate: f32,
        min_freq: f32,
        max_freq: f32,
        duration_ms: f32,
    ) -> Result<Self, ConfigError> {
        let window_size = Self::calculate_optimal_window_size(duration_ms, sample_rate);
        let duration_preset = Self::get_duration_preset(duration_ms);
        let hop_size = Self::calculate_hop_size_for_short_audio(window_size, duration_preset);

        let mut config = Self {
            window_size,
            hop_size,
            sample_rate,
            min_freq,
            max_freq,
            image_width: 1200,
            image_height: 600,
            window_type: WindowType::Hanning,
        };

        config.validate()?;
        Ok(config)
    }

    /// Create a new configuration with manual parameters
    pub fn new(
        sample_rate: f32,
        min_freq: f32,
        max_freq: f32,
        window_size: usize,
        quality_level: QualityLevel,
    ) -> Result<Self, ConfigError> {
        let hop_size = Self::calculate_hop_size(window_size, quality_level);

        let mut config = Self {
            window_size,
            hop_size,
            sample_rate,
            min_freq,
            max_freq,
            image_width: 1200,
            image_height: 600,
            window_type: WindowType::Hanning,
        };

        config.validate()?;
        Ok(config)
    }

    /// Create configuration from legacy parameters with optional duration hint
    ///
    /// When window_size is explicitly specified (non-zero), it will be used
    /// regardless of the duration_ms hint. The duration is only used to
    /// determine the appropriate hop_size for better time resolution.
    pub fn from_legacy_params_with_duration(
        window_size: usize,
        _overlap: f32, // Ignored - use adaptive hop_size
        min_freq: f32,
        max_freq: f32,
        sample_rate: f32,
        duration_ms: Option<f32>,
    ) -> Result<Self, ConfigError> {
        // Use adaptive hop_size based on duration if provided
        let hop_size = if let Some(duration) = duration_ms {
            let duration_preset = Self::get_duration_preset(duration);
            Self::calculate_hop_size_for_short_audio(window_size, duration_preset)
        } else {
            // Default to high-resolution hop_size (87.5% overlap)
            (window_size / 8).max(1)
        };

        let mut config = Self {
            window_size,
            hop_size,
            sample_rate,
            min_freq,
            max_freq,
            image_width: 1200,
            image_height: 600,
            window_type: WindowType::Hanning,
        };

        config.validate()?;
        Ok(config)
    }

    /// Create configuration from legacy parameters
    pub fn from_legacy_params(
        window_size: usize,
        overlap: f32,
        min_freq: f32,
        max_freq: f32,
        sample_rate: f32,
    ) -> Result<Self, ConfigError> {
        Self::from_legacy_params_with_duration(
            window_size,
            overlap,
            min_freq,
            max_freq,
            sample_rate,
            None,
        )
    }

    /// Create configuration with automatic window size selection
    ///
    /// Automatically selects optimal parameters based on analysis duration.
    /// This is used when --window-size is set to 0 or --adaptive flag is used.
    ///
    /// Note: When window_size is explicitly specified (non-zero), that value
    /// will be used instead of auto-configuration, regardless of duration.
    ///
    /// # Examples
    ///
    /// ```
    /// // Auto-configure for 120ms analysis
    /// // - Uses window_size=256, hop_size=13 (95% overlap)
    /// let config = SpectrogramConfig::auto_configure(44100.0, 20.0, 20000.0, 120.0)?;
    ///
    /// // Auto-configure for 1500ms analysis
    /// // - Uses window_size=1024, hop_size=128 (87.5% overlap)
    /// let config = SpectrogramConfig::auto_configure(44100.0, 20.0, 20000.0, 1500.0)?;
    /// ```
    pub fn auto_configure(
        sample_rate: f32,
        min_freq: f32,
        max_freq: f32,
        duration_ms: f32,
    ) -> Result<Self, ConfigError> {
        let window_size = Self::calculate_optimal_window_size(duration_ms, sample_rate);
        let duration_preset = Self::get_duration_preset(duration_ms);
        let hop_size = Self::calculate_hop_size_for_short_audio(window_size, duration_preset);

        let mut config = Self {
            window_size,
            hop_size,
            sample_rate,
            min_freq,
            max_freq,
            image_width: 1200,
            image_height: 600,
            window_type: WindowType::Hanning,
        };

        config.validate()?;
        Ok(config)
    }

    /// Calculate hop size based on quality level
    fn calculate_hop_size(window_size: usize, quality: QualityLevel) -> usize {
        let overlap_ratio = match quality {
            QualityLevel::Fast => 0.5,      // 50% overlap
            QualityLevel::Standard => 0.75, // 75% overlap
            QualityLevel::High => 0.875,    // 87.5% overlap
        };

        let hop_size = (window_size as f32 * (1.0 - overlap_ratio)) as usize;
        hop_size.max(1)
    }

    /// Validate configuration parameters
    pub fn validate(&mut self) -> Result<(), ConfigError> {
        // Validate window size
        if self.window_size == 0 || !self.window_size.is_power_of_two() {
            return Err(ConfigError::InvalidWindowSize(
                "Window size must be a power of 2".to_string(),
            ));
        }

        // Validate hop size
        if self.hop_size == 0 || self.hop_size > self.window_size {
            return Err(ConfigError::InvalidHopSize("Invalid hop size".to_string()));
        }

        // Validate sample rate
        if self.sample_rate <= 0.0 {
            return Err(ConfigError::InvalidSampleRate(
                "Sample rate must be positive".to_string(),
            ));
        }

        // Validate frequency range
        if self.min_freq <= 0.0 || self.max_freq <= self.min_freq {
            return Err(ConfigError::InvalidFrequencyRange(
                "Invalid frequency range".to_string(),
            ));
        }

        // Auto-adjust max frequency to Nyquist frequency if it exceeds
        let nyquist = self.sample_rate / 2.0;
        if self.max_freq > nyquist {
            eprintln!("Warning: Max frequency {:.1} Hz exceeds Nyquist frequency {:.1} Hz. Auto-adjusting to {:.1} Hz.",
                     self.max_freq, nyquist, nyquist);
            self.max_freq = nyquist;
        }

        Ok(())
    }

    /// Calculate frequency resolution in Hz
    pub fn freq_resolution(&self) -> f32 {
        self.sample_rate / self.window_size as f32
    }

    /// Calculate time resolution in seconds
    pub fn time_resolution(&self) -> f32 {
        self.hop_size as f32 / self.sample_rate
    }

    /// Calculate number of frequency bins
    pub fn freq_bins(&self) -> usize {
        self.window_size / 2
    }

    /// Calculate expected number of time frames for given sample count
    pub fn time_frames(&self, sample_count: usize) -> usize {
        if sample_count < self.window_size {
            return 0;
        }
        (sample_count - self.window_size) / self.hop_size + 1
    }

    /// Get time step for rendering (time per frame)
    pub fn time_step(&self, total_frames: usize, total_duration: f32) -> f32 {
        if total_frames <= 1 {
            total_duration
        } else {
            total_duration / total_frames as f32
        }
    }

    /// Get frequency step for rendering
    pub fn freq_step(&self) -> f32 {
        self.freq_resolution()
    }

    /// Get frequency range from preset
    pub fn frequency_preset(preset: FrequencyPreset, sample_rate: f32) -> (f32, f32) {
        let nyquist = sample_rate / 2.0;
        match preset {
            FrequencyPreset::Full => (20.0, nyquist),
            FrequencyPreset::AudioRange => (20.0, 20000.0_f32.min(nyquist)),
            FrequencyPreset::SpeechRange => (80.0, 8000.0_f32.min(nyquist)),
            FrequencyPreset::MusicRange => (80.0, 12000.0_f32.min(nyquist)),
            FrequencyPreset::Bass => (60.0, 250.0_f32.min(nyquist)),
        }
    }

    /// Set custom frequency range
    pub fn with_frequency_range(
        mut self,
        min_freq: f32,
        max_freq: f32,
    ) -> Result<Self, ConfigError> {
        self.min_freq = min_freq;
        self.max_freq = max_freq;

        // Auto-adjust max frequency if needed
        let nyquist = self.sample_rate / 2.0;
        if self.max_freq > nyquist {
            self.max_freq = nyquist;
        }

        self.validate()?;
        Ok(self)
    }
}

impl Default for SpectrogramConfig {
    fn default() -> Self {
        Self {
            window_size: 2048,
            hop_size: 512,
            sample_rate: 44100.0,
            min_freq: 20.0,
            max_freq: 20000.0,
            image_width: 1200,
            image_height: 600,
            window_type: WindowType::Hanning,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config =
            SpectrogramConfig::new(44100.0, 20.0, 20000.0, 2048, QualityLevel::Standard).unwrap();

        assert!(config.window_size.is_power_of_two());
        assert!(config.hop_size < config.window_size);
    }

    #[test]
    fn test_legacy_compatibility() {
        let config =
            SpectrogramConfig::from_legacy_params(2048, 0.75, 20.0, 20000.0, 44100.0).unwrap();

        assert_eq!(config.window_size, 2048);
        assert_eq!(config.hop_size, 256); // Fixed to window_size / 8 for high resolution
    }

    #[test]
    fn test_adaptive_window_size() {
        assert_eq!(
            SpectrogramConfig::calculate_optimal_window_size(50.0, 44100.0),
            256
        );
        assert_eq!(
            SpectrogramConfig::calculate_optimal_window_size(200.0, 44100.0),
            512
        );
        assert_eq!(
            SpectrogramConfig::calculate_optimal_window_size(1000.0, 44100.0),
            1024
        );
        assert_eq!(
            SpectrogramConfig::calculate_optimal_window_size(5000.0, 44100.0),
            2048
        );
    }

    #[test]
    fn test_short_audio_config() {
        let config = SpectrogramConfig::for_short_audio(44100.0, 20.0, 20000.0, 80.0).unwrap();
        assert_eq!(config.window_size, 256);
        assert_eq!(config.hop_size, 12); // 95% overlap for very short audio

        let config2 = SpectrogramConfig::for_short_audio(44100.0, 20.0, 20000.0, 300.0).unwrap();
        assert_eq!(config2.window_size, 512);
        assert_eq!(config2.hop_size, 51); // 90% overlap for short audio
    }

    #[test]
    fn test_frequency_presets() {
        let (min, max) = SpectrogramConfig::frequency_preset(FrequencyPreset::SpeechRange, 44100.0);
        assert_eq!(min, 80.0);
        assert_eq!(max, 8000.0);
    }

    #[test]
    fn test_low_sample_rate_auto_adjustment() {
        // Test with 16kHz sample rate (Nyquist = 8kHz)
        let config =
            SpectrogramConfig::new(16000.0, 20.0, 20000.0, 2048, QualityLevel::Standard).unwrap();

        // Max frequency should be auto-adjusted to Nyquist frequency
        assert_eq!(config.max_freq, 8000.0);

        // Test with very low sample rate (8kHz, Nyquist = 4kHz)
        let config2 =
            SpectrogramConfig::new(8000.0, 80.0, 8000.0, 1024, QualityLevel::Standard).unwrap();
        assert_eq!(config2.max_freq, 4000.0);
    }

    #[test]
    fn test_frequency_preset_with_low_sample_rate() {
        // Test presets with low sample rate
        let (min, max) = SpectrogramConfig::frequency_preset(FrequencyPreset::AudioRange, 16000.0);
        assert_eq!(min, 20.0);
        assert_eq!(max, 8000.0); // Should be limited to Nyquist frequency

        let (min, max) = SpectrogramConfig::frequency_preset(FrequencyPreset::SpeechRange, 8000.0);
        assert_eq!(min, 80.0);
        assert_eq!(max, 4000.0); // Should be limited to Nyquist frequency
    }
}
