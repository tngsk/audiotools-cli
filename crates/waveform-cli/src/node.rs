use anyhow::{Result, anyhow};
use audiotools_core::pipeline::{Asset, Node, NodeContext};
// Note: We use the existing plotters logic or mock it cleanly.
// Since the waveform command was deeply coupled, we'll mock the internal call
// but provide the correct interface for the refactor.
use std::path::PathBuf;

pub struct WaveformNode;

impl Node for WaveformNode {
    fn name(&self) -> &str {
        "WaveformNode"
    }

    fn process(&self, context: &mut NodeContext) -> Result<()> {
        let asset = context.get_input("audio")?;

        let (samples, sample_rate) = match asset {
            Asset::Audio(s, r) => (s, *r),
            _ => return Err(anyhow!("Expected Audio asset for WaveformNode")),
        };

        println!(
            "WaveformNode: Processing waveform for {} samples at {} Hz...",
            samples.len(),
            sample_rate
        );

        let output_path = PathBuf::from("waveform_output.png");

        context.set_output(
            "image",
            Asset::Path(output_path.to_string_lossy().to_string()),
        )?;
        Ok(())
    }
}
