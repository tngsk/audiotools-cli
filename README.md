# AudioTools CLI

AudioTools CLI is a collection of command-line utilities written in Rust for comprehensive audio file processing, analysis, and visualization. Designed for audio engineers, producers, and developers, it provides efficient tools for common audio tasks.

## Features

AudioTools CLI is composed of several specialized sub-commands:

### `audiotools convert` - Audio Format Conversion & Processing
Convert audio files between various formats, adjust bit depth, sample rate, channel count, and apply peak normalization.

-   **Supported Formats**: WAV (16/24-bit), FLAC, MP3, AAC, M4A, OGG, WMA, AIFF, ALAC, OPUS.
-   **Features**:
    -   Format conversion (e.g., WAV to MP3, FLAC to WAV).
    -   Bit depth adjustment (e.g., 16-bit to 24-bit WAV).
    -   Sample rate conversion.
    -   Channel conversion (mono/stereo).
    -   Peak normalization to a target dBFS level.
    -   Recursive directory processing.
    -   Customizable output filenames with prefixes/postfixes.
    -   Option to flatten output directory structure.

### `audiotools normalize` - Peak Level Normalization
Normalize audio files to a specified peak level (dBFS) by applying gain. This command is optimized for level adjustment.

-   **Features**:
    -   Detects current peak level and applies necessary gain.
    -   Supports various input formats.
    -   Recursive directory processing.
    -   Force overwrite existing files.

### `audiotools info` - Audio File Metadata Extraction
Extract detailed metadata from audio files, including format, size, and stream information using `ffprobe`. Provides specific WAV header details for `.wav` files.

-   **Features**:
    -   Extracts codec, sample rate, channels, bit rate, duration, and more.
    -   Specific WAV header parsing for `.wav` files.
    -   Filter output to display specific fields.
    -   Recursive directory processing.
    -   Output to console or a specified file.

### `audiotools loudness` - EBU R128 Loudness Analysis
Perform EBU R128 loudness measurements on audio files using `ffmpeg`, providing integrated loudness, loudness range, and true peak values.

-   **Features**:
    -   Calculates Integrated Loudness (LUFS).
    -   Determines Loudness Range (LRA).
    -   Measures True Peak (dBTP).
    -   Recursive directory processing.
    -   Output to console or a specified file.

### `audiotools spectrum` - Spectrogram Visualization
Generate high-resolution spectrogram images from audio files, offering detailed frequency analysis and visualization.

-   **Features**:
    -   Customizable FFT window size and overlap.
    -   Adjustable frequency range (min/max Hz) with presets (Full, AudioRange, SpeechRange, MusicRange, Bass).
    -   Adaptive mode for optimizing analysis of short audio segments.
    -   Time range selection for specific analysis segments.
    -   Automatic start/silence detection.
    -   Frequency annotations for highlighting specific frequencies.
    -   Outputs to PNG image.

### `audiotools waveform` - Waveform Visualization
Generate detailed waveform images from audio files, visualizing amplitude over time.

-   **Features**:
    -   Display scale options: Amplitude or Decibel (dBFS).
    -   Time range selection for specific visualization segments.
    -   Automatic start/silence detection.
    -   Time-based annotations for marking points of interest.
    -   Option to show RMS (Root Mean Square) envelope.
    -   Outputs to PNG image (currently supports WAV input for visualization).

### `fmtr` - JSON Output Formatter
A separate utility to convert the text output from `audiotools info` and `audiotools loudness` into a structured JSON format.

-   **Features**:
    -   Parses `info` command output into a JSON array of audio file details.
    -   Parses `loudness` command output into a JSON array of loudness measurements.

## Prerequisites

-   **Rust**: Latest stable version.
-   **FFmpeg**: Must be installed and accessible in your system's PATH. AudioTools CLI heavily relies on FFmpeg for audio processing and information extraction.

### Installing FFmpeg

