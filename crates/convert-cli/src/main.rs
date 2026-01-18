use clap::Parser;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

mod utils;
use crate::utils::detection::detect_peak_level;
use crate::utils::get_walker;

// 定数の定義
const SUPPORTED_FORMATS: &[&str] = &["wav", "flac", "mp3"];
const SUPPORTED_BIT_DEPTHS: &[u8] = &[16, 24];
const DEFAULT_MP3_BITRATE: &str = "320k";
const DEFAULT_FLAC_COMPRESSION: &str = "8";
const CHANNEL_CONVERSION_FACTOR: f32 = 0.7071; // -3dB

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
    let output_format = args.output_format
        .or(config.convert.as_ref().and_then(|c| c.format.clone()))
        .unwrap_or_else(|| "wav".to_string());
        
    let recursive = args.recursive || config.global.as_ref().and_then(|g| g.recursive).unwrap_or(false);
    let force = args.force || config.global.as_ref().and_then(|g| g.overwrite).unwrap_or(false);

    let normalize_level = args.normalize_level.or(config.normalize.as_ref().and_then(|n| n.level));
    
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

    // Determine codec and extension based on output format
    let (codec, out_ext) = match output_format.to_lowercase().as_str() {
        "wav" => {
            if !SUPPORTED_BIT_DEPTHS.contains(&bit_depth) {
                panic!(
                    "Unsupported bit depth for WAV. Supported depths are: {:?}",
                    SUPPORTED_BIT_DEPTHS
                );
            }
            (
                match bit_depth {
                    16 => "pcm_s16le",
                    24 => "pcm_s24le",
                    _ => unreachable!(),
                },
                "wav",
            )
        }
        "flac" => ("flac", "flac"),
        "mp3" => ("libmp3lame", "mp3"),
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

                let mut cmd = Command::new("ffmpeg");
                cmd.arg("-i").arg(entry.path());

                if force {
                    cmd.arg("-y");
                } else {
                    cmd.arg("-n");
                }

                // ノーマライズ処理の改善
                if let Some(target_level) = normalize_level {
                    match detect_peak_level(&entry.path().to_path_buf()) {
                        Ok(current_peak) => {
                            let gain = target_level - current_peak;
                            println!(
                                "Current peak: {:.1} dBFS, Target: {:.1} dBFS, Applying gain: {:.1} dB",
                                current_peak, target_level, gain
                            );
                            cmd.args(&["-af", &format!("volume={}dB", gain)]);
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

                // モノラルステレオ変換
                if let Some(ch) = args.channels {
                    match ch {
                        1 => {
                            cmd.args(&[
                                "-af",
                                &format!(
                                    "pan=mono|c0={}*c0+{}*c1",
                                    CHANNEL_CONVERSION_FACTOR, CHANNEL_CONVERSION_FACTOR
                                ),
                            ]);
                        }
                        2 => {
                            cmd.args(&[
                                "-af",
                                &format!(
                                    "pan=stereo|c0={}*c0|c1={}*c0",
                                    CHANNEL_CONVERSION_FACTOR, CHANNEL_CONVERSION_FACTOR
                                ),
                            ]);
                        }
                        _ => {
                            panic!("Unsupported number of channels. Use 1 for mono or 2 for stereo")
                        }
                    }
                }

                // サンプリングレート
                if let Some(rate) = args.sample_rate {
                    cmd.arg("-ar").arg(rate.to_string());
                }

                // ファイル形式とコーデック
                match output_format.as_str() {
                    "mp3" => {
                        cmd.args(&["-b:a", DEFAULT_MP3_BITRATE]);
                    }
                    "flac" => {
                        cmd.args(&["-compression_level", DEFAULT_FLAC_COMPRESSION]);
                    }
                    _ => {}
                }
                cmd.args(&["-acodec", codec]).arg(&output);

                // 変換実行
                cmd.output().expect("Failed to execute ffmpeg");
                println!(
                    "Converted: {} -> {}",
                    entry.path().display(),
                    output.display()
                );
            }
        }
    }
}
