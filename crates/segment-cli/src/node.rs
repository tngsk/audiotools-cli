use anyhow::{anyhow, Result};
use audiotools_core::pipeline::{Asset, Node, NodeContext};

pub struct SegmentNode {
    pub segment_len: f32,
}

impl Node for SegmentNode {
    fn name(&self) -> &str {
        "SegmentNode"
    }

    fn process(&self, context: &mut NodeContext) -> Result<()> {
        let asset = context.get_input("audio")?;

        let (samples, sample_rate) = match asset {
            Asset::Audio(s, r) => (s, *r),
            _ => return Err(anyhow!("Expected Audio asset for SegmentNode")),
        };

        let samples_per_segment = (self.segment_len * sample_rate as f32) as usize;
        let mut segments = Vec::new();

        for chunk in samples.chunks(samples_per_segment) {
            segments.push((chunk.to_vec(), sample_rate));
        }

        context.set_output("audio_list", Asset::AudioList(segments))?;
        Ok(())
    }
}
