use crate::core::audio::processor::process_time_range;
use crate::core::config::SpectrogramConfig;
use crate::core::{AudioLoader, SpectralAnalyzer, SpectrogramRenderer};
use crate::domain::audio_data::AudioData;
use crate::domain::request::SpectrumRequest;
use crate::domain::spectrogram::{Spectrogram, SpectrogramMetadata};
use crate::error::SpectrumError;
use std::path::PathBuf;
use std::time::Duration;

pub struct SpectrumCommand {
    audio_loader: Box<dyn AudioLoader>,
    analyzer: Box<dyn SpectralAnalyzer>,
    renderer: Box<dyn SpectrogramRenderer>,
}

impl SpectrumCommand {
    pub fn new(
        loader: Box<dyn AudioLoader>,
        analyzer: Box<dyn SpectralAnalyzer>,
        renderer: Box<dyn SpectrogramRenderer>,
    ) -> Self {
        Self {
            audio_loader: loader,
            analyzer,
            renderer,
        }
    }

    pub async fn execute(
        &self,
        request: SpectrumRequest,
    ) -> Result<SpectrumResponse, SpectrumError> {
        let start_time_overall = std::time::Instant::now();

        let audio_data = self.load_audio_data(&request.input_path)?;

        let (config, start_time, end_time, analysis_duration_ms) =
            self.determine_time_range_and_config(&request, &audio_data)?;

        let samples = self.extract_samples(&audio_data, start_time, end_time)?;

        let spectrogram = self.analyze_and_render(
            &request,
            &config,
            samples,
            start_time,
            end_time,
            analysis_duration_ms,
        )?;

        let processing_time = start_time_overall.elapsed();

        Ok(SpectrumResponse {
            output_path: request.output_path,
            metadata: spectrogram.metadata,
            processing_time,
            config_used: config,
        })
    }

    fn load_audio_data(&self, input_path: &PathBuf) -> Result<AudioData, SpectrumError> {
        self.audio_loader.load(input_path)
    }

    fn determine_time_range_and_config(
        &self,
        request: &SpectrumRequest,
        audio_data: &AudioData,
    ) -> Result<(SpectrogramConfig, f32, f32, f32), SpectrumError> {
        let total_duration = audio_data.samples.len() as f32 / audio_data.sample_rate;

        let (start_time, end_time) = process_time_range(
            &audio_data.samples,
            audio_data.sample_rate,
            request.time_range.clone(),
            request.auto_start.clone(),
            total_duration,
        )?;

        let analysis_duration = end_time - start_time;
        let analysis_duration_ms = analysis_duration * 1000.0;

        let mut config = if request.config.window_size == 0 {
            SpectrogramConfig::auto_configure(
                audio_data.sample_rate,
                request.config.min_freq,
                request.config.max_freq,
                analysis_duration_ms,
            )?
        } else {
            let mut cfg = request.config.clone();
            cfg.analysis_duration_ms = analysis_duration_ms;
            cfg
        };

        config.sample_rate = audio_data.sample_rate;

        println!(
            "Analysis: {:.1}ms ({:.3}s to {:.3}s) | Window: {} | Hop: {} | Overlap: {:.1}%",
            analysis_duration_ms,
            start_time,
            end_time,
            config.window_size,
            config.hop_size,
            (1.0 - (config.hop_size as f32 / config.window_size as f32)) * 100.0
        );

        Ok((config, start_time, end_time, analysis_duration_ms))
    }

    fn extract_samples<'a>(
        &self,
        audio_data: &'a AudioData,
        start_time: f32,
        end_time: f32,
    ) -> Result<&'a [f32], SpectrumError> {
        let start_sample = (start_time * audio_data.sample_rate) as usize;
        let end_sample = (end_time * audio_data.sample_rate) as usize;
        Ok(&audio_data.samples[start_sample..end_sample.min(audio_data.samples.len())])
    }

    fn analyze_and_render(
        &self,
        request: &SpectrumRequest,
        config: &SpectrogramConfig,
        samples: &[f32],
        start_time: f32,
        end_time: f32,
        analysis_duration_ms: f32,
    ) -> Result<Spectrogram, SpectrumError> {
        let (spectrogram_data, time_axis, freq_axis) = self.analyzer.analyze(samples)?;

        let spectrogram = Spectrogram {
            data: spectrogram_data,
            time_axis,
            freq_axis,
            metadata: SpectrogramMetadata {
                config: config.clone(),
                input_path: request.input_path.clone(),
                start_time,
                end_time,
                analysis_duration_ms,
                annotations: request.annotations.clone(),
            },
        };

        self.renderer.render(&spectrogram, &request.output_path)?;
        Ok(spectrogram)
    }

    // バッチ処理用 (TODO: Implement)
    pub async fn execute_batch(
        &self,
        requests: Vec<SpectrumRequest>,
    ) -> Vec<Result<SpectrumResponse, SpectrumError>> {
        // This needs to be async and use join_all if concurrent, or a simple loop if sequential.
        // For now, let's make it a simple loop.
        let mut results = Vec::with_capacity(requests.len());
        for req in requests {
            results.push(self.execute(req).await);
        }
        results
    }
}

#[derive(Debug, Clone)]
pub struct SpectrumResponse {
    pub output_path: PathBuf,
    pub metadata: SpectrogramMetadata,
    pub processing_time: Duration,
    pub config_used: SpectrogramConfig,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::frequency::parse_frequency_annotation;
    use crate::utils::time::{TimeRange, TimeSpecification};

    #[test]
    fn test_parse_frequency_annotation() {
        let result = parse_frequency_annotation("440:A4");
        assert!(result.is_ok());
        let (freq, label) = result.unwrap();
        assert_eq!(freq, 440.0);
        assert_eq!(label, "A4");

        let result = parse_frequency_annotation("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_time_range_adaptive_config() {
        // Create a mock audio data (10 seconds)
        let sample_rate = 44100.0;
        let samples = vec![0.0; (10.0 * sample_rate) as usize];
        let total_duration = 10.0;

        // Test short time range (100ms) within long audio
        let time_range = Some(TimeRange {
            start: TimeSpecification::Seconds(5.0),
            end: TimeSpecification::Seconds(5.1), // 100ms duration
        });

        let (start_time, end_time) =
            process_time_range(&samples, sample_rate, time_range, None, total_duration).unwrap();

        let analysis_duration_ms = (end_time - start_time) * 1000.0;
        assert!((analysis_duration_ms - 100.0).abs() < 1.0); // Should be ~100ms

        // Auto-configure should use small window size for this short duration
        let config =
            SpectrogramConfig::auto_configure(sample_rate, 20.0, 20000.0, analysis_duration_ms)
                .unwrap();

        assert_eq!(config.window_size, 256); // Should use small window for 100ms
        assert_eq!(config.hop_size, 7); // 97% overlap for very short duration
    }
}
