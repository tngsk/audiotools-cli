//! Simplified spectrum analysis and spectrogram generation module

pub mod config;
pub mod error;
pub mod fft;

// Re-export main types
pub use config::{FrequencyPreset, QualityLevel, SpectrogramConfig, WindowType};
pub use error::{Result, SpectrogramError};
pub use fft::FFTProcessor;

use crate::utils::detection::AutoStartDetection;
use crate::utils::get_walker;
use crate::utils::time::{TimeRange, TimeSpecification};
use hound::WavReader;
use plotters::prelude::*;
use std::path::PathBuf;

// Constants
const FONT_FAMILY: &str = "Fira Code";
const BACKGROUND_COLOR: RGBColor = RGBColor(4, 20, 36);
const MIN_DB: f32 = -120.0;
const MAX_DB: f32 = 0.0;

/// Parse frequency annotation string (freq:label)
pub fn parse_frequency_annotation(s: &str) -> Result<(f32, String)> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err(SpectrogramError::InvalidInput(
            "Annotation format should be 'frequency:label'".to_string(),
        ));
    }

    let freq = parts[0]
        .parse::<f32>()
        .map_err(|_| SpectrogramError::InvalidInput("Invalid frequency value".to_string()))?;

    Ok((freq, parts[1].to_string()))
}

/// Get frequency range from preset
pub fn get_frequency_preset(preset: FrequencyPreset, sample_rate: f32) -> (f32, f32) {
    SpectrogramConfig::frequency_preset(preset, sample_rate)
}

/// Create spectrograms for multiple files (legacy API)
pub fn create_spectrograms(
    input: &PathBuf,
    window_size: usize,
    overlap: f32,
    min_freq: f32,
    max_freq: f32,
    time_range: Option<TimeRange>,
    auto_start: Option<AutoStartDetection>,
    recursive: bool,
    annotations: Option<Vec<(f32, String)>>,
) -> Vec<PathBuf> {
    let mut output_files = Vec::new();

    for entry in get_walker(input, recursive) {
        if let Some(ext) = entry.path().extension() {
            if ext.to_string_lossy().to_lowercase() == "wav" {
                let input_path = PathBuf::from(entry.path());
                let output_path = input_path.with_extension("png");

                match create_spectrogram(
                    &input_path,
                    &output_path,
                    window_size,
                    overlap,
                    min_freq,
                    max_freq,
                    time_range.clone(),
                    auto_start.clone(),
                    annotations.clone(),
                ) {
                    Ok(_) => {
                        println!(
                            "Created spectrogram: {} -> {}",
                            input_path.display(),
                            output_path.display()
                        );
                        output_files.push(output_path);
                    }
                    Err(e) => {
                        eprintln!("Error processing {}: {}", input_path.display(), e);
                    }
                }
            }
        }
    }

    output_files
}

