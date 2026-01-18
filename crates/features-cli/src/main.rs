use clap::Parser;
use std::path::{Path, PathBuf};
use anyhow::{Result, Context};
use walkdir::WalkDir;
use indicatif::ProgressBar;
use std::fs::File;


mod features;
use features::AudioFeatures;
use audiotools_core::dsp::Stft;
use audiotools_core::audio;

use audiotools_core::config::Config;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    input: PathBuf,

    #[arg(short, long)]
    output: Option<PathBuf>,

    #[arg(short, long, default_value_t = false)]
    recursive: bool,

    #[arg(long, default_value = "csv")]
    format: String, // csv or json

    #[arg(long)]
    ext: Option<String>,
}

fn main() -> Result<()> {
    let config = Config::load_default().unwrap_or_default();
    let args = Args::parse();
    
    // Resolve params
    let recursive = args.recursive || config.global.as_ref().and_then(|g| g.recursive).unwrap_or(false);

    // 1. Collect files
    let mut files = Vec::new();
    let exts: Option<Vec<String>> = args.ext.map(|s| {
        s.split(',')
            .map(|e| e.trim().trim_start_matches('.').to_lowercase())
            .collect()
    });

    if args.input.is_file() {
        files.push(args.input.clone());
    } else if args.input.is_dir() {
        if recursive {
            for entry in WalkDir::new(&args.input).follow_links(true) {
                let entry = entry?;
                if entry.file_type().is_file() {
                    if is_supported_audio(&entry.path(), &exts) {
                        files.push(entry.path().to_path_buf());
                    }
                }
            }
        } else {
             for entry in std::fs::read_dir(&args.input)? {
                let entry = entry?;
                if entry.file_type()?.is_file() {
                    if is_supported_audio(&entry.path(), &exts) {
                        files.push(entry.path());
                    }
                }
            }
        }
    } else {
        anyhow::bail!("Input path not found: {:?}", args.input);
    }

    if files.is_empty() {
        println!("No audio files found.");
        return Ok(());
    }
    
    println!("Found {} files to process.", files.len());
    
    let mut results = Vec::new();
    let pb = ProgressBar::new(files.len() as u64);

    let n_fft = 2048;
    let hop_length = 512;
    let stft = Stft::new(n_fft, hop_length);

    for file_path in files {
        pb.inc(1);
        
        match audio::load_audio(&file_path) {
            Ok((y, sr)) => {
                let rms = features::calculate_rms(&y);
                let zcr = features::calculate_zcr(&y);
                
                // STFT
                let magnitudes = stft.compute_magnitude(&y);
                
                let (centroid, rolloff, flatness, flux) = 
                    features::calculate_spectral_features(&magnitudes, sr, n_fft);
                
                results.push(AudioFeatures {
                    file_name: file_path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                    path: std::fs::canonicalize(&file_path).unwrap_or(file_path.clone()).to_string_lossy().to_string(),
                    rms,
                    zero_crossing_rate: zcr,
                    spectral_centroid: centroid,
                    spectral_rolloff: rolloff,
                    spectral_flatness: flatness,
                    spectral_flux: flux,
                });
            },
            Err(e) => {
                 eprintln!("Error processing {:?}: {}", file_path, e);
            }
        }
    }
    pb.finish_with_message("Done");

    if results.is_empty() {
        println!("No features extracted.");
        return Ok(());
    }

    // Output
    let output_path = args.output.unwrap_or_else(|| {
        let name = if args.format == "json" { "features.json" } else { "features.csv" };
        if args.input.is_dir() {
            args.input.join(name)
        } else {
            args.input.with_file_name(name)
        }
    });
    
    let file = File::create(&output_path).with_context(|| format!("Failed to create output file: {:?}", output_path))?;
    
    if args.format == "json" {
        serde_json::to_writer_pretty(file, &results)?;
    } else {
        let mut wtr = csv::Writer::from_writer(file);
        for record in &results {
            wtr.serialize(record)?;
        }
        wtr.flush()?;
    }
    
    println!("Saved features to {:?}", output_path);

    Ok(())
}

fn is_supported_audio(path: &Path, extensions: &Option<Vec<String>>) -> bool {
    let ext = match path.extension() {
        Some(e) => e.to_string_lossy().to_lowercase(),
        None => return false,
    };
    
    if let Some(valid_exts) = extensions {
        return valid_exts.contains(&ext);
    }
    matches!(ext.as_str(), "wav" | "mp3" | "flac" | "ogg" | "aiff" | "m4a")
}
