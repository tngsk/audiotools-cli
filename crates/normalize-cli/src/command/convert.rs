use super::super::utils::detection::detect_peak_level;
use super::super::utils::get_walker;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::PathBuf;

// 定数の定義
const SUPPORTED_FORMATS: &[&str] = &["wav"];
const SUPPORTED_BIT_DEPTHS: &[u8] = &[16, 24];

const CHANNEL_CONVERSION_FACTOR: f32 = std::f32::consts::FRAC_1_SQRT_2; // -3dB

pub fn convert_files(
    input: &PathBuf,
    output_dir: Option<&PathBuf>,
    flatten: bool,
    input_format: &[String],
    output_format: &str,
    bit_depth: u8,
    prefix: Option<&str>,
    postfix: Option<&str>,
    recursive: bool,
    force: bool,
    channels: Option<u8>,
    normalize_level: Option<f32>,
) {
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
                let output_channels = channels.unwrap_or(input_channels as u8) as u16;
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

                // Optimization: Channel check hoisted outside the loop to prevent per-sample branch evaluation.
                // We pre-calculate combined scaling factors outside the inner loop to replace multiple
                // mathematical operations (including costly divisions) with a single multiplication and clamp.
                let inv_max_i16 = 1.0 / i16::MAX as f32;
                let combined_scale = inv_max_i16 * gain_multiplier * max_val;

                if input_channels == 1 && output_channels == 2 {
                    // Mono to Stereo
                    for sample in sample_iter {
                        let val = (sample as f32 * combined_scale).clamp(-max_val, max_val);
                        writer.write_sample(val as i32).unwrap();
                        writer.write_sample(val as i32).unwrap();
                    }
                } else if input_channels == 2 && output_channels == 1 {
                    // Stereo to Mono
                    let stereo_scale = CHANNEL_CONVERSION_FACTOR * combined_scale;
                    while let (Some(l), Some(r)) = (sample_iter.next(), sample_iter.next()) {
                        let val = ((l as f32 + r as f32) * stereo_scale).clamp(-max_val, max_val);
                        writer.write_sample(val as i32).unwrap();
                    }
                } else {
                    // Keep channels
                    for sample in sample_iter {
                        let val = (sample as f32 * combined_scale).clamp(-max_val, max_val);
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
