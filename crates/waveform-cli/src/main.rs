use clap::{Parser, ValueEnum};
use hound::WavReader;
use plotters::prelude::*;
use std::path::PathBuf;

mod utils;
use crate::utils::detection;
use crate::utils::detection::AutoStartDetection;
use crate::utils::get_walker;
use crate::utils::time;
use crate::utils::time::{TimeRange, TimeSpecification, parse_time_specification};

const FONT_FAMILY: &str = "Fira Code";
const BACKGROUND_COLOR: RGBColor = RGBColor(4, 20, 36);
const PEAK_COLOR: RGBColor = RGBColor(0, 128, 255);
const RMS_COLOR: RGBAColor = RGBAColor(0, 255, 128, 0.5);
const GRID_COLOR: RGBAColor = RGBAColor(255, 255, 255, 0.2);
const ANNOTATION_COLOR: RGBColor = RGBColor(255, 255, 0);

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(flatten)]
    args: WaveformArgs,
}

#[derive(clap::Args)]
struct WaveformArgs {
    /// Input audio file
    #[arg(short, long)]
    input: PathBuf,

    /// Output image file
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Process directories recursively
    #[arg(short, long)]
    recursive: bool,

    /// Display scale (amplitude or decibel)
    #[arg(long, value_enum, default_value = "amplitude")]
    scale: WaveformScale,

    /// Start time (seconds or MM:SS format)
    #[arg(long, value_parser = parse_time_specification)]
    start: Option<TimeSpecification>,

    /// End time (seconds or MM:SS format)
    #[arg(long, value_parser = parse_time_specification)]
    end: Option<TimeSpecification>,

    /// Enable automatic start detection
    #[arg(long)]
    auto_start: bool,

    /// Amplitude threshold for auto start detection
    #[arg(long, default_value = "0.01")]
    threshold: f32,

    /// Window size for auto start detection
    #[arg(long, default_value = "512")]
    detection_window: usize,

    /// Minimum duration for auto start detection (seconds)
    #[arg(long, default_value = "0.01")]
    min_duration: f32,

    /// Time annotations (format: "time:label", comma-separated)
    #[arg(long = "annotate", value_parser = parse_time_annotation, value_delimiter = ',')]
    annotations: Option<Vec<(f32, String)>>,

    /// Show RMS envelope
    #[arg(long)]
    show_rms: bool,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum WaveformScale {
    Amplitude,
    Decibel,
}

pub fn parse_time_annotation(s: &str) -> Result<(f32, String), String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err("Annotation format should be 'time:label'".to_string());
    }

    let time = parts[0]
        .parse::<f32>()
        .map_err(|_| "Invalid time value".to_string())?;

    Ok((time, parts[1].to_string()))
}

use audiotools_core::config::Config as AppConfig;

#[tokio::main]
async fn main() {
    let app_config = AppConfig::load_default().unwrap_or_default();
    let cli = Cli::parse();
    let args = cli.args;
    
    // Resolve params
    let recursive = args.recursive || app_config.global.as_ref().and_then(|g| g.recursive).unwrap_or(false);
    
    let wave_config = app_config.waveform.unwrap_or_default();
    let width = wave_config.width.unwrap_or(1200);
    let height = wave_config.height.unwrap_or(600);

    let time_range = time::create_time_range(args.start, args.end);
    let auto_start_config = detection::create_auto_start_config(
        args.auto_start,
        args.threshold,
        args.detection_window,
        args.min_duration,
    );

    for entry in get_walker(&args.input, recursive) {
        if let Some(ext) = entry.path().extension() {
            if ext.to_string_lossy().to_lowercase() == "wav" {
                let input_path = PathBuf::from(entry.path());
                let output_path = args
                    .output
                    .clone()
                    .unwrap_or_else(|| input_path.with_extension("png"));

                match create_waveform(
                    &input_path,
                    &output_path,
                    args.scale,
                    time_range.clone(),
                    auto_start_config.clone(),
                    args.annotations.clone(),
                    args.show_rms,
                    width,
                    height,
                ) {
                    Ok(_) => println!(
                        "Created waveform: {} -> {}",
                        input_path.display(),
                        output_path.display()
                    ),
                    Err(e) => eprintln!("Error processing {}: {}", input_path.display(), e),
                }
            }
        }
    }
}

