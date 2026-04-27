pub mod builder;
pub mod presets;
pub mod validator;

pub use validator::validate_config;

use crate::error::SpectrumError;

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
    pub analysis_duration_ms: f32, // Added for clarity in analysis module
}

impl SpectrogramConfig {
    /// Calculate optimal window size based on audio duration
    ///
    /// This considers the actual analysis duration, which may be shorter than
    /// the total file duration when using --start/--end options.
    ///
    /// # Examples
    ///
    /// ```
    /// use crate::core::config::SpectrogramConfig;
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// // 50ms analysis (very short) - use small window for high time resolution
    /// let window_size = SpectrogramConfig::calculate_optimal_window_size(50.0, 44100.0);
    /// assert_eq!(window_size, 256);
    ///
    /// // 10-second file but analyzing only 80ms section (--start 5.0 --end 5.08)
    /// let window_size = SpectrogramConfig::calculate_optimal_window_size(80.0, 44100.0);
    /// assert_eq!(window_size, 256); // Still uses small window for short analysis
    /// Ok(())
    /// }
    /// ```
    pub fn calculate_optimal_window_size(duration_ms: f32, _sample_rate: f32) -> usize {
        match duration_ms {
            d if d < 150.0 => 256,   // Very short - prioritize time resolution
            d if d < 500.0 => 512,   // Short audio
            d if d < 2000.0 => 1024, // Medium audio
            _ => 2048,               // Long - prioritize frequency resolution
        }
    }

    /// Get duration preset from audio duration
    pub fn get_duration_preset(duration_ms: f32) -> DurationPreset {
        presets::get_duration_preset(duration_ms)
    }

    /// Calculate hop size for short audio (higher overlap)
    pub fn calculate_hop_size_for_short_audio(
        window_size: usize,
        duration_preset: DurationPreset,
    ) -> usize {
        let overlap_ratio = match duration_preset {
            DurationPreset::VeryShort => 0.97,
            DurationPreset::Short => 0.93,
            DurationPreset::Medium => 0.875,
            DurationPreset::Long => 0.75,
        };
        ((window_size as f32 * (1.0 - overlap_ratio)) as usize).max(1)
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
    /// use crate::core::config::SpectrogramConfig;
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// // 80ms analysis duration - uses 256 window with 95% overlap
    /// let config = SpectrogramConfig::for_short_audio(44100.0, 20.0, 8000.0, 80.0)?;
    /// assert_eq!(config.window_size, 256);
    /// assert_eq!(config.hop_size, 7); // 97% overlap
    ///
    /// // 300ms analysis duration - uses 512 window with 90% overlap
    /// let config = SpectrogramConfig::for_short_audio(44100.0, 20.0, 8000.0, 300.0)?;
    /// assert_eq!(config.window_size, 512);
    /// assert_eq!(config.hop_size, 35); // 93% overlap
    /// Ok(())
    /// }
    /// ```
    pub fn for_short_audio(
        sample_rate: f32,
        min_freq: f32,
        max_freq: f32,
        duration_ms: f32,
    ) -> Result<Self, SpectrumError> {
        let window_size = Self::calculate_optimal_window_size(duration_ms, sample_rate);
        let hop_size = Self::calculate_hop_size_for_short_audio(
            window_size,
            Self::get_duration_preset(duration_ms),
        );

        Self::new_with_params(
            window_size,
            hop_size,
            sample_rate,
            min_freq,
            max_freq,
            duration_ms,
        )
    }

    /// Create a new configuration with manual parameters
    pub fn new(
        sample_rate: f32,
        min_freq: f32,
        max_freq: f32,
        window_size: usize,
        quality_level: QualityLevel,
    ) -> Result<Self, SpectrumError> {
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
            analysis_duration_ms: 0.0, // Default, will be set by auto_configure or from_legacy_params_with_duration
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
    ) -> Result<Self, SpectrumError> {
        let hop_size = duration_ms
            .map(|d| {
                Self::calculate_hop_size_for_short_audio(window_size, Self::get_duration_preset(d))
            })
            .unwrap_or_else(|| (window_size / 8).max(1));

        Self::new_with_params(
            window_size,
            hop_size,
            sample_rate,
            min_freq,
            max_freq,
            duration_ms.unwrap_or(0.0),
        )
    }

    /// Create configuration from legacy parameters
    pub fn from_legacy_params(
        window_size: usize,
        overlap: f32,
        min_freq: f32,
        max_freq: f32,
        sample_rate: f32,
    ) -> Result<Self, SpectrumError> {
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
    /// use crate::core::config::SpectrogramConfig;
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// // Auto-configure for 120ms analysis
    /// // - Uses window_size=256, hop_size=13 (95% overlap)
    /// let config = SpectrogramConfig::auto_configure(44100.0, 20.0, 20000.0, 120.0)?;
    ///
    /// // Auto-configure for 1500ms analysis
    /// // - Uses window_size=1024, hop_size=128 (87.5% overlap)
    /// let config = SpectrogramConfig::auto_configure(44100.0, 20.0, 20000.0, 1500.0)?;
    /// Ok(())
    /// }
    /// ```
    pub fn auto_configure(
        sample_rate: f32,
        min_freq: f32,
        max_freq: f32,
        duration_ms: f32,
    ) -> Result<Self, SpectrumError> {
        Self::for_short_audio(sample_rate, min_freq, max_freq, duration_ms)
    }

    /// Helper to create config with validation
    fn new_with_params(
        window_size: usize,
        hop_size: usize,
        sample_rate: f32,
        min_freq: f32,
        max_freq: f32,
        analysis_duration_ms: f32,
    ) -> Result<Self, SpectrumError> {
        let mut config = Self {
            window_size,
            hop_size,
            sample_rate,
            min_freq,
            max_freq,
            image_width: 1200,
            image_height: 600,
            window_type: WindowType::Hanning,
            analysis_duration_ms,
        };
        config.validate()?;
        Ok(config)
    }

    /// Calculate hop size based on quality level
    pub fn calculate_hop_size(window_size: usize, quality: QualityLevel) -> usize {
        let overlap_ratio = match quality {
            QualityLevel::Fast => 0.5,      // 50% overlap
            QualityLevel::Standard => 0.75, // 75% overlap
            QualityLevel::High => 0.875,    // 87.5% overlap
        };

        let hop_size = (window_size as f32 * (1.0 - overlap_ratio)) as usize;
        hop_size.max(1)
    }

    /// Validate configuration parameters
    pub fn validate(&mut self) -> Result<(), SpectrumError> {
        validate_config(self)
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
        presets::frequency_preset(preset, sample_rate)
    }

    /// Set custom frequency range
    pub fn with_frequency_range(
        mut self,
        min_freq: f32,
        max_freq: f32,
    ) -> Result<Self, SpectrumError> {
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

    // Add this method to get the default quality level
    pub fn quality_level(&self) -> QualityLevel {
        // This is a simplified way to get the quality level from the config.
        // In a more complex scenario, you might store it directly or derive it.
        // For now, we'll assume a standard quality if not explicitly set.
        QualityLevel::Standard
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
            analysis_duration_ms: 0.0,
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
        assert_eq!(config.hop_size, 7); // 97% overlap for very short audio

        let config2 = SpectrogramConfig::for_short_audio(44100.0, 20.0, 20000.0, 300.0).unwrap();
        assert_eq!(config2.window_size, 512);
        assert_eq!(config2.hop_size, 35); // 93% overlap for short audio
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
