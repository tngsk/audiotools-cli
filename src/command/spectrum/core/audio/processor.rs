use crate::utils::detection::AutoStartDetection;
use crate::utils::time::{TimeRange, TimeSpecification};
use crate::command::spectrum::error::SpectrumError;

/// Process time range with auto-detection support
pub fn process_time_range(
    samples: &[f32],
    sample_rate: f32,
    time_range: Option<TimeRange>,
    auto_start: Option<AutoStartDetection>,
    total_duration: f32,
) -> Result<(f32, f32), SpectrumError> {
    let (start_time, end_time) = if let Some(auto_config) = auto_start {
        let detected_start = auto_config
            .detect_start_time(samples, sample_rate)
            .ok_or_else(|| {
                SpectrumError::TimeRange("Failed to detect start time".to_string())
            })?;

        let end_time = if let Some(range) = time_range {
            TimeRange {
                start: TimeSpecification::Seconds(detected_start),
                end: range.end,
            }
            .resolve(total_duration)
            .map_or(total_duration, |(_, end)| end)
        } else {
            total_duration
        };

        (detected_start, end_time)
    } else if let Some(range) = time_range {
        range
            .resolve(total_duration)
            .map_err(|e| SpectrumError::TimeRange(format!("Invalid time range: {}", e)))?
    } else {
        (0.0, total_duration)
    };

    // Basic validation
    if start_time < 0.0 || end_time <= start_time || start_time >= total_duration {
        return Err(SpectrumError::TimeRange(
            "Invalid time range".to_string(),
        ));
    }

    Ok((start_time, end_time))
}
