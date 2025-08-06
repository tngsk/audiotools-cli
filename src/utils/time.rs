#[derive(Clone, Debug, Copy)]
pub enum TimeSpecification {
    Seconds(f32),             // 秒指定
    MinutesSeconds(u32, u32), // 分:秒指定
}

#[derive(Clone, Debug)]
pub struct TimeRange {
    pub start: TimeSpecification,
    pub end: TimeSpecification,
}

impl TimeRange {
    pub fn resolve(&self, total_duration: f32) -> Result<(f32, f32), String> {
        let resolve_spec = |spec: &TimeSpecification| -> Result<f32, String> {
            match spec {
                TimeSpecification::Seconds(s) => Ok(*s),
                TimeSpecification::MinutesSeconds(m, s) => Ok(*m as f32 * 60.0 + *s as f32),
            }
        };

        let start_time = resolve_spec(&self.start)?;
        let end_time = resolve_spec(&self.end)?;

        // 妥当性チェック
        if start_time >= end_time {
            return Err("Start time must be less than end time".to_string());
        }
        if start_time < 0.0 {
            return Err("Start time must be positive".to_string());
        }
        if end_time > total_duration {
            return Err(format!(
                "End time ({}) exceeds audio duration ({})",
                end_time, total_duration
            ));
        }

        Ok((start_time, end_time))
    }
}

pub fn create_time_range(
    start: Option<TimeSpecification>,
    end: Option<TimeSpecification>,
) -> Option<TimeRange> {
    if start.is_some() || end.is_some() {
        Some(TimeRange {
            start: start.unwrap_or(TimeSpecification::Seconds(0.0)),
            end: end.unwrap_or(TimeSpecification::Seconds(f32::MAX)),
        })
    } else {
        None
    }
}

/// 時間指定文字列をパース
pub fn parse_time_specification(time_str: &str) -> Result<TimeSpecification, String> {
    if time_str.contains(':') {
        // 分:秒指定
        let parts: Vec<&str> = time_str.split(':').collect();
        if parts.len() != 2 {
            return Err("Invalid time format. Use MM:SS".to_string());
        }
        let minutes = parts[0].parse::<u32>().map_err(|_| "Invalid minutes")?;
        let seconds = parts[1].parse::<u32>().map_err(|_| "Invalid seconds")?;
        if seconds >= 60 {
            return Err("Seconds must be less than 60".to_string());
        }
        Ok(TimeSpecification::MinutesSeconds(minutes, seconds))
    } else {
        // 秒指定
        let seconds = time_str.parse::<f32>().map_err(|_| "Invalid seconds")?;
        if seconds < 0.0 {
            return Err("Seconds must be positive".to_string());
        }
        Ok(TimeSpecification::Seconds(seconds))
    }
}
