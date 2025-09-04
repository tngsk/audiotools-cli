use clap::Parser;
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

mod utils;
use crate::utils::{format_size, get_walker, is_audio_file};

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
    #[arg(short, long)]
    recursive: bool,
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

            // 対応フォーマットのチェック
            if is_audio_file(&ext_str) {
                // ファイルサイズの取得と変換
                let file_size = fs::metadata(entry.path())
                    .map(|m| format_size(m.len()))
                    .unwrap_or_else(|_| "Unknown size".to_string());

                // ffmpegコマンドの実行
                let loudness_output = Command::new("ffmpeg")
                    .arg("-i")
                    .arg(entry.path())
                    .arg("-filter_complex")
                    .arg("ebur128=peak=true")
                    .arg("-f")
                    .arg("null")
                    .arg("-")
                    .output();

                match loudness_output {
                    Ok(output) => {
                        // 結果の出力
                        let info = String::from_utf8_lossy(&output.stderr);
                        let formatted_output = format!(
                            "File: {}\nFormat: {}\nSize: {}\nLoudness Analysis:\n{}\n",
                            entry.path().display(),
                            ext_str.to_uppercase(),
                            file_size,
                            // EBU R128の関連する行のみを抽出
                            info.lines()
                                .filter(|line| {
                                    line.contains("LUFS")
                                        || line.contains("LU")
                                        || line.contains("Summary")
                                        || line.contains("Integrated")
                                        || line.contains("Loudness")
                                        || line.contains("Range")
                                        || line.contains("True Peak")
                                })
                                .collect::<Vec<&str>>()
                                .join("\n")
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
