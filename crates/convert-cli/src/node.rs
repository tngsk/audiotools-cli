use anyhow::{Result, anyhow};
use audiotools_core::pipeline::{Asset, Node, NodeContext};

pub struct ConvertNode {
    pub target_channels: Option<u16>,
}

impl Node for ConvertNode {
    fn name(&self) -> &str {
        "ConvertNode"
    }

    fn process(&self, context: &mut NodeContext) -> Result<()> {
        let asset = context.get_input("audio")?;

        let (samples, sample_rate) = match asset {
            Asset::Audio(s, r) => (s, *r),
            _ => return Err(anyhow!("Expected Audio asset for ConvertNode")),
        };

        // Simplified for pipeline: just output the samples as-is because the actual
        // conversion (channels, bit depth) happens during WAV writing in AudioOutputNode
        // or through a separate processor.

        context.set_output("audio", Asset::Audio(samples.clone(), sample_rate))?;
        Ok(())
    }
}
