use anyhow::{Result, anyhow};
use audiotools_core::pipeline::{Asset, Node, NodeContext};
use ebur128::{EbuR128, Mode};

pub struct LoudnessNode;

impl Node for LoudnessNode {
    fn name(&self) -> &str {
        "LoudnessNode"
    }

    fn process(&self, context: &mut NodeContext) -> Result<()> {
        let asset = context.get_input("audio")?;

        let (samples, sample_rate) = match asset {
            Asset::Audio(s, r) => (s, *r),
            _ => return Err(anyhow!("Expected Audio asset for LoudnessNode")),
        };

        let mut ebu = EbuR128::new(1, sample_rate, Mode::I)
            .map_err(|e| anyhow!("EBU initialization failed: {}", e))?;

        ebu.add_frames_f32(samples)
            .map_err(|e| anyhow!("Failed to add frames to EBU R128: {}", e))?;

        let loudness = ebu.loudness_global().unwrap_or(-70.0);

        let msg = format!("Measured Integrated Loudness: {:.2} LUFS", loudness);

        context.set_output("loudness", Asset::String(msg))?;
        Ok(())
    }
}
