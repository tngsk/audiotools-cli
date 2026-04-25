use clap::Parser;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::PathBuf;

mod utils;
use utils::detection::detect_peak_level;
use utils::get_walker;

// 定数の定義
const SUPPORTED_FORMATS: &[&str] = &["wav"];
const SUPPORTED_BIT_DEPTHS: &[u8] = &[16, 24];

const CHANNEL_CONVERSION_FACTOR: f32 = std::f32::consts::FRAC_1_SQRT_2; // -3dB, fixed clippy warning while at it

use audiotools_core::config::Config;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(flatten)]
    args: ConvertArgs,
}

#[derive(clap::Args)]
struct ConvertArgs {
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
    #[arg(short = 'I', long, value_delimiter = ',')]
    input_format: Option<Vec<String>>,

    /// Target output format
    #[arg(short = 'O', long)]
    output_format: Option<String>,

    /// Output bit depth for WAV files
    #[arg(short, long, default_value = "16")]
    bit_depth: u8,

    /// Prefix to add to output filenames
    #[arg(long)]
    prefix: Option<String>,

    /// Postfix to add to output filenames
    #[arg(long)]
    postfix: Option<String>,

    /// Process directories recursively
    #[arg(short, long, default_value_t = false)]
    recursive: bool,

    /// Force overwrite of existing files
    #[arg(long, default_value_t = false)]
    force: bool,

    /// Number of output channels (1=mono, 2=stereo)
    #[arg(long, value_name = "CHANNELS")]
    channels: Option<u8>,

    /// Target peak level in dBFS (e.g., -1.0)
    #[arg(short = 'l', long = "level", allow_negative_numbers = true)]
    normalize_level: Option<f32>,
}

