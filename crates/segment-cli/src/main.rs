use anyhow::Result;
use clap::Parser;
use indicatif::ProgressBar;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use audiotools_core::config::Config;

mod segmenter;

use segmenter::AudioSegmenter;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    input: PathBuf,

    #[arg(short, long)]
    output: Option<PathBuf>,

    #[arg(long)]
    segment_len: Option<f32>,

    #[arg(long)]
    top_db: Option<i32>,

    #[arg(short, long, default_value_t = false)]
    overwrite: bool,

    #[arg(short, long, default_value_t = false)]
    recursive: bool,

    #[arg(long)]
    ext: Option<String>,
}

fn main() -> Result<()> {
    let config = Config::load_default().unwrap_or_default();
    let args = Args::parse();

    // Resolve parameters
    let seg_config = config.segment.unwrap_or_default();

    let segment_len = args.segment_len.or(seg_config.segment_len).unwrap_or(2.0);

    let top_db = args.top_db.or(seg_config.top_db).unwrap_or(20);

    let recursive = args.recursive
        || config
            .global
            .as_ref()
            .and_then(|g| g.recursive)
            .unwrap_or(false);
    let overwrite = args.overwrite
        || config
            .global
            .as_ref()
            .and_then(|g| g.overwrite)
            .unwrap_or(false);

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
            // Non-recursive: just read dir
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

    println!("Found {} files.", files.len());

    // 2. Setup Segmenter
    let segmenter = AudioSegmenter::new(segment_len, top_db);

    // 3. Output directory
    let output_root = args
        .output
        .unwrap_or_else(|| PathBuf::from("segments_output"));
    if !output_root.exists() {
        std::fs::create_dir_all(&output_root)?;
    }
    println!("Output directory: {:?}", output_root);

    // 4. Process
    let pb = ProgressBar::new(files.len() as u64);

    for file_path in files {
        pb.inc(1);
        // Load
        match segmenter.load_audio(&file_path) {
            Ok((y, sr)) => {
                // Segment
                let segments = segmenter.segment_audio(&y, sr);
                if segments.is_empty() {
                    // println!("No segments generated for {:?}", file_path);
                    continue;
                }

                // Save
                if let Err(e) =
                    segmenter.save_segments(&segments, sr, &file_path, &output_root, overwrite)
                {
                    eprintln!("Error saving segments for {:?}: {}", file_path, e);
                }
            }
            Err(e) => {
                eprintln!("Error loading {:?}: {}", file_path, e);
            }
        }
    }
    pb.finish_with_message("Done");

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

    // Default extensions
    matches!(
        ext.as_str(),
        "wav" | "mp3" | "flac" | "ogg" | "aiff" | "m4a"
    )
}
