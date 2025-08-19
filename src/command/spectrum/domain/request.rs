use std::path::PathBuf;
use crate::command::spectrum::core::config::SpectrogramConfig;
use crate::utils::detection::AutoStartDetection;
use crate::utils::time::TimeRange;

#[derive(Debug, Clone)]
pub struct SpectrumRequest {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub config: SpectrogramConfig,
    pub time_range: Option<TimeRange>,
    pub auto_start: Option<AutoStartDetection>,
    pub annotations: Option<Vec<(f32, String)>>,
    pub options: SpectrumOptions,
}

#[derive(Debug, Clone)]
pub struct SpectrumOptions; // Placeholder for future options
