use anyhow::{Result, anyhow};
use audiotools_core::pipeline::{Asset, Node, NodeContext};

pub struct NormalizeNode {
    pub level_dbfs: f32,
}

impl Node for NormalizeNode {
    fn name(&self) -> &str {
        "NormalizeNode"
    }

    fn process(&self, context: &mut NodeContext) -> Result<()> {
        let asset = context.get_input("audio")?;

        let (samples, sample_rate) = match asset {
            Asset::Audio(s, r) => (s, *r),
            _ => return Err(anyhow!("Expected Audio asset for NormalizeNode")),
        };

        let current_peak = samples.iter().map(|s| s.abs()).fold(0.0_f32, f32::max);

        let current_peak_dbfs = if current_peak > 0.0 {
            20.0 * current_peak.log10()
        } else {
            -100.0
        };

        let gain_db = self.level_dbfs - current_peak_dbfs;
        let gain_multiplier = 10.0_f32.powf(gain_db / 20.0);

        let normalized_samples: Vec<f32> = samples
            .iter()
            .map(|&s| (s * gain_multiplier).clamp(-1.0, 1.0))
            .collect();

        context.set_output("audio", Asset::Audio(normalized_samples, sample_rate))?;
        Ok(())
    }
}
