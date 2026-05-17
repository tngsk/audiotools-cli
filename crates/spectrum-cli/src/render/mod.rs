pub mod annotations;
pub mod canvas;
pub mod colormap;
pub mod layout;

use crate::core::SpectrogramRenderer;
use crate::core::config::SpectrogramConfig;
use crate::domain::spectrogram::Spectrogram;
use crate::error::SpectrumError;
use crate::render::annotations::draw_annotations;
use crate::render::colormap::power_to_color;
use plotters::prelude::*;
use std::path::Path;

// Constants (consider moving to a dedicated constants module or make configurable)
const FONT_FAMILY: &str = "Fira Code";
const BACKGROUND_COLOR: RGBColor = RGBColor(4, 20, 36);

pub struct DefaultSpectrogramRenderer;

impl SpectrogramRenderer for DefaultSpectrogramRenderer {
    fn render(&self, spectrogram: &Spectrogram, output: &Path) -> Result<(), SpectrumError> {
        if spectrogram.data.is_empty() {
            return Err(SpectrumError::new("No spectrogram data to render"));
        }

        let config = &spectrogram.metadata.config; // Assuming config is part of metadata
        let input_path = &spectrogram.metadata.input_path; // Assuming input_path is part of metadata
        let start_time = spectrogram.metadata.start_time; // Assuming start_time is part of metadata
        let end_time = spectrogram.metadata.end_time; // Assuming end_time is part of metadata
        let analysis_duration_ms = spectrogram.metadata.analysis_duration_ms; // Assuming analysis_duration_ms is part of metadata
        let annotations = &spectrogram.metadata.annotations; // Assuming annotations is part of metadata

        let root = BitMapBackend::new(
            output.to_str().unwrap(),
            (config.image_width, config.image_height),
        )
        .into_drawing_area();

        root.fill(&BACKGROUND_COLOR)
            .map_err(|e| SpectrumError::new(e.to_string()))?;

        let filename = input_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Spectrogram");

        let title = format!("{} ({:.1}ms)", filename, analysis_duration_ms);

        let total_time = end_time - start_time;
        let time_per_frame = if spectrogram.data.len() > 1 {
            total_time / (spectrogram.data.len() - 1) as f32
        } else {
            total_time
        };

        // Create chart
        let mut chart = ChartBuilder::on(&root)
            .margin(40)
            .caption(title, (FONT_FAMILY, 24).into_font().color(&WHITE))
            .set_label_area_size(LabelAreaPosition::Left, 60)
            .set_label_area_size(LabelAreaPosition::Bottom, 40)
            .build_cartesian_2d(0.0..total_time, config.min_freq..config.max_freq)
            .map_err(|e| SpectrumError::new(e.to_string()))?;

        chart
            .configure_mesh()
            .label_style((FONT_FAMILY, 14).into_font().color(&WHITE))
            .light_line_style(WHITE.mix(0.05))
            .axis_style(WHITE.mix(0.5))
            .x_labels(20)
            .x_label_formatter(&|x| format!("{:.1}", x))
            .y_desc("Frequency (Hz)")
            .x_desc("Time (s)")
            .y_labels(10)
            .y_label_formatter(&|y| format!("{:.0}", y))
            .draw()
            .map_err(|e| SpectrumError::new(e.to_string()))?;

        // Draw spectrogram data with improved resolution
        draw_spectrogram_data(
            &mut chart,
            &spectrogram.data,
            config,
            time_per_frame,
            analysis_duration_ms,
        )?;

        // Draw annotations
        if !annotations.is_empty() {
            draw_annotations(&mut chart, annotations, config, total_time)?;
        }

        root.present()
            .map_err(|e| SpectrumError::new(e.to_string()))?;

        Ok(())
    }
}

