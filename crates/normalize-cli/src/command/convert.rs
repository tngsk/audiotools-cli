use super::super::utils::detection::detect_peak_level;
use super::super::utils::get_walker;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

// 定数の定義
const SUPPORTED_FORMATS: &[&str] = &["wav", "flac", "mp3"];
const SUPPORTED_BIT_DEPTHS: &[u8] = &[16, 24];
const DEFAULT_MP3_BITRATE: &str = "320k";
const DEFAULT_FLAC_COMPRESSION: &str = "8";
const CHANNEL_CONVERSION_FACTOR: f32 = 0.7071; // -3dB

pub fn convert_files(
    input: &PathBuf,
    output_dir: Option<&PathBuf>,
    flatten: bool,
    input_format: &[String],
    output_format: &str,
    bit_depth: u8,
    sample_rate: Option<u32>,
    prefix: Option<&str>,
    postfix: Option<&str>,
    recursive: bool,
    force: bool,
    channels: Option<u8>,
    normalize_level: Option<f32>,
) {
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
    let input_extensions: Vec<String> = input_format.iter().map(|f| f.to_lowercase()).collect();

    for entry in get_walker(input, recursive) {
        if let Some(ext) = entry.path().extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            if input_extensions.contains(&ext_str) {
                let stem = entry.path().file_stem().unwrap().to_string_lossy();
                let filename = format!(
                    "{}{}{}.{}",
                    prefix.unwrap_or(""),
                    stem,
                    postfix.unwrap_or(""),
                    out_ext
                );

                let output = if let Some(out_dir) = output_dir {
                    if flatten {
                        out_dir.join(&filename)
                    } else {
                        let relative_path = entry
                            .path()
                            .strip_prefix(input)
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
                if let Some(ch) = channels {
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
                if let Some(rate) = sample_rate {
                    cmd.arg("-ar").arg(rate.to_string());
                }

                // ファイル形式とコーデック
                match output_format {
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
