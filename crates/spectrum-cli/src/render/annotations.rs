use crate::core::config::SpectrogramConfig;
use crate::domain::frequency::FrequencyAnnotation;
use crate::error::SpectrumError;
use plotters::prelude::*;

// Constants (consider moving to a dedicated constants module or make configurable)
const FONT_FAMILY: &str = "Fira Code";

/// Draw frequency annotations
pub fn draw_annotations(
    chart: &mut ChartContext<
        BitMapBackend,
        Cartesian2d<plotters::coord::types::RangedCoordf32, plotters::coord::types::RangedCoordf32>,
    >,
    annotations: &[FrequencyAnnotation],
    config: &SpectrogramConfig,
    total_time: f32,
) -> Result<(), SpectrumError> {
    for annotation in annotations.iter() {
        if annotation.frequency >= config.min_freq && annotation.frequency <= config.max_freq {
            // Draw horizontal line
            chart
                .draw_series(LineSeries::new(
                    vec![
                        (0.0, annotation.frequency),
                        (total_time, annotation.frequency),
                    ],
                    GREEN.stroke_width(2),
                ))
                .map_err(|e| SpectrumError::new(e.to_string()))?;

            // Draw label
            chart
                .draw_series(std::iter::once(Text::new(
                    annotation.label.clone(),
                    (
                        total_time * 0.95,
                        annotation.frequency + config.freq_resolution() * 5.0,
                    ),
                    (FONT_FAMILY, 14).into_font().color(&GREEN),
                )))
                .map_err(|e| SpectrumError::new(e.to_string()))?;
        }
    }

    Ok(())
}
