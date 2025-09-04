use crate::error::SpectrumError;
use crate::utils::detection::AutoStartDetection;
use crate::utils::time::{TimeRange, TimeSpecification};

// Process time range for samples
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
            .ok_or_else(|| SpectrumError::new("Failed to detect audio start time"))?;

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
            .map_err(|e| SpectrumError::new(format!("Invalid time range: {}", e)))?
    } else {
        (0.0, total_duration)
    };

    // Basic validation
    if start_time < 0.0 || end_time <= start_time || start_time >= total_duration {
        return Err(SpectrumError::new("Invalid time range"));
    }

    Ok((start_time, end_time))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_time_range_no_options() {
        let samples = vec![0.0; 44100]; // 1 second at 44.1kHz
        let sample_rate = 44100.0;
        let total_duration = 1.0;

        let result = process_time_range(&samples, sample_rate, None, None, total_duration);

        assert!(result.is_ok());
        let (start, end) = result.unwrap();
        assert_eq!(start, 0.0);
        assert_eq!(end, 1.0);
    }

    #[test]
    fn test_process_time_range_with_range() {
        let samples = vec![0.0; 441000]; // 10 seconds at 44.1kHz
        let sample_rate = 44100.0;
        let total_duration = 10.0;

        let time_range = Some(TimeRange {
            start: TimeSpecification::Seconds(2.0),
            end: TimeSpecification::Seconds(5.0),
        });

        let result = process_time_range(&samples, sample_rate, time_range, None, total_duration);

        assert!(result.is_ok());
        let (start, end) = result.unwrap();
        assert_eq!(start, 2.0);
        assert_eq!(end, 5.0);
    }

    #[test]
    fn test_process_time_range_with_minutes_seconds() {
        let samples = vec![0.0; 441000]; // 10 seconds at 44.1kHz
        let sample_rate = 44100.0;
        let total_duration = 10.0;

        let time_range = Some(TimeRange {
            start: TimeSpecification::MinutesSeconds(0, 1), // 0:01 = 1 second
            end: TimeSpecification::MinutesSeconds(0, 5),   // 0:05 = 5 seconds
        });

        let result = process_time_range(&samples, sample_rate, time_range, None, total_duration);

        assert!(result.is_ok());
        let (start, end) = result.unwrap();
        assert_eq!(start, 1.0);
        assert_eq!(end, 5.0);
    }

    #[test]
    fn test_process_time_range_with_milliseconds() {
        let samples = vec![0.0; 44100]; // 1 second at 44.1kHz
        let sample_rate = 44100.0;
        let total_duration = 1.0;

        let time_range = Some(TimeRange {
            start: TimeSpecification::Seconds(0.1), // 0.1 seconds
            end: TimeSpecification::Seconds(0.5),   // 0.5 seconds
        });

        let result = process_time_range(&samples, sample_rate, time_range, None, total_duration);

        assert!(result.is_ok());
        let (start, end) = result.unwrap();
        assert_eq!(start, 0.1);
        assert_eq!(end, 0.5);
    }

    #[test]
    fn test_process_time_range_invalid_range() {
        let samples = vec![0.0; 44100]; // 1 second at 44.1kHz
        let sample_rate = 44100.0;
        let total_duration = 1.0;

        // End before start
        let time_range = Some(TimeRange {
            start: TimeSpecification::Seconds(0.8),
            end: TimeSpecification::Seconds(0.2),
        });

        let result = process_time_range(&samples, sample_rate, time_range, None, total_duration);
        assert!(result.is_err());
    }

    #[test]
    fn test_process_time_range_out_of_bounds() {
        let samples = vec![0.0; 44100]; // 1 second at 44.1kHz
        let sample_rate = 44100.0;
        let total_duration = 1.0;

        // Start after end of file
        let time_range = Some(TimeRange {
            start: TimeSpecification::Seconds(2.0),
            end: TimeSpecification::Seconds(3.0),
        });

        let result = process_time_range(&samples, sample_rate, time_range, None, total_duration);
        assert!(result.is_err());
    }

    #[test]
    fn test_process_time_range_with_auto_detect() {
        // Create samples with silence then signal
        let mut samples = vec![0.0; 4410]; // 0.1 seconds silence
        samples.extend(vec![0.5; 39690]); // 0.9 seconds signal
        let sample_rate = 44100.0;
        let total_duration = 1.0;

        let auto_start = Some(AutoStartDetection {
            threshold: 0.01,
            window_size: 441,   // 10ms at 44.1kHz
            min_duration: 0.02, // 20ms
        });

        let result = process_time_range(&samples, sample_rate, None, auto_start, total_duration);

        assert!(result.is_ok());
        let (start, end) = result.unwrap();
        // Should detect start around 0.1 seconds
        assert!(start >= 0.09 && start <= 0.11);
        assert_eq!(end, 1.0);
    }

    #[test]
    fn test_process_time_range_auto_detect_with_time_range() {
        // Create samples with silence then signal
        let mut samples = vec![0.0; 4410]; // 0.1 seconds silence
        samples.extend(vec![0.5; 435690]); // 9.9 seconds signal
        let sample_rate = 44100.0;
        let total_duration = 10.0;

        let auto_start = Some(AutoStartDetection {
            threshold: 0.01,
            window_size: 441,   // 10ms at 44.1kHz
            min_duration: 0.02, // 20ms
        });

        let time_range = Some(TimeRange {
            start: TimeSpecification::Seconds(0.0), // Will be overridden by auto-detect
            end: TimeSpecification::Seconds(5.0),
        });

        let result = process_time_range(
            &samples,
            sample_rate,
            time_range,
            auto_start,
            total_duration,
        );

        assert!(result.is_ok());
        let (start, end) = result.unwrap();
        // Should detect start around 0.1 seconds
        assert!(start >= 0.09 && start <= 0.11);
        assert_eq!(end, 5.0);
    }

    #[test]
    fn test_process_time_range_negative_start() {
        let samples = vec![0.0; 44100];
        let sample_rate = 44100.0;
        let total_duration = 1.0;

        let time_range = Some(TimeRange {
            start: TimeSpecification::Seconds(-1.0),
            end: TimeSpecification::Seconds(0.5),
        });

        let result = process_time_range(&samples, sample_rate, time_range, None, total_duration);
        assert!(result.is_err());
    }
}
