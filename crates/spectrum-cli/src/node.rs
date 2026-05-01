use crate::core::SpectrogramRenderer;
use crate::core::analysis::fft::FFTProcessor;
use crate::domain::request::SpectrumOptions;
use crate::render::DefaultSpectrogramRenderer;
use anyhow::{Result, anyhow};
use audiotools_core::pipeline::{Asset, Node, NodeContext};
use std::path::PathBuf;

pub struct SpectrumNode {
    pub options: SpectrumOptions,
}

impl Node for SpectrumNode {
    fn name(&self) -> &str {
        "SpectrumNode"
    }

    fn process(&self, context: &mut NodeContext) -> Result<()> {
        let asset = context.get_input("audio")?;

        let (samples, sample_rate) = match asset {
            Asset::Audio(s, r) => (s, *r),
            _ => return Err(anyhow!("Expected Audio asset for SpectrumNode")),
        };

        let mut config = crate::core::config::SpectrogramConfig::default();
        config.sample_rate = sample_rate as f32;

        let analyzer = FFTProcessor::new(config.clone());
        let spectrogram_data = analyzer
            .process_signal(samples)
            .map_err(|e| anyhow!("Analyzer failed: {:?}", e))?;

        // FFTProcessor doesn't return time_axis and freq_axis, we need to generate them
        let total_frames = spectrogram_data.len();
        let total_duration = samples.len() as f32 / sample_rate as f32;
        let time_step = if total_frames > 1 {
            total_duration / (total_frames - 1) as f32
        } else {
            total_duration
        };
        let freq_step = config.freq_resolution();

        let time_axis: Vec<f32> = (0..total_frames).map(|i| i as f32 * time_step).collect();
        let freq_axis: Vec<f32> = (0..spectrogram_data[0].len())
            .map(|i| i as f32 * freq_step)
            .collect();

        let start_time = 0.0;
        let end_time = total_duration;
        let analysis_duration_ms = end_time * 1000.0;

        let spectrogram = crate::domain::spectrogram::Spectrogram {
            data: spectrogram_data,
            time_axis,
            freq_axis,
            metadata: crate::domain::spectrogram::SpectrogramMetadata {
                config,
                input_path: PathBuf::from("memory"),
                start_time,
                end_time,
                analysis_duration_ms,
                annotations: vec![],
            },
        };

        let renderer = DefaultSpectrogramRenderer;
        let output_path = PathBuf::from("spectrum_output.png");
        renderer
            .render(&spectrogram, &output_path)
            .map_err(|e| anyhow!("Renderer failed: {:?}", e))?;

        context.set_output(
            "image",
            Asset::Path(output_path.to_string_lossy().to_string()),
        )?;
        Ok(())
    }
}
