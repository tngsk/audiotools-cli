use clap::Parser;
use std::path::PathBuf;

use normalize_cli::command::convert;
use normalize_cli::utils::detection::detect_peak_level;
use normalize_cli::utils::get_walker;

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
    #[arg(short, long, default_value_t = -1.0, allow_negative_numbers = true)]
    level: f32,

    /// Input formats to process (e.g., wav,flac,mp3)
    #[arg(short = 'I', long, value_delimiter = ',', default_value = "wav")]
    input_format: Vec<String>,

    /// Process directories recursively
    #[arg(short, long)]
    recursive: bool,

    /// Force overwrite of existing files
    #[arg(long)]
    force: bool,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let args = cli.args;

    // 入力フォーマットを小文字に変換
    let input_extensions: Vec<String> =
        args.input_format.iter().map(|f| f.to_lowercase()).collect();

    // フォルダ内のファイルを走査
    for entry in get_walker(&args.input, args.recursive) {
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

                        let gain = args.level - peak_dbfs;
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
                            None,
                            Some(&format!("_normalized_{}dB", args.level)),
                            false,
                            args.force,
                            None,
                            Some(args.level),
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
