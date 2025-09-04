use crate::core::config::{QualityLevel, SpectrogramConfig, WindowType};
use crate::error::SpectrumError;

pub struct ConfigBuilder {
    sample_rate: Option<f32>,
    min_freq: Option<f32>,
    max_freq: Option<f32>,
    window_size: Option<usize>,
    quality_level: Option<QualityLevel>,
    duration_ms: Option<f32>,
    image_width: Option<u32>,
    image_height: Option<u32>,
    window_type: Option<WindowType>,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            sample_rate: None,
            min_freq: None,
            max_freq: None,
            window_size: None,
            quality_level: None,
            duration_ms: None,
            image_width: None,
            image_height: None,
            window_type: None,
        }
    }

    pub fn sample_rate(mut self, sample_rate: f32) -> Self {
        self.sample_rate = Some(sample_rate);
        self
    }

    pub fn frequency_range(mut self, min: f32, max: f32) -> Self {
        self.min_freq = Some(min);
        self.max_freq = Some(max);
        self
    }

    pub fn window_size(mut self, size: usize) -> Self {
        self.window_size = Some(size);
        self
    }

    pub fn quality_level(mut self, level: QualityLevel) -> Self {
        self.quality_level = Some(level);
        self
    }

    pub fn duration_ms(mut self, duration: f32) -> Self {
        self.duration_ms = Some(duration);
        self
    }

    pub fn image_dimensions(mut self, width: u32, height: u32) -> Self {
        self.image_width = Some(width);
        self.image_height = Some(height);
        self
    }

    pub fn window_type(mut self, window_type: WindowType) -> Self {
        self.window_type = Some(window_type);
        self
    }

    pub fn auto_configure(mut self, duration_ms: f32) -> Self {
        self.duration_ms = Some(duration_ms);
        // window_size will be calculated in build based on duration_ms if not explicitly set
        self
    }

    pub fn build(self) -> Result<SpectrogramConfig, SpectrumError> {
        let default_config = SpectrogramConfig::default();

        let sample_rate = self.sample_rate.unwrap_or(default_config.sample_rate);
        let min_freq = self.min_freq.unwrap_or(default_config.min_freq);
        let max_freq = self.max_freq.unwrap_or(default_config.max_freq);
        let image_width = self.image_width.unwrap_or(default_config.image_width);
        let image_height = self.image_height.unwrap_or(default_config.image_height);
        let window_type = self.window_type.unwrap_or(default_config.window_type);
        let analysis_duration_ms = self
            .duration_ms
            .unwrap_or(default_config.analysis_duration_ms);

        let window_size = if let Some(ws) = self.window_size {
            ws
        } else if self.duration_ms.is_some() {
            // Only auto-configure if duration_ms was explicitly set
            SpectrogramConfig::calculate_optimal_window_size(analysis_duration_ms, sample_rate)
        } else {
            default_config.window_size // Fallback to default if no window_size and no duration_ms
        };

        let hop_size = if self.duration_ms.is_some() {
            // If duration is provided (e.g., via auto_configure), use adaptive hop_size
            SpectrogramConfig::calculate_hop_size_for_short_audio(
                window_size,
                SpectrogramConfig::get_duration_preset(analysis_duration_ms),
            )
        } else {
            // Otherwise, use quality level based hop_size
            SpectrogramConfig::calculate_hop_size(
                window_size,
                self.quality_level.unwrap_or(default_config.quality_level()),
            )
        };

        let mut config = SpectrogramConfig {
            window_size,
            hop_size,
            sample_rate,
            min_freq,
            max_freq,
            image_width,
            image_height,
            window_type,
            analysis_duration_ms,
        };

        config.validate()?; // Validate the final config
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_builder_defaults() {
        let config = ConfigBuilder::new().build().unwrap();
        assert_eq!(config.sample_rate, 44100.0);
        assert_eq!(config.min_freq, 20.0);
        assert_eq!(config.max_freq, 20000.0);
        assert_eq!(config.window_size, 2048); // Default for long audio (0.0ms duration)
        assert_eq!(config.hop_size, 512); // Default for standard quality
    }

    #[test]
    fn test_config_builder_custom_values() {
        let config = ConfigBuilder::new()
            .sample_rate(48000.0)
            .frequency_range(100.0, 10000.0)
            .window_size(512)
            .quality_level(QualityLevel::High)
            .image_dimensions(800, 400)
            .window_type(WindowType::Hamming)
            .build()
            .unwrap();

        assert_eq!(config.sample_rate, 48000.0);
        assert_eq!(config.min_freq, 100.0);
        assert_eq!(config.max_freq, 10000.0);
        assert_eq!(config.window_size, 512);
        assert_eq!(config.hop_size, 64); // 87.5% overlap for 512 window
        assert_eq!(config.image_width, 800);
        assert_eq!(config.image_height, 400);
        assert_eq!(config.window_type, WindowType::Hamming);
    }

    #[test]
    fn test_config_builder_auto_configure() {
        let config = ConfigBuilder::new()
            .sample_rate(44100.0)
            .auto_configure(80.0) // 80ms duration
            .build()
            .unwrap();

        assert_eq!(config.window_size, 256); // Optimal for 80ms
        assert_eq!(config.hop_size, 7); // 97% overlap for very short audio
        assert_eq!(config.analysis_duration_ms, 80.0);
    }

    #[test]
    fn test_config_builder_invalid_values() {
        let config = ConfigBuilder::new().sample_rate(-100.0).build();
        assert!(config.is_err());

        let config = ConfigBuilder::new()
            .window_size(100) // Not power of 2
            .build();
        assert!(config.is_err());
    }
}
