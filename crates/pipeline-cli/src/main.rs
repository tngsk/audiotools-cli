use anyhow::{anyhow, Result};
use clap::Parser;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

use audiotools_core::pipeline::{
    io::{AudioInputNode, AudioOutputNode},
    AssetStore, Node, NodeContext,
};

use convert_cli::node::ConvertNode;
use features_cli::node::FeaturesNode;
use info_cli::node::InfoNode;
use loudness_cli::node::LoudnessNode;
use normalize_cli::node::NormalizeNode;
use pca_cli::node::PcaNode;
use segment_cli::node::SegmentNode;
use spectrum_cli::domain::request::SpectrumOptions;
use spectrum_cli::node::SpectrumNode;
use waveform_cli::node::WaveformNode;

#[derive(Parser)]
#[command(author, version, about = "AudioTools Pipeline Orchestrator", long_about = None)]
struct Cli {
    /// Path to the pipeline definition YAML file
    #[arg(short, long, default_value = "pipeline.yaml")]
    config: PathBuf,
}

#[derive(Debug, Deserialize)]
struct PipelineConfig {
    nodes: Vec<NodeConfig>,
}

#[derive(Debug, Deserialize)]
struct NodeConfig {
    id: String,
    #[serde(rename = "type")]
    node_type: String,
    #[serde(default)]
    config: serde_yaml::Value,
    #[serde(default)]
    inputs: HashMap<String, String>,
    #[serde(default)]
    outputs: HashMap<String, String>,
}

fn create_node(config: &NodeConfig) -> Result<Box<dyn Node>> {
    match config.node_type.as_str() {
        "AudioInputNode" => {
            let path = config
                .config
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("AudioInputNode requires 'path' config"))?;
            Ok(Box::new(AudioInputNode {
                filepath: PathBuf::from(path),
            }))
        }
        "AudioOutputNode" => {
            let path = config
                .config
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("AudioOutputNode requires 'path' config"))?;
            Ok(Box::new(AudioOutputNode {
                filepath: PathBuf::from(path),
            }))
        }
        "NormalizeNode" => {
            let level = config
                .config
                .get("level")
                .and_then(|v| v.as_f64())
                .unwrap_or(-1.0) as f32;
            Ok(Box::new(NormalizeNode { level_dbfs: level }))
        }
        "ConvertNode" => {
            let channels = config
                .config
                .get("channels")
                .and_then(|v| v.as_u64())
                .map(|v| v as u16);
            Ok(Box::new(ConvertNode {
                target_channels: channels,
            }))
        }
        "SpectrumNode" => Ok(Box::new(SpectrumNode {
            options: SpectrumOptions::default(),
        })),
        "WaveformNode" => Ok(Box::new(WaveformNode)),
        "SegmentNode" => {
            let segment_len = config
                .config
                .get("segment_len")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0) as f32;
            Ok(Box::new(SegmentNode { segment_len }))
        }
        "FeaturesNode" => Ok(Box::new(FeaturesNode {
            extract_rms: true,
            extract_zcr: true,
        })),
        "PcaNode" => {
            let components = config
                .config
                .get("components")
                .and_then(|v| v.as_u64())
                .unwrap_or(2) as u32;
            Ok(Box::new(PcaNode { components }))
        }
        "InfoNode" => Ok(Box::new(InfoNode)),
        "LoudnessNode" => Ok(Box::new(LoudnessNode)),
        _ => Err(anyhow!("Unknown node type: {}", config.node_type)),
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let config_content = std::fs::read_to_string(&cli.config)
        .map_err(|e| anyhow!("Failed to read pipeline config: {}", e))?;

    let pipeline_config: PipelineConfig = serde_yaml::from_str(&config_content)
        .map_err(|e| anyhow!("Failed to parse pipeline config: {}", e))?;

    let mut store = AssetStore::new();

    println!("Starting pipeline execution...");

    for node_cfg in pipeline_config.nodes {
        println!("Executing node: {} ({})", node_cfg.id, node_cfg.node_type);

        let node = create_node(&node_cfg)?;

        let mut context = NodeContext {
            store: &mut store,
            inputs: &node_cfg.inputs,
            outputs: &node_cfg.outputs,
        };

        node.process(&mut context)?;
    }

    println!("Pipeline execution completed successfully.");
    Ok(())
}
