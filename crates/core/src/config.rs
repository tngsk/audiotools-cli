use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Config {
    pub global: Option<GlobalConfig>,
    pub segment: Option<SegmentConfig>,
    pub features: Option<FeaturesConfig>,
    pub spectrogram: Option<SpectrogramConfig>,
    pub convert: Option<ConvertConfig>,
    pub waveform: Option<WaveformConfig>,
    pub normalize: Option<NormalizeConfig>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct GlobalConfig {
    pub overwrite: Option<bool>,
    pub recursive: Option<bool>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct SegmentConfig {
    pub segment_len: Option<f32>,
    pub top_db: Option<i32>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct FeaturesConfig {
    pub save_individual: Option<bool>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct SpectrogramConfig {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub fmax: Option<f32>,
    pub n_mels: Option<usize>,
    pub n_fft: Option<usize>,
    pub hop_length: Option<usize>,
    pub marker_hz: Option<f32>,
    pub title_fontsize: Option<u32>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ConvertConfig {
    pub format: Option<String>,
    pub subtype: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct WaveformConfig {
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct NormalizeConfig {
    pub level: Option<f32>,
}

impl Config {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: Config =
            serde_yaml::from_str(&content).context("Failed to parse config file")?;
        Ok(config)
    }

    pub fn load_default() -> Result<Self> {
        // Try loading from current dir `config.yaml`
        if Path::new("config.yaml").exists() {
            return Self::load_from_file("config.yaml");
        }
        Ok(Self::default())
    }
}