-   **Ubuntu/Debian**: `sudo apt-get update && sudo apt-get install ffmpeg`
-   **macOS**: `brew install ffmpeg`
-   **Windows**: Download from the [FFmpeg official website](https://ffmpeg.org/download.html) and ensure it's added to your system PATH.

## Installation

1.  **Clone the repository**:
    ```bash
    git clone https://github.com/tngsk/audiotools-cli.git
    cd audiotools-cli
    ```

2.  **Build the project**:
    ```bash
    cargo build --release
    ```
    This will compile all sub-commands. The executables will be located in `target/release/`.
    You can then add `target/release/` to your system's PATH or copy the executables to a directory already in your PATH (e.g., `/usr/local/bin`).

## Usage

All commands are invoked via the `audiotools` binary, followed by the specific sub-command. The `fmtr` utility is a separate binary.

### `audiotools convert` Examples

```bash
# Convert all WAV files in 'input_dir' to 24-bit FLAC with -1.0 dBFS peak normalization
audiotools convert -i input_dir -O flac -b 24 --level -1.0 -r

# Convert 'input.wav' to mono MP3 (320kbps)
audiotools convert -i input.wav -O mp3 --channels 1

# Convert all FLAC files in 'input_dir' to 16-bit WAV, output to 'output_dir', overwrite existing
audiotools convert -i input_dir -O wav -b 16 -I flac -o output_dir --force -r
```

### `audiotools normalize` Examples

```bash
# Normalize all WAV files in 'input_dir' to -3.0 dBFS, preserving format
audiotools normalize -i input_dir --level -3.0 -r

# Normalize a single MP3 file to -0.5 dBFS, overwriting if it exists
audiotools normalize -i audio.mp3 --level -0.5 --force
```

### `audiotools info` Examples

```bash
# Get duration and bitrate for all audio files in 'input_dir' recursively
audiotools info -i input_dir -f duration,bit_rate -r

# Get full metadata for 'song.wav' and save to 'song_info.txt'
audiotools info -i song.wav -o song_info.txt
```

### `audiotools loudness` Examples

```bash
# Perform EBU R128 loudness analysis on all audio files in 'input_dir' recursively
audiotools loudness -i input_dir -r

# Analyze 'podcast.mp3' and save results to 'podcast_loudness.txt'
audiotools loudness -i podcast.mp3 -o podcast_loudness.txt
```

### `audiotools spectrum` Examples

```bash
# Generate a basic spectrogram for 'track.wav'
audiotools spectrum -i track.wav -o track_spectrum.png

# Generate a spectrogram with a specific window size, overlap, and frequency range
audiotools spectrum -i track.wav -o track_detailed_spectrum.png --window-size 4096 --overlap 0.85 --min-freq 50 --max-freq 10000

# Spectrogram with auto-start detection and music frequency preset
audiotools spectrum -i vocal.wav -o vocal_auto_spectrum.png --auto-start --freq-preset MusicRange

# Spectrogram with frequency annotations
audiotools spectrum -i synth.wav -o synth_annotated.png --annotate "440:A4,880:A5"
```

### `audiotools waveform` Examples

```bash
# Generate a basic waveform for 'clip.wav' with decibel scale and RMS envelope
audiotools waveform -i clip.wav -o clip_waveform.png --scale decibel --show-rms

# Waveform with auto-start detection and time annotations
audiotools waveform -i speech.wav -o speech_auto_waveform.png --auto-start --annotate "1.5:start,4.2:end"

# Waveform for a specific time range (10 to 20 seconds)
audiotools waveform -i long_track.wav -o long_track_segment.png --start 10 --end 20
```

### `fmtr` Examples

```bash
# Format the output of 'audiotools info' to JSON
audiotools info -i input.wav -o info_output.txt
fmtr -i info_output.txt -o info.json -t info

# Format the output of 'audiotools loudness' to JSON
audiotools loudness -i input.wav -o loudness_output.txt
fmtr -i loudness_output.txt -o loudness.json -t loudness
```

## Command Line Options

Each sub-command has its own set of options. Run `audiotools <subcommand> --help` for detailed usage.

Example: `audiotools convert --help`

## Dependencies

The project relies on the following key Rust crates:

-   `clap`: For robust command-line argument parsing.
-   `hound`: For reading and writing WAV files.
-   `plotters`: For powerful plotting and visualization capabilities (used in `spectrum` and `waveform`).
-   `rustfft`: For efficient Fast Fourier Transform computations (used in `spectrum`).
-   `walkdir`: For recursive directory traversal.
-   `rodio`: For audio playback and processing (used in `convert` and `normalize`).
-   `byteorder`: For handling byte order in binary data (used in `info`).
-   `serde` & `serde_json`: For serialization/deserialization, especially for `fmtr`.
-   `tokio`: For asynchronous operations.
-   `thiserror`: For custom error types.
-   `rand`: For random number generation.

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request or open an issue.

## Acknowledgments

This project heavily relies on [FFmpeg](https://ffmpeg.org/), an amazing open-source multimedia framework. Special thanks to the FFmpeg team and contributors for providing such a powerful and reliable tool that makes this project possible.