use crate::core::config::SpectrogramConfig;
use crate::domain::frequency::FrequencyAnnotation;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Spectrogram {
    pub data: Vec<Vec<f32>>,
    pub time_axis: Vec<f32>,
    pub freq_axis: Vec<f32>,
    pub metadata: SpectrogramMetadata,
}

#[derive(Debug, Clone)]
pub struct SpectrogramMetadata {
    pub config: SpectrogramConfig,
    pub input_path: PathBuf,
    pub start_time: f32,
    pub end_time: f32,
    pub analysis_duration_ms: f32,
    pub annotations: Vec<FrequencyAnnotation>,
}
