use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use audiotools::command::{
    convert, info, loudness, normalize,
    spectrum::{FrequencyPreset},
    waveform::{self, parse_time_annotation, WaveformScale},
};

use audiotools::utils::detection;
use audiotools::utils::time::{self, TimeSpecification};

// New imports for spectrum command
use audiotools::command::spectrum::command::SpectrumCommand;
use audiotools::command::spectrum::domain::request::{SpectrumRequest, SpectrumOptions};
use audiotools::command::spectrum::command::SpectrumResponse;
use audiotools::command::spectrum::core::audio::DefaultAudioLoader;
use audiotools::command::spectrum::render::DefaultSpectrogramRenderer;
use audiotools::command::spectrum::core::config::{SpectrogramConfig};
use audiotools::command::spectrum::core::config::builder::ConfigBuilder;
use audiotools::command::spectrum::domain::frequency::parse_frequency_annotation; // Import directly from new location
use audiotools::command::spectrum::core::analysis::DefaultSpectralAnalyzer;

// Define CLI application structure using clap
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

// Define available subcommands and their arguments
#[derive(Subcommand)]
enum Commands {
    /// Convert audio files between formats
    Convert {
        /// Input directory or file path
        #[arg(short, long)]
        input: PathBuf,

        /// Output directory path
        #[arg(short, long)]
        output_dir: Option<PathBuf>,

        /// Flatten output directory structure (ignore source directory hierarchy)
        #[arg(short = 'f', long)]
        flatten: bool,

        /// Input formats to process (e.g., wav,flac,mp3)
        #[arg(short = 'I', long, value_delimiter = ',', default_value = "wav")]
        input_format: Vec<String>,

        /// Target output format
        #[arg(short = 'O', long, default_value = "wav")]
        output_format: String,

        /// Output bit depth for WAV files
        #[arg(short, long, default_value = "16")]
        bit_depth: u8,

        /// Target sample rate for conversion
        #[arg(short, long)]
        sample_rate: Option<u32>,

        /// Prefix to add to output filenames
        #[arg(long)]
        prefix: Option<String>,

        /// Postfix to add to output filenames
        #[arg(long)]
        postfix: Option<String>,

        /// Process directories recursively
        #[arg(short, long)]
        recursive: bool,

        /// Force overwrite of existing files
        #[arg(long)]
        force: bool,

        /// Number of output channels (1=mono, 2=stereo)
        #[arg(long, value_name = "CHANNELS")]
        channels: Option<u8>,

        /// Target peak level in dBFS (e.g., -1.0)
        #[arg(short = 'l', long = "level", allow_negative_numbers = true)]
        normalize_level: Option<f32>,
    },

    /// Display audio file information
    Info {
        /// Input directory or file path
        #[arg(short, long)]
        input: PathBuf,

        /// Output file for information
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Fields to display in output
        #[arg(short, long, value_delimiter = ',')]
        fields: Vec<String>,

        /// Process directories recursively
        #[arg(short, long)]
        recursive: bool,
    },

    /// Measure audio loudness using EBU R128
    Loudness {
        /// Input directory or file path
        #[arg(short, long)]
        input: PathBuf,

        /// Output file for measurements
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Process directories recursively
        #[arg(short, long)]
        recursive: bool,
    },

    /// Normalize audio files to target peak level
    Normalize {
        /// Input directory or file path
        #[arg(short, long)]
        input: PathBuf,

        /// Output directory path
        #[arg(short, long)]
        output_dir: Option<PathBuf>,

        /// Target peak level in dBFS (e.g., -1.0)
        #[arg(short, long, default_value_t = -1.0, allow_negative_numbers = true)]
        level: f32,

        /// Input formats to process (e.g., wav,flac,mp3)
        #[arg(short = 'I', long, value_delimiter = ',', default_value = "wav")]
        input_format: Vec<String>,

        /// Process directories recursively
        #[arg(short, long)]
        recursive: bool,

        /// Force overwrite of existing files
        #[arg(long)]
        force: bool,
    },
    /// Create spectrogram from audio file
    Spectrum {
        /// Input audio file
        #[arg(short, long)]
        input: PathBuf,

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

        /// Process directories recursively
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
    },
    Waveform {
        /// Input audio file
        #[arg(short, long)]
        input: PathBuf,

        /// Process directories recursively
        #[arg(short, long)]
        recursive: bool,

        /// Display scale (amplitude or decibel)
        #[arg(long, value_enum, default_value = "amplitude")]
        scale: WaveformScale,

        /// Start time (seconds or MM:SS format)
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

        /// Time annotations (format: "time:label", comma-separated)
        #[arg(long = "annotate", value_parser = parse_time_annotation, value_delimiter = ',')]
        annotations: Option<Vec<(f32, String)>>,

        /// Show RMS envelope
        #[arg(long)]
        show_rms: bool,
    },
}

