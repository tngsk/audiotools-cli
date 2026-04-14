use clap::Parser;
use std::path::PathBuf;

mod command;
mod utils;
use crate::command::convert;
use crate::utils::detection::detect_peak_level;
use crate::utils::get_walker;

use audiotools_core::config::Config;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(flatten)]
    args: NormalizeArgs,
}

#[derive(clap::Args)]
struct NormalizeArgs {
    /// Input directory or file path
    #[arg(short, long)]
    input: PathBuf,

    /// Output directory path
    #[arg(short, long)]
    output_dir: Option<PathBuf>,

    /// Target peak level in dBFS (e.g., -1.0)
    #[arg(short, long, allow_negative_numbers = true)]
    level: Option<f32>,

    /// Input formats to process (e.g., wav,flac,mp3)
    #[arg(short = 'I', long, value_delimiter = ',')]
    input_format: Option<Vec<String>>,

    /// Process directories recursively
    #[arg(short, long, default_value_t = false)]
    recursive: bool,

    /// Force overwrite of existing files
    #[arg(long, default_value_t = false)]
    force: bool,
}

#[tokio::main]
async fn main() {
    let config = Config::load_default().unwrap_or_default();
    let cli = Cli::parse();
    let args = cli.args;

    // Resolve defaults
    let level = args
        .level
        .or(config.normalize.as_ref().and_then(|n| n.level))
        .unwrap_or(-1.0);

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

    // 入力フォーマットを小文字に変換
    let input_format_list = args.input_format.unwrap_or_else(|| vec!["wav".to_string()]);
    let input_extensions: Vec<String> =
        input_format_list.iter().map(|f| f.to_lowercase()).collect();

    // フォルダ内のファイルを走査
    for entry in get_walker(&args.input, recursive) {
        if let Some(ext) = entry.path().extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            if input_extensions.contains(&ext_str) {
                // 各ファイルのピークレベルを検出
                match detect_peak_level(&entry.path().to_path_buf()) {
                    Ok(peak_dbfs) => {
                        println!(
                            "Processing: {} (Peak level: {:.1} dBFS)",
                            entry.path().display(),
                            peak_dbfs
                        );

                        let gain = level - peak_dbfs;
                        println!("Applying gain: {:.1} dB", gain);

                        // 変換処理の実行
                        convert::convert_files(
                            &entry.path().to_path_buf(),
                            args.output_dir.as_ref(),
                            false,
                            &[ext_str],
                            "wav",
                            24,
                            None,
                            Some(&format!("_normalized_{}dB", level)),
                            false,
                            force,
                            None,
                            Some(level),
                        );
                    }
                    Err(e) => {
                        println!("Error processing {}: {}", entry.path().display(), e);
                        continue;
                    }
                }
            }
        }
    }
}
