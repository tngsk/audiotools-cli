use clap::{Parser, ValueEnum};
use std::path::PathBuf;

mod command;
mod core;
mod domain;
mod error;
mod render;
mod utils;

use crate::command::SpectrumCommand;
use crate::core::analysis::DefaultSpectralAnalyzer;
use crate::core::audio::DefaultAudioLoader;
use crate::core::config::builder::ConfigBuilder;
use crate::core::config::{FrequencyPreset, SpectrogramConfig};
use crate::domain::frequency::{FrequencyAnnotation, parse_frequency_annotation};
use crate::domain::request::{SpectrumOptions, SpectrumRequest};
use crate::render::DefaultSpectrogramRenderer;
use crate::utils::detection;
use crate::utils::time::{self, TimeSpecification};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(flatten)]
    args: SpectrumArgs,
}

#[derive(clap::Args)]
struct SpectrumArgs {
    /// Input audio file
    #[arg(short, long)]
    input: PathBuf,

    /// Output image file
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// FFT window size (0 for auto-configure based on audio duration)
    #[arg(long, default_value = "2048")]
    window_size: usize,

    /// Window overlap ratio (0.0-1.0)
    #[arg(long, default_value = "0.75")]
    overlap: f32,

    /// Minimum frequency to display (Hz)
    #[arg(long, default_value = "20.0")]
    min_freq: f32,

    /// Maximum frequency to display (Hz)
    #[arg(long, default_value = "20000.0")]
    max_freq: f32,

    /// Frequency range preset (overrides min/max-freq)
    #[arg(long, value_enum)]
    freq_preset: Option<FrequencyPresetArg>,

    /// Enable adaptive mode for short audio optimization
    #[arg(long)]
    adaptive: bool,

    /// Process directories recursively (Note: Not fully implemented for spectrum-cli yet)
    #[arg(short, long)]
    recursive: bool,

    // Start time (seconds or MM:SS format)
    #[arg(long, value_parser = time::parse_time_specification)]
    start: Option<TimeSpecification>,

    /// End time (seconds or MM:SS format)
    #[arg(long, value_parser = time::parse_time_specification)]
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

    /// Frequency annotations (format: "freq:label", comma-separated)
    #[arg(long = "annotate", value_parser = parse_frequency_annotation, value_delimiter = ',')]
    annotations: Option<Vec<(f32, String)>>,
}

use audiotools_core::config::Config as AppConfig;

#[tokio::main]
async fn main() {
    let app_config = AppConfig::load_default().unwrap_or_default();
    let cli = Cli::parse();
    let args = cli.args;

    let time_range = time::create_time_range(args.start, args.end);
    let auto_start_config = detection::create_auto_start_config(
        args.auto_start,
        args.threshold,
        args.detection_window,
        args.min_duration,
    );

    // Resolve Spectrogram Config
    let spec_config = app_config.spectrogram.unwrap_or_default();
    
    let width = spec_config.width.unwrap_or(1200);
    let height = spec_config.height.unwrap_or(600);
    
    let window_size = if args.window_size != 2048 {
        args.window_size
    } else {
        spec_config.n_fft.unwrap_or(2048)
    };
    
    let min_freq = if (args.min_freq - 20.0).abs() > f32::EPSILON {
        args.min_freq
    } else {
        0.0 // preset or default?
    };
    
    let max_freq = if (args.max_freq - 20000.0).abs() > f32::EPSILON {
         args.max_freq
    } else {
         spec_config.fmax.unwrap_or(20000.0)
    };
    
    // Build SpectrogramConfig using ConfigBuilder
    let mut config_builder = ConfigBuilder::new().image_dimensions(width, height);

    if let Some(preset) = args.freq_preset {
        // Use a default sample rate for preset calculation, actual will be from audio file
        let default_sample_rate = 44100.0;
        let (p_min, p_max) =
            SpectrogramConfig::frequency_preset(preset.into(), default_sample_rate);
        config_builder = config_builder.frequency_range(p_min, p_max);
    } else {
        config_builder = config_builder.frequency_range(min_freq.max(args.min_freq), max_freq);
    }

    if window_size != 0 && !args.adaptive {
        config_builder = config_builder.window_size(window_size);
    } else if args.adaptive {
        // When adaptive, window_size will be auto-configured based on duration
        // The actual duration will be passed to auto_configure inside SpectrumCommand.execute
        config_builder = config_builder.auto_configure(0.0); // Placeholder duration
    }

    let config = config_builder
        .build()
        .expect("Failed to build spectrogram config");

    let output_path = args
        .output
        .unwrap_or_else(|| args.input.with_extension("png")); // Default output path

    let request = SpectrumRequest {
        input_path: args.input,
        output_path,
        config: config.clone(), // Clone config for the request
        time_range,
        auto_start: auto_start_config,
        annotations: args
            .annotations
            .map(|anns| {
                anns.into_iter()
                    .map(|(freq, label)| FrequencyAnnotation {
                        frequency: freq,
                        label,
                    })
                    .collect()
            })
            .unwrap_or_default(),
        options: SpectrumOptions, // Use direct import
    };

    let audio_loader = Box::new(DefaultAudioLoader);
    let spectral_analyzer = Box::new(DefaultSpectralAnalyzer::new(config)); // Pass config to analyzer
    let spectrogram_renderer = Box::new(DefaultSpectrogramRenderer);

    let spectrum_command =
        SpectrumCommand::new(audio_loader, spectral_analyzer, spectrogram_renderer);

    // Handle recursive processing
    let recursive = args.recursive || app_config.global.as_ref().and_then(|g| g.recursive).unwrap_or(false);
    if recursive {
        // TODO: Implement batch processing for directories
        eprintln!(
            "Warning: Recursive processing for spectrum command is not yet fully implemented with the new architecture. Processing single file."
        );
        if let Err(e) = spectrum_command.execute(request).await {
            eprintln!("Error creating spectrogram: {}", e);
        }
    } else {
        if let Err(e) = spectrum_command.execute(request).await {
            eprintln!("Error creating spectrogram: {}", e);
        }
    }
}

#[derive(ValueEnum, Clone, Debug)]
enum FrequencyPresetArg {
    Full,
    AudioRange,
    SpeechRange,
    MusicRange,
    Bass,
}

impl From<FrequencyPresetArg> for FrequencyPreset {
    fn from(arg: FrequencyPresetArg) -> Self {
        match arg {
            FrequencyPresetArg::Full => crate::core::config::FrequencyPreset::Full,
            FrequencyPresetArg::AudioRange => crate::core::config::FrequencyPreset::AudioRange,
            FrequencyPresetArg::SpeechRange => crate::core::config::FrequencyPreset::SpeechRange,
            FrequencyPresetArg::MusicRange => crate::core::config::FrequencyPreset::MusicRange,
            FrequencyPresetArg::Bass => crate::core::config::FrequencyPreset::Bass,
        }
    }
}