#[tokio::main]
async fn main() {
    let config = Config::load_default().unwrap_or_default();
    let cli = Cli::parse();
    let args = cli.args;

    // Resolve defaults
    let output_format = args
        .output_format
        .or(config.convert.as_ref().and_then(|c| c.format.clone()))
        .unwrap_or_else(|| "wav".to_string());

    let recursive = args.recursive
        || config
            .global
            .as_ref()
            .and_then(|g| g.recursive)
            .unwrap_or(false);
    let force = args.force
        || config
            .global
            .as_ref()
            .and_then(|g| g.overwrite)
            .unwrap_or(false);

    let normalize_level = args
        .normalize_level
        .or(config.normalize.as_ref().and_then(|n| n.level));

    // Subtype to bit_depth mapping
    let bit_depth = if args.bit_depth != 16 {
        args.bit_depth
    } else {
        // Try config
        if let Some(subtype) = config.convert.as_ref().and_then(|c| c.subtype.clone()) {
            match subtype.as_str() {
                "PCM_16" => 16,
                "PCM_24" => 24,
                _ => 16, // Default or warn?
            }
        } else {
            16
        }
    };

    // Input format default
    let input_format_list = args.input_format.unwrap_or_else(|| vec!["wav".to_string()]);

    // Determine extension based on output format
    let out_ext = match output_format.to_lowercase().as_str() {
        "wav" => {
            if !SUPPORTED_BIT_DEPTHS.contains(&bit_depth) {
                panic!(
                    "Unsupported bit depth for WAV. Supported depths are: {:?}",
                    SUPPORTED_BIT_DEPTHS
                );
            }
            "wav"
        }
        format => panic!(
            "Unsupported output format: {}. Supported formats are: {:?}",
            format, SUPPORTED_FORMATS
        ),
    };

    // Convert input formats to lowercase for comparison
    let input_extensions: Vec<String> =
        input_format_list.iter().map(|f| f.to_lowercase()).collect();

    for entry in get_walker(&args.input, recursive) {
        if let Some(ext) = entry.path().extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            if input_extensions.contains(&ext_str) {
                let stem = entry.path().file_stem().unwrap().to_string_lossy();
                let filename = format!(
                    "{}{}{}.{}",
                    args.prefix.clone().unwrap_or(String::new()),
                    stem,
                    args.postfix.clone().unwrap_or(String::new()),
                    out_ext
                );

                let output = if let Some(out_dir) = &args.output_dir {
                    if args.flatten {
                        out_dir.join(&filename)
                    } else {
                        let relative_path = entry
                            .path()
                            .strip_prefix(&args.input)
                            .unwrap_or(entry.path())
                            .parent()
                            .unwrap_or_else(|| std::path::Path::new(""));
                        let full_output_dir = out_dir.join(relative_path);
                        fs::create_dir_all(&full_output_dir)
                            .expect("Failed to create output directory");
                        full_output_dir.join(&filename)
                    }
                } else {
                    entry.path().with_file_name(filename)
                };

                if output.exists() && !force {
                    println!(
                        "Skipped: {} (output file already exists. Use --force to overwrite)",
                        output.display()
                    );
                    continue;
                }

                let file = File::open(entry.path()).expect("Failed to open input file");
                let decoder = match rodio::Decoder::new(BufReader::new(file)) {
                    Ok(d) => d,
                    Err(e) => {
                        println!("Failed to decode {}: {}", entry.path().display(), e);
                        continue;
                    }
                };

                let input_channels = rodio::Source::channels(&decoder);
                let input_sample_rate = rodio::Source::sample_rate(&decoder);
                let output_channels = args.channels.unwrap_or(input_channels as u8) as u16;
                let mut gain_multiplier = 1.0f32;

                // ノーマライズ処理の改善
                if let Some(target_level) = normalize_level {
                    match detect_peak_level(&entry.path().to_path_buf()) {
                        Ok(current_peak) => {
                            let gain = target_level - current_peak;
                            gain_multiplier = 10.0f32.powf(gain / 20.0);
                            println!(
                                "Current peak: {:.1} dBFS, Target: {:.1} dBFS, Applying gain: {:.1} dB",
                                current_peak, target_level, gain
                            );
                        }
                        Err(e) => {
                            println!(
                                "Warning: Could not detect peak level for {}: {}. Skipping normalization.",
                                entry.path().display(),
                                e
                            );
                        }
                    }
                }

                let spec = hound::WavSpec {
                    channels: output_channels,
                    sample_rate: input_sample_rate,
                    bits_per_sample: bit_depth as u16,
                    sample_format: hound::SampleFormat::Int,
                };

                let mut writer =
                    hound::WavWriter::create(&output, spec).expect("Failed to create output file");

                let max_val = if bit_depth == 16 {
                    i16::MAX as f32
                } else if bit_depth == 24 {
                    8388607.0f32
                } else {
                    i32::MAX as f32
                };

                let mut sample_iter = decoder.into_iter();

                // Optimization: Channel check hoisted outside the loop to prevent per-sample branch evaluation
                if input_channels == 1 && output_channels == 2 {
                    // Mono to Stereo
                    for sample in sample_iter {
                        let val = (sample as f32 / i16::MAX as f32) * gain_multiplier;
                        let val = val.clamp(-1.0, 1.0) * max_val;
                        writer.write_sample(val as i32).unwrap();
                        writer.write_sample(val as i32).unwrap();
                    }
                } else if input_channels == 2 && output_channels == 1 {
                    // Stereo to Mono
                    while let (Some(l), Some(r)) = (sample_iter.next(), sample_iter.next()) {
                        let l_val = l as f32 / i16::MAX as f32;
                        let r_val = r as f32 / i16::MAX as f32;
                        let val = (l_val * CHANNEL_CONVERSION_FACTOR
                            + r_val * CHANNEL_CONVERSION_FACTOR)
                            * gain_multiplier;
                        let val = val.clamp(-1.0, 1.0) * max_val;
                        writer.write_sample(val as i32).unwrap();
                    }
                } else {
                    // Keep channels
                    for sample in sample_iter {
                        let val = (sample as f32 / i16::MAX as f32) * gain_multiplier;
                        let val = val.clamp(-1.0, 1.0) * max_val;
                        writer.write_sample(val as i32).unwrap();
                    }
                }

                writer.finalize().expect("Failed to finalize output file");

                println!(
                    "Converted: {} -> {}",
                    entry.path().display(),
                    output.display()
                );
            }
        }
    }
}
