use anyhow::{Result, anyhow};
use audiotools_core::pipeline::{Asset, Node, NodeContext};

pub struct InfoNode;

impl Node for InfoNode {
    fn name(&self) -> &str {
        "InfoNode"
    }

    fn process(&self, context: &mut NodeContext) -> Result<()> {
        let asset = context.get_input("audio")?;

        let (samples, sample_rate) = match asset {
            Asset::Audio(s, r) => (s, *r),
            _ => return Err(anyhow!("Expected Audio asset for InfoNode")),
        };

        let duration = samples.len() as f32 / sample_rate as f32;
        let msg = format!(
            "Audio Info: {} samples, {} Hz, {:.2} seconds",
            samples.len(),
            sample_rate,
            duration
        );

        context.set_output("info", Asset::String(msg))?;
        Ok(())
    }
}