/// Create a spectrogram with the simplified system
pub fn create_spectrogram(
    input: &PathBuf,
    output: &PathBuf,
    window_size: usize,
    overlap: f32,
    min_freq: f32,
    max_freq: f32,
    time_range: Option<TimeRange>,
    auto_start: Option<AutoStartDetection>,
    annotations: Option<Vec<(f32, String)>>,
) -> Result<()> {
    // Load audio data
    let audio_data = load_audio_samples(input)?;
    let total_duration = audio_data.samples.len() as f32 / audio_data.sample_rate;

    // Process time range first to get actual analysis duration
    let (start_time, end_time) = process_time_range(
        &audio_data.samples,
        audio_data.sample_rate,
        time_range,
        auto_start,
        total_duration,
    )?;

    // Calculate actual analysis duration in milliseconds
    let analysis_duration = end_time - start_time;
    let analysis_duration_ms = analysis_duration * 1000.0;

    // Determine if we should use adaptive configuration based on actual analysis duration
    let (config, mode_description) = if window_size == 0 {
        // Auto-configure based on actual analysis duration
        let cfg = SpectrogramConfig::auto_configure(
            audio_data.sample_rate,
            min_freq,
            max_freq,
            analysis_duration_ms,
        )?;
        (cfg, "auto-configured")
    } else {
        // Use explicitly specified window size, regardless of duration
        let cfg = SpectrogramConfig::from_legacy_params_with_duration(
            window_size,
            overlap,
            min_freq,
            max_freq,
            audio_data.sample_rate,
            Some(analysis_duration_ms),
        )?;
        (cfg, "user-specified")
    };

    println!(
        "Analysis: {:.1}ms ({:.3}s to {:.3}s) | Mode: {} | Window: {} | Hop: {} | Overlap: {:.1}%",
        analysis_duration_ms,
        start_time,
        end_time,
        mode_description,
        config.window_size,
        config.hop_size,
        (1.0 - (config.hop_size as f32 / config.window_size as f32)) * 100.0
    );

    // Extract sample range
    let start_sample = (start_time * audio_data.sample_rate) as usize;
    let end_sample = (end_time * audio_data.sample_rate) as usize;
    let samples = &audio_data.samples[start_sample..end_sample.min(audio_data.samples.len())];

    // Generate spectrogram with padding for short audio
    let fft_processor = FFTProcessor::new(config.clone());
    let spectrogram_data = if analysis_duration_ms < 150.0 {
        // Add substantial padding for very short analysis (30% padding)
        fft_processor.process_signal_with_padding(samples, 0.3)?
    } else if analysis_duration_ms < 300.0 {
        // Add moderate padding for short analysis (20% padding)
        fft_processor.process_signal_with_padding(samples, 0.2)?
    } else if analysis_duration_ms < 500.0 {
        // Add light padding for medium-short analysis (10% padding)
        fft_processor.process_signal_with_padding(samples, 0.1)?
    } else {
        // No padding for longer analysis
        fft_processor.process_signal(samples)?
    };

    // Render to image
    render_spectrogram(
        &spectrogram_data,
        &config,
        output,
        input,
        start_time,
        end_time,
        analysis_duration_ms,
        annotations,
    )?;

    Ok(())
}

/// Audio data structure
#[derive(Debug)]
struct AudioData {
    samples: Vec<f32>,
    sample_rate: f32,
}

/// Load audio samples from WAV file
fn load_audio_samples(input: &PathBuf) -> Result<AudioData> {
    let mut reader = WavReader::open(input)?;
    let spec = reader.spec();
    let sample_rate = spec.sample_rate as f32;

    // Convert to mono f32 samples
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => {
            let samples: std::result::Result<Vec<f32>, hound::Error> =
                reader.samples::<f32>().collect();
            samples?
                .chunks(spec.channels as usize)
                .map(|chunk| chunk.iter().sum::<f32>() / chunk.len() as f32)
                .collect()
        }
        hound::SampleFormat::Int => {
            let bits = spec.bits_per_sample;
            let max_value = (1 << (bits - 1)) as f32;

            let samples: std::result::Result<Vec<i32>, hound::Error> =
                reader.samples::<i32>().collect();
            samples?
                .chunks(spec.channels as usize)
                .map(|chunk| chunk.iter().sum::<i32>() as f32 / (chunk.len() as f32 * max_value))
                .collect()
        }
    };

    Ok(AudioData {
        samples,
        sample_rate,
    })
}

