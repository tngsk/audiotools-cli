use anyhow::{anyhow, Result};
use audiotools_core::pipeline::{Asset, Node, NodeContext};
use std::collections::HashMap;

pub struct FeaturesNode {
    pub extract_rms: bool,
    pub extract_zcr: bool,
}

impl Node for FeaturesNode {
    fn name(&self) -> &str {
        "FeaturesNode"
    }

    fn process(&self, context: &mut NodeContext) -> Result<()> {
        let asset = context.get_input("audio")?;

        let (samples, _) = match asset {
            Asset::Audio(s, r) => (s, *r),
            _ => return Err(anyhow!("Expected Audio asset for FeaturesNode")),
        };

        let mut features = HashMap::new();

        if self.extract_rms {
            let sum_sq: f32 = samples.iter().fold(0.0, |acc, &x| acc + x * x);
            let rms = (sum_sq / samples.len() as f32).sqrt();
            features.insert("rms".to_string(), vec![rms]);
        }

        if self.extract_zcr {
            let mut crossings = 0;
            for i in 1..samples.len() {
                // Optimization: simplified boolean branch reduces overhead
                if (samples[i] >= 0.0) != (samples[i - 1] >= 0.0) {
                    crossings += 1;
                }
            }
            let zcr = crossings as f32 / samples.len() as f32;
            features.insert("zcr".to_string(), vec![zcr]);
        }

        context.set_output("features", Asset::Features(features))?;
        Ok(())
    }
}
