use clap::Parser;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

mod utils;
use crate::utils::{format_size, get_walker, is_audio_file};

use audiotools_core::config::Config;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

struct AudioFileInfo {
    file: String,
    format: String,
    file_size: String,
    sample_rate: Option<u32>,
    bits_per_sample: Option<u32>, // Symphonia usually returns u32 for bits
    channels: Option<u16>, // Symphonia channels count
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
    #[arg(short, long, default_value_t = false)]
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
    let config = Config::load_default().unwrap_or_default();
    let cli = Cli::parse();
    let args = cli.args;
    
    let recursive = args.recursive || config.global.as_ref().and_then(|g| g.recursive).unwrap_or(false);

    let mut output_file = args
        .output
        .map(|path| File::create(path).expect("Failed to create output file"));

    for entry in get_walker(&args.input, recursive) {
        if let Some(ext) = entry.path().extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();

            if is_audio_file(&ext_str) {
                let file_size = fs::metadata(entry.path())
                    .map(|m| format_size(m.len()))
                    .unwrap_or_else(|_| "Unknown size".to_string());

                // Symphonia probe
                if let Ok(info) = probe_audio(entry.path(), &ext_str, file_size.clone()) {
                     let formatted_output = info.format_output(&args.output_format);
                    if let Some(file) = &mut output_file {
                        writeln!(file, "{}", formatted_output).expect("Failed to write to output file");
                    } else {
                        println!("{}", formatted_output);
                    }
                } else {
                    eprintln!("Failed to probe audio file: {}", entry.path().display());
                }
            }
        }
    }
}

fn probe_audio(path: &Path, ext: &str, file_size: String) -> Result<AudioFileInfo, Box<dyn std::error::Error>> {
    let src = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    let mut hint = Hint::new();
    hint.with_extension(ext);

    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts: FormatOptions = Default::default();

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &fmt_opts, &meta_opts)?;

    let format = probed.format;
    
    // Default track (usually the first audio track)
    let track = format.tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or("No supported audio track found")?;

    let params = &track.codec_params;
    
    let mut info = AudioFileInfo::new(
        path.file_name().unwrap_or_default().to_string_lossy().to_string(),
        format_name(&format), // Basic format info
        file_size,
    );
    
    info.sample_rate = params.sample_rate;
    info.channels = params.channels.map(|c| c.count() as u16);
    info.bits_per_sample = params.bits_per_sample;
    info.total_samples = params.n_frames;

    if let Some(sr) = info.sample_rate {
         info.time_precision = Some(1.0 / sr as f64);
         
         // Calculate duration from frames if available
         if let Some(frames) = params.n_frames {
             info.duration = Some(frames as f64 / sr as f64);
         } else {
             // Try hint from format
             // Cannot easily get duration if n_frames is missing without scanning.
             // But some formats provide it?
             // Symphonia doesn't expose duration directly on FormatReader unless calculated.
             // We can check metadata tags potentially.
         }
         
         if let (Some(ts), Some(tp)) = (info.total_samples, info.time_precision) {
             info.sample_accurate_duration = Some(ts as f64 * tp);
         }
    }
    
    Ok(info)
}

fn format_name(_fmt: &Box<dyn symphonia::core::formats::FormatReader>) -> String {
    // Try to get a string representation if possible, or just return basic type
    // Symphonia doesn't explicitly expose a format name string easily in FormatReader trait?
    // We can infer from extension or probe result, but probe result consumed.
    // For now returning "Audio" or extension upper.
    "Audio".to_string() 
}

