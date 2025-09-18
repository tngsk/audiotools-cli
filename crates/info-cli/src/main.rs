use clap::Parser;
use serde_json::Value;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

mod utils;
use crate::utils::ffprobe::run_ffprobe;
use crate::utils::wave_header::WavHeader;
use crate::utils::{format_size, get_walker, is_audio_file};

struct AudioFileInfo {
    file: String,
    format: String,
    file_size: String,
    sample_rate: Option<u32>,
    bits_per_sample: Option<u16>,
    channels: Option<u16>,
    duration: Option<f64>,
    total_samples: Option<u64>,
    time_precision: Option<f64>,
    sample_accurate_duration: Option<f64>,
}

impl AudioFileInfo {
    fn new(file: String, format: String, file_size: String) -> Self {
        AudioFileInfo {
            file,
            format,
            file_size,
            sample_rate: None,
            bits_per_sample: None,
            channels: None,
            duration: None,
            total_samples: None,
            time_precision: None,
            sample_accurate_duration: None,
        }
    }

    fn format_output(&self, format_string: &str) -> String {
        format_string
            .replace("{file}", &self.file)
            .replace("{format}", &self.format)
            .replace("{file_size}", &self.file_size)
            .replace("{file_size_kb}", &self.file_size_kb())
            .replace(
                "{sample_rate}",
                &self
                    .sample_rate
                    .map_or("N/A".to_string(), |s| s.to_string()),
            )
            .replace(
                "{bits_per_sample}",
                &self
                    .bits_per_sample
                    .map_or("N/A".to_string(), |b| b.to_string()),
            )
            .replace(
                "{channels}",
                &self.channels.map_or("N/A".to_string(), |c| c.to_string()),
            )
            .replace(
                "{duration}",
                &self
                    .duration
                    .map_or("N/A".to_string(), |d| format!("{:.3}", d)),
            )
            .replace(
                "{total_samples}",
                &self
                    .total_samples
                    .map_or("N/A".to_string(), |t| t.to_string()),
            )
            .replace(
                "{time_precision}",
                &self
                    .time_precision
                    .map_or("N/A".to_string(), |tp| format!("{:.8}", tp)),
            )
            .replace(
                "{sample_accurate_duration}",
                &self
                    .sample_accurate_duration
                    .map_or("N/A".to_string(), |sad| format!("{:.8}", sad)),
            )
    }

    fn file_size_kb(&self) -> String {
        // Assuming file_size is like "1.23 MB" or "123 Bytes"
        // This is a simplified conversion and might not be perfectly accurate for all units.
        if let Some(size_str) = self.file_size.split_whitespace().next() {
            if let Ok(size_val) = size_str.parse::<f64>() {
                if self.file_size.contains("KB") {
                    return format!("{:.2}", size_val);
                } else if self.file_size.contains("MB") {
                    return format!("{:.2}", size_val * 1024.0);
                } else if self.file_size.contains("GB") {
                    return format!("{:.2}", size_val * 1024.0 * 1024.0);
                } else if self.file_size.contains("Bytes") {
                    return format!("{:.2}", size_val / 1024.0);
                }
            }
        }
        "N/A".to_string()
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(flatten)]
    args: InfoArgs,
}

#[derive(clap::Args)]
struct InfoArgs {
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