/// Draw spectrogram data using optimized Rectangle rendering
fn draw_spectrogram_data(
    chart: &mut ChartContext<
        BitMapBackend,
        Cartesian2d<plotters::coord::types::RangedCoordf32, plotters::coord::types::RangedCoordf32>,
    >,
    spectrogram_data: &[Vec<f32>],
    config: &SpectrogramConfig,
    time_per_frame: f32,
    analysis_duration_ms: f32,
) -> Result<(), SpectrumError> {
    let freq_resolution = config.freq_resolution();
    let (min_db, max_db) = get_adaptive_db_range(config.window_size);
    let inv_db_range = 1.0 / (max_db - min_db);
    let use_interpolation = analysis_duration_ms < 300.0 && spectrogram_data.len() > 30;

    // Use exact steps without artificial overlap to prevent gaps and artifacts
    let time_step = if use_interpolation {
        // Slightly overlap for smoother rendering
        time_per_frame * 1.1
    } else {
        time_per_frame
    };
    let freq_step = freq_resolution;

    // Apply interpolation for smooth rendering if needed
    if use_interpolation && spectrogram_data.len() > 1 {
        draw_interpolated_spectrogram(
            chart,
            spectrogram_data,
            config,
            time_per_frame,
            analysis_duration_ms,
        )?;
    } else {
        // Standard rendering for longer audio
        for (frame_idx, spectrum) in spectrogram_data.iter().enumerate() {
            let time_start = frame_idx as f32 * time_per_frame;
            let time_end = time_start + time_step;

            for (bin, &power_db) in spectrum.iter().enumerate() {
                // Calculate frequency directly from bin index
                let freq_start = bin as f32 * freq_resolution;
                let freq_end = freq_start + freq_step;

                // Only render frequencies within our range
                if freq_start >= config.min_freq && freq_start <= config.max_freq {
                    let normalized_power = ((power_db - min_db) * inv_db_range).clamp(0.0, 1.0);

                    // Render all power levels for complete coverage
                    if normalized_power > 0.001 {
                        let color = power_to_color(normalized_power);

                        chart
                            .draw_series(std::iter::once(Rectangle::new(
                                [(time_start, freq_start), (time_end, freq_end)],
                                color.filled(),
                            )))
                            .map_err(|e| SpectrumError::new(e.to_string()))?;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Draw spectrogram with interpolation for smooth rendering
fn draw_interpolated_spectrogram(
    chart: &mut ChartContext<
        BitMapBackend,
        Cartesian2d<plotters::coord::types::RangedCoordf32, plotters::coord::types::RangedCoordf32>,
    >,
    spectrogram_data: &[Vec<f32>],
    config: &SpectrogramConfig,
    time_per_frame: f32,
    _analysis_duration_ms: f32,
) -> Result<(), SpectrumError> {
    let freq_resolution = config.freq_resolution();
    let (min_db, max_db) = get_adaptive_db_range(config.window_size);
    let inv_db_range = 1.0 / (max_db - min_db);

    // Simple linear interpolation between frames
    for (frame_idx, window) in spectrogram_data.windows(2).enumerate() {
        let current = &window[0];
        let next = &window[1];

        for step in 0..=2 {
            let t = step as f32 / 2.0;
            let time = (frame_idx as f32 + t) * time_per_frame;
            let time_end = time + time_per_frame / 2.0;

            for (bin, (&curr_power, &next_power)) in current.iter().zip(next.iter()).enumerate() {
                let freq = bin as f32 * freq_resolution;
                if freq < config.min_freq || freq > config.max_freq {
                    continue;
                }

                let power = curr_power + (next_power - curr_power) * t;
                let normalized = ((power - min_db) * inv_db_range).clamp(0.0, 1.0);

                if normalized > 0.001 {
                    chart
                        .draw_series(std::iter::once(Rectangle::new(
                            [(time, freq), (time_end, freq + freq_resolution)],
                            power_to_color(normalized).filled(),
                        )))
                        .map_err(|e| SpectrumError::new(e.to_string()))?;
                }
            }
        }
    }

    // Draw last frame
    if let Some(last) = spectrogram_data.last() {
        let time = (spectrogram_data.len() - 1) as f32 * time_per_frame;
        for (bin, &power) in last.iter().enumerate() {
            let freq = bin as f32 * freq_resolution;
            if freq >= config.min_freq && freq <= config.max_freq {
                let normalized = ((power - min_db) * inv_db_range).clamp(0.0, 1.0);
                if normalized > 0.001 {
                    chart
                        .draw_series(std::iter::once(Rectangle::new(
                            [
                                (time, freq),
                                (time + time_per_frame, freq + freq_resolution),
                            ],
                            power_to_color(normalized).filled(),
                        )))
                        .map_err(|e| SpectrumError::new(e.to_string()))?;
                }
            }
        }
    }

    Ok(())
}

/// Get adaptive dB range based on window size
fn get_adaptive_db_range(window_size: usize) -> (f32, f32) {
    match window_size {
        w if w <= 256 => (-90.0, -10.0),
        w if w <= 512 => (-100.0, -5.0),
        _ => (-120.0, 0.0),
    }
}
