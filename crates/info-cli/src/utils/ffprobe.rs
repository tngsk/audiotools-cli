use std::path::Path;
use std::process::Command;

pub fn run_ffprobe(path: &Path, args: &[&str]) -> Result<String, std::io::Error> {
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("quiet")
        .args(args)
        .arg(path)
        .output()?;

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