    /// Output format string (e.g., "{file},{sample_rate},{bits_per_sample},{channels},{duration},{file_size_kb},{total_samples},{time_precision}")
    #[arg(
        long,
        default_value = "{file},{sample_rate},{bits_per_sample},{channels},{duration},{file_size_kb},{total_samples},{time_precision},{sample_accurate_duration}"
    )]
    output_format: String,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let args = cli.args;

    let mut output_file = args
        .output
        .map(|path| File::create(path).expect("Failed to create output file"));

    for entry in get_walker(&args.input, args.recursive) {
        if let Some(ext) = entry.path().extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();

            if is_audio_file(&ext_str) {
                let file_size = fs::metadata(entry.path())
                    .map(|m| format_size(m.len()))
                    .unwrap_or_else(|_| "Unknown size".to_string());

                // ffprobeによる情報取得
                let probe_result = run_ffprobe(
                    entry.path(),
                    &["-print_format", "json", "-show_format", "-show_streams"],
                );

                let mut file_info = AudioFileInfo::new(
                    entry.path().file_name().map_or_else(
                        || "N/A".to_string(),
                        |name| name.to_string_lossy().to_string(),
                    ),
                    ext_str.to_uppercase(),
                    file_size,
                );

                if ext_str == "wav" {
                    if let Ok(mut file) = File::open(entry.path()) {
                        match WavHeader::read_from_file(&mut file) {
                            Ok(header) => {
                                file_info.sample_rate = Some(header.sample_rate());
                                file_info.bits_per_sample = Some(header.bits_per_sample());
                                file_info.channels = Some(header.num_channels());
                                file_info.total_samples = header.total_samples();
                                file_info.time_precision = header.time_precision();
                            }
                            Err(e) => {
                                eprintln!(
                                    "Error reading WAV header for {}: {}",
                                    entry.path().display(),
                                    e
                                );
                            }
                        }
                    }
                }

                match probe_result {
                    Ok(json_output) => {
                        match serde_json::from_str::<Value>(&json_output) {
                            Ok(parsed_json) => {
                                if let Some(format) = parsed_json.get("format") {
                                    if let Some(duration_str) =
                                        format.get("duration").and_then(|v| v.as_str())
                                    {
                                        file_info.duration = duration_str.parse::<f64>().ok();
                                    }
                                }

                                if let Some(streams) =
                                    parsed_json.get("streams").and_then(|v| v.as_array())
                                {
                                    if let Some(audio_stream) = streams.iter().find(|s| {
                                        s.get("codec_type").and_then(|v| v.as_str())
                                            == Some("audio")
                                    }) {
                                        if file_info.sample_rate.is_none() {
                                            file_info.sample_rate = audio_stream
                                                .get("sample_rate")
                                                .and_then(|v| v.as_str())
                                                .and_then(|s| s.parse::<u32>().ok());
                                        }
                                        if file_info.bits_per_sample.is_none() {
                                            file_info.bits_per_sample = audio_stream
                                                .get("bits_per_sample")
                                                .and_then(|v| v.as_str())
                                                .and_then(|s| s.parse::<u16>().ok());
                                        }
                                        if file_info.channels.is_none() {
                                            file_info.channels = audio_stream
                                                .get("channels")
                                                .and_then(|v| v.as_u64())
                                                .map(|c| c as u16);
                                        }
                                        if file_info.total_samples.is_none() {
                                            file_info.total_samples = audio_stream
                                                .get("nb_samples")
                                                .and_then(|v| v.as_str())
                                                .and_then(|s| s.parse::<u64>().ok());
                                        }

                                        // Fallback for total_samples and time_precision if not directly available
                                        if file_info.total_samples.is_none() {
                                            if let (Some(sr), Some(dur)) =
                                                (file_info.sample_rate, file_info.duration)
                                            {
                                                file_info.total_samples =
                                                    Some((sr as f64 * dur) as u64);
                                            }
                                        }
                                        if file_info.time_precision.is_none() {
                                            file_info.time_precision =
                                                file_info.sample_rate.map(|sr| 1.0 / sr as f64);
                                        }

                                        if let (Some(ts), Some(tp)) =
                                            (file_info.total_samples, file_info.time_precision)
                                        {
                                            file_info.sample_accurate_duration =
                                                Some(ts as f64 * tp);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!(
                                    "Error parsing ffprobe JSON for {}: {}",
                                    entry.path().display(),
                                    e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "File: {}\nError: Failed to get audio info: {}",
                            entry.path().display(),
                            e
                        );
                    }
                }

                let formatted_output = file_info.format_output(&args.output_format);

                if let Some(file) = &mut output_file {
                    writeln!(file, "{}", formatted_output).expect("Failed to write to output file");
                } else {
                    println!("{}", formatted_output);
                }
            }
        }
    }
}
