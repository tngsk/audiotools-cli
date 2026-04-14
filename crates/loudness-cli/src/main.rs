use clap::Parser;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

mod utils;
use crate::utils::{format_size, get_walker, is_audio_file};

use audiotools_core::config::Config;
use ebur128::{EbuR128, Mode};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(flatten)]
    args: LoudnessArgs,
}

#[derive(clap::Args)]
struct LoudnessArgs {
    /// Input directory or file path
    #[arg(short, long)]
    input: PathBuf,

    /// Output file for measurements
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Process directories recursively
    #[arg(short, long, default_value_t = false)]
    recursive: bool,
}

#[tokio::main]
async fn main() {
    let config = Config::load_default().unwrap_or_default();
    let cli = Cli::parse();
    let args = cli.args;

    let recursive = args.recursive
        || config
            .global
            .as_ref()
            .and_then(|g| g.recursive)
            .unwrap_or(false);

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

                match measure_loudness(entry.path(), &ext_str) {
                    Ok(loudness_results) => {
                        let formatted_output = format!(
                            "File: {}\nFormat: {}\nSize: {}\nLoudness Analysis:\n{}\n",
                            entry.path().display(),
                            ext_str.to_uppercase(),
                            file_size,
                            loudness_results
                        );

                        if let Some(file) = &mut output_file {
                            writeln!(file, "{}", formatted_output)
                                .expect("Failed to write to output file");
                        } else {
                            println!("{}", formatted_output);
                        }
                    }
                    Err(e) => {
                        let error_msg = format!(
                            "File: {}\nError: Failed to measure loudness: {}\n",
                            entry.path().display(),
                            e
                        );
                        if let Some(file) = &mut output_file {
                            writeln!(file, "{}", error_msg)
                                .expect("Failed to write to output file");
                        } else {
                            eprintln!("{}", error_msg);
                        }
                    }
                }
            }
        }
    }
}

fn measure_loudness(path: &Path, ext: &str) -> Result<String, Box<dyn std::error::Error>> {
    let src = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(src), Default::default());

    let mut hint = Hint::new();
    hint.with_extension(ext);

    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts: FormatOptions = Default::default();

    let probed = symphonia::default::get_probe().format(&hint, mss, &fmt_opts, &meta_opts)?;

    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or("No supported audio track found")?;

    let params = &track.codec_params;
    let sample_rate = params.sample_rate.ok_or("Unknown sample rate")?;
    let channels = params.channels.ok_or("Unknown channels")?.count() as u32;

    let mut decoder = symphonia::default::get_codecs().make(params, &Default::default())?;

    let mut ebu = EbuR128::new(channels, sample_rate, Mode::I | Mode::LRA | Mode::TRUE_PEAK)?;

    let mut sample_buf: Option<SampleBuffer<f32>> = None;

    while let Ok(packet) = format.next_packet() {
        let decoded = decoder.decode(&packet)?;

        if sample_buf.is_none() {
            let spec = *decoded.spec();
            let duration = decoded.capacity() as u64;
            sample_buf = Some(SampleBuffer::new(duration, spec));
        }

        if let Some(buf) = sample_buf.as_mut() {
            buf.copy_interleaved_ref(decoded);
            ebu.add_frames_f32(buf.samples())?;
        }
    }

    let integrated = ebu.loudness_global()?;
    let lra = ebu.loudness_range()?;

    let mut true_peaks = Vec::new();
    for i in 0..channels {
        true_peaks.push(ebu.true_peak(i)?);
    }
    let max_true_peak = true_peaks.into_iter().fold(f64::NEG_INFINITY, f64::max);
    let max_true_peak_db = 20.0 * max_true_peak.log10();

    Ok(format!(
        "  Integrated Loudness: {:.1} LUFS\n  Loudness Range: {:.1} LU\n  True Peak: {:.1} dBTP",
        integrated, lra, max_true_peak_db
    ))
}
