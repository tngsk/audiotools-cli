use crate::core::config::SpectrogramConfig;
use crate::domain::frequency::FrequencyAnnotation;
use crate::utils::detection::AutoStartDetection;
use crate::utils::time::TimeRange;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SpectrumRequest {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub config: SpectrogramConfig,
    pub time_range: Option<TimeRange>,
    pub auto_start: Option<AutoStartDetection>,
    pub annotations: Vec<FrequencyAnnotation>,
    pub options: SpectrumOptions,
}

#[derive(Debug, Clone, Default)]
pub struct SpectrumOptions; // Placeholder for future options
