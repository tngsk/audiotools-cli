use crate::audio::{load_audio, save_wav};
use crate::pipeline::{Asset, Node, NodeContext};
use anyhow::Result;
use std::path::PathBuf;

pub struct AudioInputNode {
    pub filepath: PathBuf,
}

impl Node for AudioInputNode {
    fn name(&self) -> &str {
        "AudioInputNode"
    }

    fn process(&self, context: &mut NodeContext) -> Result<()> {
        let (samples, sample_rate) = load_audio(&self.filepath)?;
        context.set_output("audio", Asset::Audio(samples, sample_rate))?;
        Ok(())
    }
}

pub struct AudioOutputNode {
    pub filepath: PathBuf,
}

impl Node for AudioOutputNode {
    fn name(&self) -> &str {
        "AudioOutputNode"
    }

    fn process(&self, context: &mut NodeContext) -> Result<()> {
        let asset = context.get_input("audio")?;
        if let Asset::Audio(samples, sample_rate) = asset {
            save_wav(&self.filepath, samples, *sample_rate)?;
            Ok(())
        } else {
            anyhow::bail!("Expected Audio asset for AudioOutputNode");
        }
    }
}