// Main function: Parse CLI arguments and dispatch to appropriate handler
#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Convert {
            input,
            output_dir,
            flatten,
            input_format,
            output_format,
            bit_depth,
            sample_rate,
            prefix,
            postfix,
            recursive,
            force,
            channels,
            normalize_level,
        } => {
            convert::convert_files(
                &input,
                output_dir.as_ref(),
                flatten,
                &input_format,
                &output_format,
                bit_depth,
                sample_rate,
                prefix.as_deref(),
                postfix.as_deref(),
                recursive,
                force,
                channels,
                normalize_level,
            );
        }
        Commands::Info {
            input,
            output,
            fields,
            recursive,
        } => {
            info::get_audio_info(&input, output.as_ref(), &fields, recursive);
        }
        Commands::Loudness {
            input,
            output,
            recursive,
        } => {
            loudness::measure_loudness(&input, output.as_ref(), recursive);
        }
        Commands::Normalize {
            input,
            output_dir,
            level,
            input_format,
            recursive,
            force,
        } => {
            let _ = normalize::normalize_files(
                &input,
                output_dir.as_ref(),
                level,
                &input_format,
                recursive,
                force,
            );
        }
        Commands::Spectrum {
            input,
            window_size,
            overlap: _overlap, // Mark as ignored
            min_freq,
            max_freq,
            freq_preset,
            adaptive,
            recursive,
            start,
            end,
            auto_start,
            threshold,
            detection_window,
            min_duration,
            annotations,
        } => {
            let time_range = time::create_time_range(start, end);
            let auto_start_config = detection::create_auto_start_config(
                auto_start,
                threshold,
                detection_window,
                min_duration,
            );

            // Build SpectrogramConfig using ConfigBuilder
            let mut config_builder = ConfigBuilder::new()
                .image_dimensions(1200, 600);

            if let Some(preset) = freq_preset {
                // Use a default sample rate for preset calculation, actual will be from audio file
                let default_sample_rate = 44100.0;
                let (p_min, p_max) = SpectrogramConfig::frequency_preset(preset.into(), default_sample_rate);
                config_builder = config_builder.frequency_range(p_min, p_max);
            } else {
                config_builder = config_builder.frequency_range(min_freq, max_freq);
            }

            if window_size != 0 && !adaptive {
                config_builder = config_builder.window_size(window_size);
            } else if adaptive {
                // When adaptive, window_size will be auto-configured based on duration
                // The actual duration will be passed to auto_configure inside SpectrumCommand.execute
                config_builder = config_builder.auto_configure(0.0); // Placeholder duration
            }

            let config = config_builder.build().expect("Failed to build spectrogram config");

            let output_path = input.with_extension("png"); // Default output path

            let request = SpectrumRequest {
                input_path: input,
                output_path,
                config: config.clone(), // Clone config for the request
                time_range,
                auto_start: auto_start_config,
                annotations,
                options: SpectrumOptions, // Use direct import
            };

            let audio_loader = Box::new(DefaultAudioLoader);
            let spectral_analyzer = Box::new(DefaultSpectralAnalyzer::new(config)); // Pass config to analyzer
            let spectrogram_renderer = Box::new(DefaultSpectrogramRenderer);

            let spectrum_command = SpectrumCommand::new(
                audio_loader,
                spectral_analyzer,
                spectrogram_renderer,
            );

            // Handle recursive processing
            if recursive {
                // TODO: Implement batch processing for directories
                eprintln!("Warning: Recursive processing for spectrum command is not yet fully implemented with the new architecture. Processing single file.");
                if let Err(e) = spectrum_command.execute(request).await {
                    eprintln!("Error creating spectrogram: {}", e);
                }
            } else {
                if let Err(e) = spectrum_command.execute(request).await {
                    eprintln!("Error creating spectrogram: {}", e);
                }
            }
        }
        Commands::Waveform {
            input,
            recursive,
            scale,
            start,
            end,
            auto_start,
            threshold,
            detection_window,
            min_duration,
            annotations,
            show_rms,
        } => {
            let time_range = time::create_time_range(start, end);
            let auto_start_config = detection::create_auto_start_config(
                auto_start,
                threshold,
                detection_window,
                min_duration,
            );
            waveform::create_waveforms(
                &input,
                scale,
                time_range,
                auto_start_config,
                recursive,
                annotations,
                show_rms,
            );
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
            FrequencyPresetArg::Full => audiotools::command::spectrum::core::config::FrequencyPreset::Full,
            FrequencyPresetArg::AudioRange => audiotools::command::spectrum::core::config::FrequencyPreset::AudioRange,
            FrequencyPresetArg::SpeechRange => audiotools::command::spectrum::core::config::FrequencyPreset::SpeechRange,
            FrequencyPresetArg::MusicRange => audiotools::command::spectrum::core::config::FrequencyPreset::MusicRange,
            FrequencyPresetArg::Bass => audiotools::command::spectrum::core::config::FrequencyPreset::Bass,
        }
    }
}