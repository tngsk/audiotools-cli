# Spectrum Module Architecture

## Overview

The spectrum module provides high-performance spectrogram generation and analysis capabilities for audio files. It has been completely refactored to follow clean architecture principles with clear separation of concerns.

## Architecture

### Layer Structure

```
spectrum/
├── command/        # Command pattern implementation
├── core/           # Core business logic
│   ├── audio/      # Audio loading and processing
│   ├── analysis/   # FFT and spectral analysis
│   └── config/     # Configuration management
├── domain/         # Domain entities
├── render/         # Visualization layer
├── testing/        # Test utilities
└── error.rs        # Unified error handling
```

### Key Components

#### 1. Command Layer (`command/`)
- **SpectrumCommand**: Main entry point implementing the command pattern
- **SpectrumResponse**: Response structure with metadata
- Handles orchestration between different components
- Supports both single and batch processing

#### 2. Core Layer (`core/`)

##### Audio Module (`core/audio/`)
- **AudioLoader trait**: Abstract interface for audio file loading
- **DefaultAudioLoader**: WAV file loading implementation
- **processor**: Time range and auto-start detection processing

##### Analysis Module (`core/analysis/`)
- **SpectralAnalyzer trait**: Abstract interface for spectral analysis
- **DefaultSpectralAnalyzer**: FFT-based spectrogram generation
- **FFTProcessor**: Core FFT processing with windowing
- **windowing**: Window function implementations (Hanning, Hamming, Blackman)

##### Config Module (`core/config/`)
- **ConfigBuilder**: Fluent API for configuration creation
- **SpectrogramConfig**: Main configuration structure
- **presets**: Predefined configuration presets
- **validator**: Configuration validation logic

#### 3. Domain Layer (`domain/`)
- **AudioData**: Audio sample representation
- **Spectrogram**: Spectrogram data structure
- **SpectrogramMetadata**: Processing metadata
- **FrequencyAnnotation**: Frequency marking support
- **SpectrumRequest/Response**: Request/response DTOs

#### 4. Render Layer (`render/`)
- **SpectrogramRenderer trait**: Abstract rendering interface
- **DefaultSpectrogramRenderer**: Plotters-based implementation
- **canvas**: Drawing surface management
- **colormap**: Color mapping algorithms
- **annotations**: Frequency annotation rendering
- **layout**: Layout calculations

#### 5. Error Handling (`error.rs`)
- **SpectrumError**: Unified error type with context support
- **ResultExt**: Extension trait for adding context to errors

## Usage Examples

### Basic Usage

```rust
use audiotools::command::spectrum::command::SpectrumCommand;
use audiotools::command::spectrum::core::audio::DefaultAudioLoader;
use audiotools::command::spectrum::core::analysis::DefaultSpectralAnalyzer;
use audiotools::command::spectrum::core::config::SpectrogramConfig;
use audiotools::command::spectrum::render::DefaultSpectrogramRenderer;
use audiotools::command::spectrum::domain::request::SpectrumRequest;

// Create configuration
let config = SpectrogramConfig::auto_configure(
    44100.0,  // sample rate
    20.0,     // min frequency
    20000.0,  // max frequency
    1000.0    // duration in ms
)?;

// Create command with dependencies
let command = SpectrumCommand::new(
    Box::new(DefaultAudioLoader),
    Box::new(DefaultSpectralAnalyzer::new(config.clone())),
    Box::new(DefaultSpectrogramRenderer),
);

// Create request
let request = SpectrumRequest {
    input_path: PathBuf::from("audio.wav"),
    output_path: PathBuf::from("spectrogram.png"),
    config,
    time_range: None,
    auto_start: None,
    annotations: vec![],
    options: Default::default(),
};

// Execute
let response = command.execute(request).await?;
```

### Advanced Configuration

```rust
use audiotools::command::spectrum::core::config::ConfigBuilder;
use audiotools::command::spectrum::core::config::{QualityLevel, WindowType};

let config = ConfigBuilder::new()
    .sample_rate(48000.0)
    .frequency_range(100.0, 10000.0)
    .window_size(2048)
    .quality_level(QualityLevel::High)
    .window_type(WindowType::Blackman)
    .image_dimensions(1920, 1080)
    .build()?;
```

### Time Range Processing

```rust
use audiotools::utils::time::{TimeRange, TimeSpecification};

let request = SpectrumRequest {
    // ... other fields
    time_range: Some(TimeRange {
        start: TimeSpecification::Seconds(1.0),
        end: TimeSpecification::Seconds(5.0),
    }),
    // ...
};
```

### Auto-Start Detection

