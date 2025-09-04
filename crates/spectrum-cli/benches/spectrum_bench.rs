use audiotools::command::spectrum::command::SpectrumCommand;
use audiotools::command::spectrum::core::analysis::DefaultSpectralAnalyzer;
use audiotools::command::spectrum::core::config::{SpectrogramConfig, WindowType};
use audiotools::command::spectrum::domain::request::{SpectrumOptions, SpectrumRequest};
use audiotools::command::spectrum::testing::{
    create_chirp_audio, create_harmonic_audio, create_test_audio, MockAudioLoader,
    MockSpectrogramRenderer,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::path::PathBuf;

fn benchmark_fft_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("fft_processing");

    // Test different window sizes
    for window_size in [512, 1024, 2048, 4096] {
        group.bench_function(format!("window_{}", window_size), |b| {
            let config = SpectrogramConfig {
                window_size,
                hop_size: window_size / 4,
                sample_rate: 44100.0,
                min_freq: 20.0,
                max_freq: 20000.0,
                image_width: 800,
                image_height: 600,
                window_type: WindowType::Hanning,
                analysis_duration_ms: 1000.0,
            };

            let analyzer = DefaultSpectralAnalyzer::new(config);
            let samples = create_test_audio(440.0, 1.0, 44100.0).samples;

            b.iter(|| {
                analyzer.analyze(black_box(&samples)).unwrap();
            });
        });
    }

    group.finish();
}

fn benchmark_audio_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("audio_generation");

    group.bench_function("sine_wave_1s", |b| {
        b.iter(|| {
            create_test_audio(black_box(440.0), black_box(1.0), black_box(44100.0));
        });
    });

    group.bench_function("harmonic_1s", |b| {
        let harmonics = vec![0.5, 0.25, 0.125];
        b.iter(|| {
            create_harmonic_audio(
                black_box(100.0),
                black_box(&harmonics),
                black_box(1.0),
                black_box(44100.0),
            );
        });
    });

    group.bench_function("chirp_1s", |b| {
        b.iter(|| {
            create_chirp_audio(
                black_box(100.0),
                black_box(1000.0),
                black_box(1.0),
                black_box(44100.0),
            );
        });
    });

    group.finish();
}

fn benchmark_spectrum_command(c: &mut Criterion) {
    let mut group = c.benchmark_group("spectrum_command");

    // Create a runtime for async benchmarks
    let runtime = tokio::runtime::Runtime::new().unwrap();

    group.bench_function("end_to_end_small", |b| {
        let loader = Box::new(
            MockAudioLoader::new()
                .with_sample_rate(44100.0)
                .with_duration(0.1)
                .with_sine_wave(440.0),
        );
        let config = SpectrogramConfig {
            window_size: 512,
            hop_size: 128,
            sample_rate: 44100.0,
            min_freq: 20.0,
            max_freq: 20000.0,
            image_width: 800,
            image_height: 600,
            window_type: WindowType::Hanning,
            analysis_duration_ms: 100.0,
        };
        let analyzer = Box::new(DefaultSpectralAnalyzer::new(config.clone()));
        let renderer = Box::new(MockSpectrogramRenderer::new());

        let command = SpectrumCommand::new(loader, analyzer, renderer);

        let request = SpectrumRequest {
            input_path: PathBuf::from("test.wav"),
            output_path: PathBuf::from("output.png"),
            config,
            time_range: None,
            auto_start: None,
            annotations: vec![],
            options: SpectrumOptions::default(),
        };

        b.iter(|| {
            runtime.block_on(async {
                command.execute(black_box(request.clone())).await.unwrap();
            });
        });
    });

    group.bench_function("end_to_end_large", |b| {
        let loader = Box::new(
            MockAudioLoader::new()
                .with_sample_rate(44100.0)
                .with_duration(10.0)
                .with_sine_wave(440.0),
        );
        let config = SpectrogramConfig {
            window_size: 2048,
            hop_size: 512,
            sample_rate: 44100.0,
            min_freq: 20.0,
            max_freq: 20000.0,
            image_width: 1920,
            image_height: 1080,
            window_type: WindowType::Blackman,
            analysis_duration_ms: 10000.0,
        };
        let analyzer = Box::new(DefaultSpectralAnalyzer::new(config.clone()));
        let renderer = Box::new(MockSpectrogramRenderer::new());

        let command = SpectrumCommand::new(loader, analyzer, renderer);

        let request = SpectrumRequest {
            input_path: PathBuf::from("test.wav"),
            output_path: PathBuf::from("output.png"),
            config,
            time_range: None,
            auto_start: None,
            annotations: vec![],
            options: SpectrumOptions::default(),
        };

        b.iter(|| {
            runtime.block_on(async {
                command.execute(black_box(request.clone())).await.unwrap();
            });
        });
    });

    group.finish();
}

fn benchmark_config_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_creation");

    group.bench_function("auto_configure", |b| {
        b.iter(|| {
            SpectrogramConfig::auto_configure(
                black_box(44100.0),
                black_box(20.0),
                black_box(20000.0),
                black_box(100.0),
            )
            .unwrap();
        });
    });

    group.bench_function("for_short_audio", |b| {
        b.iter(|| {
            SpectrogramConfig::for_short_audio(
                black_box(44100.0),
                black_box(100.0),
                black_box(10000.0),
                black_box(50.0),
            )
            .unwrap();
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_fft_processing,
    benchmark_audio_generation,
    benchmark_spectrum_command,
    benchmark_config_creation
);
criterion_main!(benches);
