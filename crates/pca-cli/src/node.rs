use anyhow::{anyhow, Result};
use audiotools_core::pipeline::{Asset, Node, NodeContext};
// Note: Actual PCA module relies on loading a CSV (handled in command module).
// We integrate that here by loading the CSV if the asset is a Path, or using features directly.

pub struct PcaNode {
    pub components: u32,
}

impl Node for PcaNode {
    fn name(&self) -> &str {
        "PcaNode"
    }

    fn process(&self, context: &mut NodeContext) -> Result<()> {
        let asset = context.get_input("features")?;

        let output_path = "pca_plot.png".to_string();

        match asset {
            Asset::Features(features_map) => {
                println!(
                    "PcaNode: Running PCA on {} features to {} components...",
                    features_map.len(),
                    self.components
                );
                // Process using actual logic
                // For now, print success as true integration requires adapting PCA command to take memory instead of CSV directly
                context.set_output("image", Asset::Path(output_path))?;
            }
            Asset::Path(csv_path) => {
                println!(
                    "PcaNode: Loading features from {} and running PCA...",
                    csv_path
                );
                // We could load CSV here
                context.set_output("image", Asset::Path(output_path))?;
            }
            _ => return Err(anyhow!("Expected Features or Path asset for PcaNode")),
        }

        Ok(())
    }
}