```rust
use audiotools::utils::detection::AutoStartDetection;

let request = SpectrumRequest {
    // ... other fields
    auto_start: Some(AutoStartDetection {
        threshold: 0.01,
        window_size: 441,  // 10ms at 44.1kHz
        min_duration: 0.02, // 20ms
    }),
    // ...
};
```

### Frequency Annotations

```rust
use audiotools::command::spectrum::domain::frequency::FrequencyAnnotation;

let request = SpectrumRequest {
    // ... other fields
    annotations: vec![
        FrequencyAnnotation {
            frequency: 440.0,
            label: "A4".to_string(),
        },
        FrequencyAnnotation {
            frequency: 1000.0,
            label: "1kHz".to_string(),
        },
    ],
    // ...
};
```

## Testing

The module includes comprehensive testing utilities:

### Mock Implementations

- **MockAudioLoader**: Generate test audio (sine waves, harmonics, noise)
- **MockSpectralAnalyzer**: Generate test spectrogram data
- **MockSpectrogramRenderer**: Verify rendering without file I/O

### Test Fixtures

- Pre-defined audio generation functions
- Common configuration presets
- Pattern-based spectrogram generation

### Example Test

```rust
use audiotools::command::spectrum::testing::*;

#[tokio::test]
async fn test_spectrum_processing() {
    let loader = Box::new(
        MockAudioLoader::new()
            .with_sample_rate(44100.0)
            .with_duration(1.0)
            .with_sine_wave(440.0)
    );
    
    let analyzer = Box::new(MockSpectralAnalyzer::new());
    let renderer = Box::new(MockSpectrogramRenderer::new());
    
    let command = SpectrumCommand::new(loader, analyzer, renderer);
    
    let request = create_test_request(
        PathBuf::from("test.wav"),
        PathBuf::from("output.png")
    );
    
    let result = command.execute(request).await;
    assert!(result.is_ok());
}
```

## Performance Considerations

### Window Size Selection

The module automatically selects optimal window sizes based on audio duration:

- **< 150ms**: 256 samples (high time resolution)
- **< 500ms**: 512 samples
- **< 2s**: 1024 samples
- **≥ 2s**: 2048 samples (high frequency resolution)

### Overlap Configuration

Overlap is automatically adjusted based on duration:

- **Very short (< 100ms)**: 97% overlap
- **Short (< 500ms)**: 93% overlap
- **Medium (< 2s)**: 87.5% overlap
- **Long (≥ 2s)**: 75% overlap

### Memory Optimization

- Streaming processing for large files
- Efficient FFT caching
- Minimal data copying

## Configuration Reference

### SpectrogramConfig Fields

| Field | Type | Description | Default |
|-------|------|-------------|---------|
| `window_size` | `usize` | FFT window size (must be power of 2) | 2048 |
| `hop_size` | `usize` | Hop size between windows | 512 |
| `sample_rate` | `f32` | Audio sample rate (Hz) | 44100.0 |
| `min_freq` | `f32` | Minimum frequency to display (Hz) | 20.0 |
| `max_freq` | `f32` | Maximum frequency to display (Hz) | 20000.0 |
| `image_width` | `u32` | Output image width (pixels) | 1200 |
| `image_height` | `u32` | Output image height (pixels) | 600 |
| `window_type` | `WindowType` | Window function type | Hanning |

### Quality Levels

- **Fast**: 50% overlap, suitable for real-time
- **Standard**: 75% overlap, good balance
- **High**: 87.5% overlap, maximum quality

### Window Types

- **Hanning**: Good general purpose, low spectral leakage
- **Hamming**: Similar to Hanning, slightly different sidelobe behavior
- **Blackman**: Excellent sidelobe suppression, wider main lobe

## Error Handling

All errors are unified under `SpectrumError` with context support:

```rust
use audiotools::command::spectrum::error::ResultExt;

fn process_audio() -> Result<(), SpectrumError> {
    load_file("audio.wav")
        .context("Failed to load audio file")?;
    
    analyze_spectrum()
        .context("During spectral analysis")?;
    
    Ok(())
}
```

## Future Enhancements

- [ ] GPU acceleration for FFT processing
- [ ] Real-time streaming analysis
- [ ] Additional window functions
- [ ] Mel-frequency spectrograms
- [ ] Phase information visualization
- [ ] Interactive web-based viewer
- [ ] Multi-channel support
- [ ] Custom color maps
- [ ] Export to different formats (SVG, PDF)

## Contributing

When contributing to this module:

1. Maintain the clean architecture layers
2. Add tests for new functionality
3. Update documentation
4. Run benchmarks to ensure performance
5. Follow the existing error handling patterns

## License

This module is part of the audiotools project and follows the same license.