pub fn create_waveform(
    input: &PathBuf,
    output: &PathBuf,
    scale: WaveformScale,
    time_range: Option<TimeRange>,
    auto_start: Option<AutoStartDetection>,
    annotations: Option<Vec<(f32, String)>>,
    show_rms: bool,
    width: u32,
    height: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = WavReader::open(input)?;
    let spec = reader.spec();
    let sample_rate = spec.sample_rate as f32;

    // サンプルデータの読み込み
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(|s| s.unwrap())
            .collect::<Vec<f32>>()
            .chunks(spec.channels as usize)
            .map(|chunk| chunk.iter().sum::<f32>() / chunk.len() as f32)
            .collect(),
        hound::SampleFormat::Int => {
            let bits = spec.bits_per_sample;
            let max_value = (1 << (bits - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|s| s.unwrap() as f32 / max_value)
                .collect::<Vec<f32>>()
                .chunks(spec.channels as usize)
                .map(|chunk| chunk.iter().sum::<f32>() / chunk.len() as f32)
                .collect()
        }
    };

    let total_duration = samples.len() as f32 / sample_rate;

    // 時間範囲の設定
    let (start_time, end_time) = if let Some(auto_config) = auto_start {
        let detected_start = auto_config
            .detect_start_time(&samples, sample_rate)
            .ok_or("Failed to detect start time")?;

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
        range.resolve(total_duration)?
    } else {
        (0.0, total_duration)
    };

    // サンプル範囲の切り出し
    let start_sample = (start_time * sample_rate) as usize;
    let end_sample = (end_time * sample_rate) as usize;
    let samples = samples[start_sample..end_sample].to_vec();

    // RMS値の計算
    let window_size = (sample_rate * 0.02) as usize; // 20ms window
    let rms_values = calculate_rms(&samples, window_size);

    // プロット作成
    let root = BitMapBackend::new(output.to_str().unwrap(), (width, height)).into_drawing_area();
    root.fill(&BACKGROUND_COLOR)?;

    let title = input
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Waveform");

    // Y軸の範囲設定
    let (y_min, y_max) = match scale {
        WaveformScale::Amplitude => (-1.0, 1.0),
        WaveformScale::Decibel => (-60.0, 0.0),
    };

    let mut chart = ChartBuilder::on(&root)
        .margin(40)
        .caption(title, (FONT_FAMILY, 24).into_font().color(&WHITE))
        .set_label_area_size(LabelAreaPosition::Left, 60)
        .set_label_area_size(LabelAreaPosition::Bottom, 40)
        .build_cartesian_2d(start_time..end_time, y_min..y_max)?;

    /// 時間の長さに応じて適切なグリッド間隔を決定する
    fn calculate_grid_interval(duration: f32) -> f32 {
        let intervals = [
            0.001, 0.002, 0.005, // ミリ秒単位
            0.01, 0.02, 0.05, // 10ミリ秒単位
            0.1, 0.2, 0.5, // 100ミリ秒単位
            1.0, 2.0, 5.0, // 秒単位
            10.0, 20.0, 30.0, // 10秒単位
            60.0, 120.0, 300.0, // 分単位
        ];

        // 目標とするグリッド数（画面の見やすさを考慮）
        const TARGET_GRID_COUNT: f32 = 10.0;

        // 最適な間隔を探す
        let ideal_interval = duration / TARGET_GRID_COUNT;

        // 理想的な間隔に最も近い定義済み間隔を選択
        intervals
            .iter()
            .min_by(|&&a, &&b| {
                let diff_a = (a - ideal_interval).abs();
                let diff_b = (b - ideal_interval).abs();
                diff_a.partial_cmp(&diff_b).unwrap()
            })
            .copied()
            .unwrap_or(1.0)
    }

    // グリッドとラベルの設定
    let duration = end_time - start_time;
    let grid_interval = calculate_grid_interval(duration);
    let grid_count = (duration / grid_interval).ceil() as usize;

    chart
        .configure_mesh()
        .label_style((FONT_FAMILY, 14).into_font().color(&WHITE))
        .axis_style(&WHITE)
        // .light_line_style(RGBAColor(255, 255, 255, 0.1))
        .bold_line_style(GRID_COLOR)
        .x_desc("Time (s)")
        .y_desc(match scale {
            WaveformScale::Amplitude => "Amplitude",
            WaveformScale::Decibel => "Level (dB)",
        })
        .x_labels(grid_count)
        .y_labels(match scale {
            WaveformScale::Amplitude => 5, // -1.0から1.0まで0.5間隔（5区間）
            WaveformScale::Decibel => 10,  // -60dBから0dBまで6dB間隔
        })
        .x_label_formatter(&|x| {
            // グリッド間隔に応じてフォーマットを調整
            if grid_interval >= 1.0 {
                format!("{:.0}s", x)
            } else if grid_interval >= 0.1 {
                format!("{:.1}s", x)
            } else if grid_interval >= 0.01 {
                format!("{:.2}s", x)
            } else {
                format!("{:.3}s", x)
            }
        })
        .draw()?;

    // 波形の描画
    let time_points: Vec<f32> = (0..samples.len())
        .map(|i| start_time + i as f32 / sample_rate)
        .collect();

    // RMS波形の描画を条件付きに
    if show_rms {
        let rms_points: Vec<(f32, f32)> = time_points
            .iter()
            .zip(rms_values.iter())
            .map(|(&t, &rms)| match scale {
                WaveformScale::Amplitude => (t, rms),
                WaveformScale::Decibel => (t, amplitude_to_db(rms)),
            })
            .collect();

        chart.draw_series(AreaSeries::new(
            rms_points.iter().map(|&(x, y)| (x, y)),
            0.0,
            &RMS_COLOR,
        ))?;
    }

    // ピーク波形の描画
    let peak_points: Vec<(f32, f32)> = time_points
        .iter()
        .zip(samples.iter())
        .map(|(&t, &sample)| match scale {
            WaveformScale::Amplitude => (t, sample),
            WaveformScale::Decibel => (t, amplitude_to_db(sample)),
        })
        .collect();

    chart.draw_series(LineSeries::new(
        peak_points.iter().map(|&(x, y)| (x, y)),
        &PEAK_COLOR,
    ))?;

    // アノテーションの描画
    if let Some(annotations) = annotations {
        for (time, label) in annotations {
            if time >= start_time && time <= end_time {
                // 垂直線の描画
                chart.draw_series(LineSeries::new(
                    vec![(time, y_min), (time, y_max)],
                    &ANNOTATION_COLOR,
                ))?;

                // ラベルの描画
                let label_y = y_max - (y_max - y_min) * 0.1;
                chart.draw_series(std::iter::once(Text::new(
                    label,
                    (time, label_y),
                    (FONT_FAMILY, 16).into_font().color(&ANNOTATION_COLOR),
                )))?;
            }
        }
    }

    Ok(())
}

fn calculate_rms(samples: &[f32], window_size: usize) -> Vec<f32> {
    let mut rms_values = Vec::with_capacity(samples.len());
    for i in 0..samples.len() {
        let start = if i < window_size / 2 {
            0
        } else {
            i - window_size / 2
        };
        let end = (i + window_size / 2).min(samples.len());

        let sum_squares: f32 = samples[start..end].iter().map(|&x| x * x).sum();
        let rms = (sum_squares / (end - start) as f32).sqrt();
        rms_values.push(rms);
    }
    rms_values
}

fn amplitude_to_db(amplitude: f32) -> f32 {
    if amplitude.abs() < 1e-6 {
        -60.0
    } else {
        (20.0 * amplitude.abs().log10()).max(-60.0)
    }
}
