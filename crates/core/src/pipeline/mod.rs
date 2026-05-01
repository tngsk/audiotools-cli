pub mod io;

use anyhow::{anyhow, Result};
use std::collections::HashMap;

/// Represents data flowing through the pipeline.
#[derive(Debug, Clone)]
pub enum Asset {
    /// Mono audio data with sample rate: (samples, sample_rate)
    Audio(Vec<f32>, u32),
    /// A list of audio segments, useful for splitting operations.
    AudioList(Vec<(Vec<f32>, u32)>),
    /// Features extracted from audio. Represents a map of feature names to values.
    Features(HashMap<String, Vec<f32>>),
    /// Represents a file path, e.g., to an image or saved audio file.
    Path(String),
    /// Represents a string value.
    String(String),
}

/// A store for assets during pipeline execution.
#[derive(Debug, Default)]
pub struct AssetStore {
    assets: HashMap<String, Asset>,
}

impl AssetStore {
    pub fn new() -> Self {
        Self {
            assets: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: impl Into<String>, asset: Asset) {
        self.assets.insert(key.into(), asset);
    }

    pub fn get(&self, key: &str) -> Result<&Asset> {
        self.assets
            .get(key)
            .ok_or_else(|| anyhow!("Asset with key '{}' not found in store", key))
    }

    pub fn take(&mut self, key: &str) -> Result<Asset> {
        self.assets
            .remove(key)
            .ok_or_else(|| anyhow!("Asset with key '{}' not found in store", key))
    }
}

/// Context passed to nodes during processing.
pub struct NodeContext<'a> {
    pub store: &'a mut AssetStore,
    pub inputs: &'a HashMap<String, String>,
    pub outputs: &'a HashMap<String, String>,
}

impl<'a> NodeContext<'a> {
    pub fn get_input(&self, local_key: &str) -> Result<&Asset> {
        let global_key = self
            .inputs
            .get(local_key)
            .ok_or_else(|| anyhow!("Input key '{}' not mapped in node configuration", local_key))?;
        self.store.get(global_key)
    }

    pub fn set_output(&mut self, local_key: &str, asset: Asset) -> Result<()> {
        let global_key = self.outputs.get(local_key).ok_or_else(|| {
            anyhow!(
                "Output key '{}' not mapped in node configuration",
                local_key
            )
        })?;
        self.store.insert(global_key, asset);
        Ok(())
    }
}

/// A node in the processing pipeline.
pub trait Node {
    /// The name of the node type, e.g., "NormalizeNode"
    fn name(&self) -> &str;

    /// Processes data using the provided context.
    fn process(&self, context: &mut NodeContext) -> Result<()>;
}
