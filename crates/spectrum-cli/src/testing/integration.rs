use crate::command::{SpectrumCommand, SpectrumResponse};
use crate::core::analysis::DefaultSpectralAnalyzer;
use crate::core::config::{SpectrogramConfig, WindowType};
use crate::domain::frequency::FrequencyAnnotation;
use crate::domain::request::{SpectrumOptions, SpectrumRequest};
use crate::testing::fixtures::{configs, create_test_audio, create_test_request};
use crate::testing::mocks::{MockAudioLoader, MockSpectralAnalyzer, MockSpectrogramRenderer};
use crate::utils::time::{TimeRange, TimeSpecification};
use std::path::PathBuf;
use std::time::Duration;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_end_to_end_with_mocks() {
        // Setup
        let loader = Box::new(
            MockAudioLoader::new()
                .with_sample_rate(44100.0)
                .with_duration(1.0)
                .with_sine_wave(440.0),
        );
        let analyzer = Box::new(MockSpectralAnalyzer::new().with_dimensions(100, 512));
        let renderer = Box::new(MockSpectrogramRenderer::new());

        let command = SpectrumCommand::new(loader, analyzer, renderer);

        let request = SpectrumRequest {
            input_path: PathBuf::from("test.wav"),
            output_path: PathBuf::from("output.png"),
            config: configs::default_config(),
            time_range: None,
            auto_start: None,
            annotations: vec![],
            options: SpectrumOptions::default(),
        };

        // Execute
        let result = command.execute(request).await;

        // Assert
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.output_path, PathBuf::from("output.png"));
        assert!(response.processing_time > Duration::ZERO);
        assert_eq!(response.config_used.window_size, 1024);
    }

    #[tokio::test]
    async fn test_with_time_range() {
        // Setup
        let loader = Box::new(
            MockAudioLoader::new()
                .with_sample_rate(44100.0)
                .with_duration(10.0), // 10 second file
        );
        let analyzer = Box::new(MockSpectralAnalyzer::new());
        let renderer = Box::new(MockSpectrogramRenderer::new());

        let command = SpectrumCommand::new(loader, analyzer, renderer);

        let request = SpectrumRequest {
            input_path: PathBuf::from("test.wav"),
            output_path: PathBuf::from("output.png"),
            config: configs::default_config(),
            time_range: Some(TimeRange {
                start: TimeSpecification::Seconds(2.0),
                end: TimeSpecification::Seconds(3.0),
            }),
            auto_start: None,
            annotations: vec![],
            options: SpectrumOptions::default(),
        };

        // Execute
        let result = command.execute(request).await;

        // Assert
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.metadata.start_time, 2.0);
        assert_eq!(response.metadata.end_time, 3.0);
        assert!((response.metadata.analysis_duration_ms - 1000.0).abs() < 0.1);
    }

    #[tokio::test]
    async fn test_with_frequency_annotations() {
        let loader = Box::new(MockAudioLoader::new());
        let analyzer = Box::new(MockSpectralAnalyzer::new());
        let renderer = Box::new(MockSpectrogramRenderer::new());

        let command = SpectrumCommand::new(loader, analyzer, renderer);

        let annotations = vec![
            FrequencyAnnotation {
                frequency: 440.0,
                label: "A4".to_string(),
            },
            FrequencyAnnotation {
                frequency: 880.0,
                label: "A5".to_string(),
            },
        ];

        let request = SpectrumRequest {
            input_path: PathBuf::from("test.wav"),
            output_path: PathBuf::from("output.png"),
            config: configs::default_config(),
            time_range: None,
            auto_start: None,
            annotations: annotations.clone(),
            options: SpectrumOptions::default(),
        };

        let result = command.execute(request).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.metadata.annotations.len(), 2);
        assert_eq!(response.metadata.annotations[0].frequency, 440.0);
        assert_eq!(response.metadata.annotations[1].label, "A5");
    }

    #[tokio::test]
    async fn test_auto_configuration() {
        let loader = Box::new(
            MockAudioLoader::new()
                .with_sample_rate(44100.0)
                .with_duration(0.1), // Very short audio
        );
        let analyzer = Box::new(MockSpectralAnalyzer::new());
        let renderer = Box::new(MockSpectrogramRenderer::new());

        let command = SpectrumCommand::new(loader, analyzer, renderer);

        let mut config = configs::default_config();
        config.window_size = 0; // Trigger auto-configuration

        let request = SpectrumRequest {
            input_path: PathBuf::from("test.wav"),
            output_path: PathBuf::from("output.png"),
            config,
            time_range: None,
            auto_start: None,
            annotations: vec![],
            options: SpectrumOptions::default(),
        };

        let result = command.execute(request).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        // Auto-config should select small window for short audio
        assert_eq!(response.config_used.window_size, 256);
    }

    #[tokio::test]
    async fn test_batch_processing() {
        let loader = Box::new(MockAudioLoader::new());
        let analyzer = Box::new(MockSpectralAnalyzer::new());
        let renderer = Box::new(MockSpectrogramRenderer::new());

        let command = SpectrumCommand::new(loader, analyzer, renderer);

        let requests = vec![
            create_test_request(PathBuf::from("test1.wav"), PathBuf::from("output1.png")),
            create_test_request(PathBuf::from("test2.wav"), PathBuf::from("output2.png")),
            create_test_request(PathBuf::from("test3.wav"), PathBuf::from("output3.png")),
        ];

        let results = command.execute_batch(requests).await;

        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.is_ok()));

        let paths: Vec<PathBuf> = results
            .into_iter()
            .map(|r| r.unwrap().output_path)
            .collect();

        assert_eq!(paths[0], PathBuf::from("output1.png"));
        assert_eq!(paths[1], PathBuf::from("output2.png"));
        assert_eq!(paths[2], PathBuf::from("output3.png"));
    }

    #[tokio::test]
    async fn test_error_handling_audio_load() {
        use crate::testing::mocks::FailingAudioLoader;

        let loader = Box::new(FailingAudioLoader::new("File not found"));
        let analyzer = Box::new(MockSpectralAnalyzer::new());
        let renderer = Box::new(MockSpectrogramRenderer::new());

        let command = SpectrumCommand::new(loader, analyzer, renderer);

        let request = create_test_request(
            PathBuf::from("nonexistent.wav"),
            PathBuf::from("output.png"),
        );

        let result = command.execute(request).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("File not found"));
    }

    #[tokio::test]
    async fn test_error_handling_render() {
        let loader = Box::new(MockAudioLoader::new());
        let analyzer = Box::new(MockSpectralAnalyzer::new());
        let renderer =
            Box::new(MockSpectrogramRenderer::new().with_failure("Failed to write image"));

        let command = SpectrumCommand::new(loader, analyzer, renderer);

        let request = create_test_request(PathBuf::from("test.wav"), PathBuf::from("output.png"));

        let result = command.execute(request).await;

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Failed to write image")
        );
    }

    #[tokio::test]
    async fn test_different_window_types() {
        let loader = Box::new(MockAudioLoader::new());
        let analyzer = Box::new(MockSpectralAnalyzer::new());
        let renderer = Box::new(MockSpectrogramRenderer::new());

        let command = SpectrumCommand::new(loader, analyzer, renderer);

        for window_type in vec![
            WindowType::Hanning,
            WindowType::Hamming,
            WindowType::Blackman,
        ] {
            let mut config = configs::default_config();
            config.window_type = window_type;

            let request = SpectrumRequest {
                input_path: PathBuf::from("test.wav"),
                output_path: PathBuf::from(format!("output_{:?}.png", window_type)),
                config,
                time_range: None,
                auto_start: None,
                annotations: vec![],
                options: SpectrumOptions::default(),
            };

            let result = command.execute(request).await;
            assert!(result.is_ok());
            let response = result.unwrap();
            assert_eq!(response.config_used.window_type, window_type);
        }
    }

    #[tokio::test]
    async fn test_various_sample_rates() {
        use crate::testing::fixtures::sample_rates;

        let test_rates = vec![
            sample_rates::CD_QUALITY,
            sample_rates::DVD_QUALITY,
            sample_rates::HIGH_RES,
            sample_rates::LOW_QUALITY,
        ];

        for rate in test_rates {
            let loader = Box::new(
                MockAudioLoader::new()
                    .with_sample_rate(rate)
                    .with_duration(1.0),
            );
            let analyzer = Box::new(MockSpectralAnalyzer::new());
            let renderer = Box::new(MockSpectrogramRenderer::new());

            let command = SpectrumCommand::new(loader, analyzer, renderer);

            let mut config = configs::default_config();
            config.sample_rate = rate;

            let request = SpectrumRequest {
                input_path: PathBuf::from("test.wav"),
                output_path: PathBuf::from(format!("output_{}.png", rate)),
                config,
                time_range: None,
                auto_start: None,
                annotations: vec![],
                options: SpectrumOptions::default(),
            };

            let result = command.execute(request).await;
            assert!(result.is_ok());
            let response = result.unwrap();
            assert_eq!(response.config_used.sample_rate, rate);
        }
    }

    #[tokio::test]
    async fn test_frequency_range_limits() {
        let loader = Box::new(MockAudioLoader::new().with_sample_rate(44100.0));
        let analyzer = Box::new(MockSpectralAnalyzer::new());
        let renderer = Box::new(MockSpectrogramRenderer::new());

        let command = SpectrumCommand::new(loader, analyzer, renderer);

        // Test various frequency ranges
        let test_cases = vec![
            (20.0, 20000.0),  // Full range
            (100.0, 10000.0), // Limited range
            (1000.0, 5000.0), // Narrow range
        ];

        for (min_freq, max_freq) in test_cases {
            let mut config = configs::default_config();
            config.min_freq = min_freq;
            config.max_freq = max_freq;

            let request = SpectrumRequest {
                input_path: PathBuf::from("test.wav"),
                output_path: PathBuf::from(format!("output_{}-{}.png", min_freq, max_freq)),
                config,
                time_range: None,
                auto_start: None,
                annotations: vec![],
                options: SpectrumOptions::default(),
            };

            let result = command.execute(request).await;
            assert!(result.is_ok());
            let response = result.unwrap();
            assert_eq!(response.config_used.min_freq, min_freq);
            assert_eq!(response.config_used.max_freq, max_freq);
        }
    }
}
