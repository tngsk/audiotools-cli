sed -i 's/const SUPPORTED_FORMATS: &\[&str\] = &\["wav", "flac", "mp3"\];/const SUPPORTED_FORMATS: \&\[\&str\] = \&\["wav"\];/' crates/convert-cli/src/main.rs
sed -i 's/const DEFAULT_MP3_BITRATE: &str = "320k";//' crates/convert-cli/src/main.rs
sed -i 's/const DEFAULT_FLAC_COMPRESSION: &str = "8";//' crates/convert-cli/src/main.rs

sed -i 's/const SUPPORTED_FORMATS: &\[&str\] = &\["wav", "flac", "mp3"\];/const SUPPORTED_FORMATS: \&\[\&str\] = \&\["wav"\];/' crates/normalize-cli/src/command/convert.rs
sed -i 's/const DEFAULT_MP3_BITRATE: &str = "320k";//' crates/normalize-cli/src/command/convert.rs
sed -i 's/const DEFAULT_FLAC_COMPRESSION: &str = "8";//' crates/normalize-cli/src/command/convert.rs