/// Process time range with auto-detection support
fn process_time_range(
    samples: &[f32],
    sample_rate: f32,
    time_range: Option<TimeRange>,
    auto_start: Option<AutoStartDetection>,
    total_duration: f32,
) -> Result<(f32, f32)> {
    let (start_time, end_time) = if let Some(auto_config) = auto_start {
        let detected_start = auto_config
            .detect_start_time(samples, sample_rate)
            .ok_or_else(|| {
                SpectrogramError::InvalidInput("Failed to detect start time".to_string())
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
            .map_err(|e| SpectrogramError::InvalidInput(format!("Invalid time range: {}", e)))?
    } else {
        (0.0, total_duration)
    };

    // Basic validation
    if start_time < 0.0 || end_time <= start_time || start_time >= total_duration {
        return Err(SpectrogramError::InvalidInput(
            "Invalid time range".to_string(),
        ));
    }

    Ok((start_time, end_time))
}

/// Render spectrogram to image file
fn render_spectrogram(
    spectrogram_data: &[Vec<f32>],
    config: &SpectrogramConfig,
    output: &PathBuf,
    input: &PathBuf,
    start_time: f32,
    end_time: f32,
    analysis_duration_ms: f32,
    annotations: Option<Vec<(f32, String)>>,
) -> Result<()> {
    if spectrogram_data.is_empty() {
        return Err(SpectrogramError::InvalidInput(
            "No spectrogram data to render".to_string(),
        ));
    }

    let root = BitMapBackend::new(
        output.to_str().unwrap(),
        (config.image_width, config.image_height),
    )
    .into_drawing_area();

    root.fill(&BACKGROUND_COLOR)
        .map_err(|e| SpectrogramError::InvalidInput(e.to_string()))?;

    let filename = input
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Spectrogram");

    let title = format!("{} ({:.1}ms)", filename, analysis_duration_ms);

    let total_time = end_time - start_time;
    let time_per_frame = if spectrogram_data.len() > 1 {
        total_time / (spectrogram_data.len() - 1) as f32
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
        .map_err(|e| SpectrogramError::InvalidInput(e.to_string()))?;

    chart
        .configure_mesh()
        .label_style((FONT_FAMILY, 14).into_font().color(&WHITE))
        .light_line_style(&WHITE.mix(0.05))
        .axis_style(&WHITE.mix(0.5))
        .x_labels(20)
        .x_label_formatter(&|x| format!("{:.1}", x))
        .y_desc("Frequency (Hz)")
        .x_desc("Time (s)")
        .y_labels(10)
        .y_label_formatter(&|y| format!("{:.0}", y))
        .draw()
        .map_err(|e| SpectrogramError::InvalidInput(e.to_string()))?;

    // Draw spectrogram data with improved resolution
    draw_spectrogram_data(
        &mut chart,
        spectrogram_data,
        config,
        time_per_frame,
        analysis_duration_ms,
    )?;

    // Draw annotations
    if let Some(annotations) = annotations {
        draw_annotations(&mut chart, &annotations, config, total_time)?;
    }

    root.present()
        .map_err(|e| SpectrogramError::InvalidInput(e.to_string()))?;

    Ok(())
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
) -> Result<()> {
    let freq_resolution = config.freq_resolution();

    // For very short analysis with many frames, use interpolated rendering
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
                    let normalized_power =
                        ((power_db - MIN_DB) / (MAX_DB - MIN_DB)).max(0.0).min(1.0);

                    // Render all power levels for complete coverage
                    if normalized_power > 0.001 {
                        let color = power_to_color(normalized_power);

                        chart
                            .draw_series(std::iter::once(Rectangle::new(
                                [(time_start, freq_start), (time_end, freq_end)],
                                color.filled(),
                            )))
                            .map_err(|e| SpectrogramError::InvalidInput(e.to_string()))?;
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
) -> Result<()> {
    let freq_resolution = config.freq_resolution();

    // Render with cubic interpolation between frames for ultra-smooth transitions
    let num_frames = spectrogram_data.len();
    let interpolation_steps = 3; // Number of interpolated points between frames

    for frame_idx in 0..num_frames - 1 {
        let current_spectrum = &spectrogram_data[frame_idx];
        let next_spectrum = &spectrogram_data[frame_idx + 1];

        // Get previous and next-next frames for cubic interpolation (if available)
        let prev_spectrum = if frame_idx > 0 {
            Some(&spectrogram_data[frame_idx - 1])
        } else {
            None
        };
        let next_next_spectrum = if frame_idx + 2 < num_frames {
            Some(&spectrogram_data[frame_idx + 2])
        } else {
            None
        };

        // Create multiple interpolated frames between current and next
        for interp_step in 0..=interpolation_steps {
            let t = interp_step as f32 / interpolation_steps as f32;
            let time_pos = (frame_idx as f32 + t) * time_per_frame;
            let time_next = ((frame_idx as f32 + t) + (1.0 / interpolation_steps as f32))
                .min((num_frames - 1) as f32)
                * time_per_frame;

            for bin in 0..current_spectrum.len().min(next_spectrum.len()) {
                let freq_start = bin as f32 * freq_resolution;
                let freq_end = freq_start + freq_resolution;

                if freq_start >= config.min_freq && freq_start <= config.max_freq {
                    // Cubic interpolation for smoother transitions
                    let power = if let (Some(prev), Some(next_next)) =
                        (prev_spectrum, next_next_spectrum)
                    {
                        if bin < prev.len() && bin < next_next.len() {
                            // Catmull-Rom cubic interpolation
                            let p0 = prev[bin];
                            let p1 = current_spectrum[bin];
                            let p2 = next_spectrum[bin];
                            let p3 = next_next[bin];

                            let t2 = t * t;
                            let t3 = t2 * t;

                            0.5 * ((2.0 * p1)
                                + (-p0 + p2) * t
                                + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
                                + (-p0 + 3.0 * p1 - 3.0 * p2 + p3) * t3)
                        } else {
                            // Fallback to linear interpolation
                            current_spectrum[bin] * (1.0 - t) + next_spectrum[bin] * t
                        }
                    } else {
                        // Linear interpolation at boundaries
                        current_spectrum[bin] * (1.0 - t) + next_spectrum[bin] * t
                    };

                    let normalized_power = ((power - MIN_DB) / (MAX_DB - MIN_DB)).max(0.0).min(1.0);

                    if normalized_power > 0.001 {
                        let color = power_to_color(normalized_power);

                        chart
                            .draw_series(std::iter::once(Rectangle::new(
                                [(time_pos, freq_start), (time_next, freq_end)],
                                color.filled(),
                            )))
                            .map_err(|e| SpectrogramError::InvalidInput(e.to_string()))?;
                    }
                }
            }
        }
    }

    // Draw the last frame
    if let Some(last_spectrum) = spectrogram_data.last() {
        let last_frame_idx = spectrogram_data.len() - 1;
        let time_start = last_frame_idx as f32 * time_per_frame;
        let time_end = time_start + time_per_frame;

        for (bin, &power_db) in last_spectrum.iter().enumerate() {
            let freq_start = bin as f32 * freq_resolution;
            let freq_end = freq_start + freq_resolution;

            if freq_start >= config.min_freq && freq_start <= config.max_freq {
                let normalized_power = ((power_db - MIN_DB) / (MAX_DB - MIN_DB)).max(0.0).min(1.0);

                if normalized_power > 0.001 {
                    let color = power_to_color(normalized_power);

                    chart
                        .draw_series(std::iter::once(Rectangle::new(
                            [(time_start, freq_start), (time_end, freq_end)],
                            color.filled(),
                        )))
                        .map_err(|e| SpectrogramError::InvalidInput(e.to_string()))?;
                }
            }
        }
    }

    Ok(())
}

/// Convert power value to color with improved smooth mapping
fn power_to_color(normalized_power: f32) -> RGBColor {
    let power = normalized_power.max(0.0).min(1.0);

    // Improved smooth heat map with better contrast
    if power < 0.1 {
        // Very low power - dark blue to black
        let ratio = power * 10.0;
        RGBColor(0, 0, (64.0 * ratio) as u8)
    } else if power < 0.4 {
        // Low power - blue to cyan
        let ratio = (power - 0.1) / 0.3;
        RGBColor(0, (128.0 * ratio) as u8, 64 + (191.0 * ratio) as u8)
    } else if power < 0.7 {
        // Medium power - cyan to yellow
        let ratio = (power - 0.4) / 0.3;
        RGBColor(
            (255.0 * ratio) as u8,
            128 + (127.0 * ratio) as u8,
            (255.0 * (1.0 - ratio)) as u8,
        )
    } else {
        // High power - yellow to white
        let ratio = (power - 0.7) / 0.3;
        RGBColor(255, 255, (255.0 * ratio) as u8)
    }
}

/// Draw frequency annotations
fn draw_annotations(
    chart: &mut ChartContext<
        BitMapBackend,
        Cartesian2d<plotters::coord::types::RangedCoordf32, plotters::coord::types::RangedCoordf32>,
    >,
    annotations: &[(f32, String)],
    config: &SpectrogramConfig,
    total_time: f32,
) -> Result<()> {
    for (freq, label) in annotations.iter() {
        if *freq >= config.min_freq && *freq <= config.max_freq {
            // Draw horizontal line
            chart
                .draw_series(LineSeries::new(
                    vec![(0.0, *freq), (total_time, *freq)],
                    GREEN.stroke_width(2),
                ))
                .map_err(|e| SpectrogramError::InvalidInput(e.to_string()))?;

            // Draw label
            chart
                .draw_series(std::iter::once(Text::new(
                    label.clone(),
                    (total_time * 0.95, *freq + config.freq_resolution() * 5.0),
                    (FONT_FAMILY, 14).into_font().color(&GREEN),
                )))
                .map_err(|e| SpectrogramError::InvalidInput(e.to_string()))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frequency_annotation() {
        let result = parse_frequency_annotation("440:A4");
        assert!(result.is_ok());
        let (freq, label) = result.unwrap();
        assert_eq!(freq, 440.0);
        assert_eq!(label, "A4");

        let result = parse_frequency_annotation("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_power_to_color() {
        let very_low = power_to_color(0.0);
        let low = power_to_color(0.4);
        let medium = power_to_color(0.7);
        let high = power_to_color(1.0);

        // Test the new smooth color mapping
        assert_eq!(very_low, RGBColor(0, 0, 0)); // Dark blue/black for very low power
        assert_eq!(low, RGBColor(0, 128, 255)); // Cyan for low power
        assert_eq!(medium, RGBColor(255, 255, 0)); // Yellow for medium power
        assert_eq!(high, RGBColor(255, 255, 255)); // White for high power
    }

    #[test]
    fn test_frequency_presets() {
        let (min, max) = get_frequency_preset(FrequencyPreset::SpeechRange, 44100.0);
        assert_eq!(min, 80.0);
        assert_eq!(max, 8000.0);

        let (min, max) = get_frequency_preset(FrequencyPreset::Bass, 44100.0);
        assert_eq!(min, 60.0);
        assert_eq!(max, 250.0);
    }

    #[test]
    fn test_time_range_adaptive_config() {
        use crate::utils::time::{TimeRange, TimeSpecification};

        // Create a mock audio data (10 seconds)
        let sample_rate = 44100.0;
        let samples = vec![0.0; (10.0 * sample_rate) as usize];
        let total_duration = 10.0;

        // Test short time range (100ms) within long audio
        let time_range = Some(TimeRange {
            start: TimeSpecification::Seconds(5.0),
            end: TimeSpecification::Seconds(5.1), // 100ms duration
        });

        let (start_time, end_time) =
            process_time_range(&samples, sample_rate, time_range, None, total_duration).unwrap();

        let analysis_duration_ms = (end_time - start_time) * 1000.0;
        assert!((analysis_duration_ms - 100.0).abs() < 1.0); // Should be ~100ms

        // Auto-configure should use small window size for this short duration
        let config =
            SpectrogramConfig::auto_configure(sample_rate, 20.0, 20000.0, analysis_duration_ms)
                .unwrap();

        assert_eq!(config.window_size, 256); // Should use small window for 100ms
        assert_eq!(config.hop_size, 7); // 97% overlap for very short duration
    }
}
