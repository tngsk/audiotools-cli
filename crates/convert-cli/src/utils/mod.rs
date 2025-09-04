pub mod detection;

pub const AUDIO_EXTENSIONS: &[&str] = &[
    "wav", "flac", "mp3", "aac", "m4a", "ogg", "wma", "aiff", "alac", "opus",
];

use std::path::PathBuf;
use walkdir::WalkDir;

pub fn get_walker(input: &PathBuf, recursive: bool) -> impl Iterator<Item = walkdir::DirEntry> {
    let walker = if recursive {
        WalkDir::new(input)
    } else {
        WalkDir::new(input).max_depth(1)
    };
    walker.into_iter().filter_map(|e| e.ok())
}

pub fn is_audio_file(ext: &str) -> bool {
    AUDIO_EXTENSIONS.contains(&ext.to_lowercase().as_str())
}
