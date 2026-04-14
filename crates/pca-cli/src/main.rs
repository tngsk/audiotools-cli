use anyhow::{Context, Result};
use clap::Parser;
use csv::ReaderBuilder;
use plotters::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;

use plotters::style::full_palette::{GREY, ORANGE, PURPLE};
use smartcore::api::{Transformer, UnsupervisedEstimator};
use smartcore::decomposition::pca::{PCAParameters, PCA};
use smartcore::linalg::basic::arrays::Array;
use smartcore::linalg::basic::matrix::DenseMatrix;
use smartcore::preprocessing::numerical::StandardScaler;

use audiotools_core::config::Config;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    input: PathBuf,

    #[arg(short, long)]
    output: Option<PathBuf>,

    #[arg(short, long, default_value_t = false)]
    overwrite: bool,
}

fn main() -> Result<()> {
    let config = Config::load_default().unwrap_or_default();
    let args = Args::parse();

    let overwrite = args.overwrite
        || config
            .global
            .as_ref()
            .and_then(|g| g.overwrite)
            .unwrap_or(false);

    let input_path = args.input;
    let output_path = args
        .output
        .unwrap_or_else(|| input_path.with_extension("png"));

    if output_path.exists() && !overwrite {
        anyhow::bail!(
            "Output file {:?} already exists. Use --overwrite to replace.",
            output_path
        );
    }

    // 1. Load CSV
    let mut rdr = ReaderBuilder::new()
        .has_headers(true)
        .from_path(&input_path)
        .with_context(|| format!("Failed to open CSV: {:?}", input_path))?;

    let headers = rdr.headers()?.clone();
    let mut data_rows = Vec::new();
    let mut file_names = Vec::new();
    let mut feature_names = Vec::new();

    // Identify numeric columns
    let non_numeric = ["file_name", "path", "segment_id"];

    let mut numeric_indices = Vec::new();
    for (i, h) in headers.iter().enumerate() {
        if !non_numeric.contains(&h) {
            numeric_indices.push(i);
            feature_names.push(h.to_string());
        }
    }

    for result in rdr.records() {
        let record = result?;
        let mut row = Vec::new();

        // Extract file name
        if let Some(idx) = headers.iter().position(|h| h == "file_name") {
            file_names.push(record.get(idx).unwrap_or("?").to_string());
        } else {
            file_names.push("?".to_string());
        }

        // Extract numeric features
        for &idx in &numeric_indices {
            let val_str = record.get(idx).unwrap_or("0");
            let val: f32 = val_str.parse().unwrap_or(0.0);
            row.push(val);
        }
        data_rows.push(row);
    }

    if data_rows.is_empty() {
        println!("No data found.");
        return Ok(());
    }

    // Convert to DenseMatrix
    let n_samples = data_rows.len();
    let n_features = data_rows[0].len();
    let flattened: Vec<f32> = data_rows.into_iter().flatten().collect();

    // Smartcore DenseMatrix is row-major by default? Yes.
    // Explicitly using new(nrows, ncols, values, column_major=false)
    let matrix = DenseMatrix::new(n_samples, n_features, flattened, false)
        .map_err(|e| anyhow::anyhow!("Failed to create matrix: {:?}", e))?;

    // 2. Scale
    let scaled_matrix = StandardScaler::fit(&matrix, Default::default())
        .and_then(|s| s.transform(&matrix))
        .map_err(|e| anyhow::anyhow!("Scaler error: {:?}", e))?;

    // 3. PCA
    // n_components = 2
    let pca = PCA::fit(
        &scaled_matrix,
        PCAParameters::default().with_n_components(2),
    )
    .map_err(|e| anyhow::anyhow!("PCA fit error: {}", e))?;

    let transformed: DenseMatrix<f32> = pca
        .transform(&scaled_matrix)
        .map_err(|e| anyhow::anyhow!("PCA transform error: {}", e))?;

    // Explained variance (not checking here as easy as linfa, but components are available)
    // components() returns (n_components, n_features) matrix.
    let components: &DenseMatrix<f32> = pca.components();

    println!("\n--- PCA Loadings (Top contributors) ---");
    // components is n_comp x n_feat
    for i in 0..2 {
        println!("\nPC{} top contributors:", i + 1);
        // Get row i
        // Smartcore DenseMatrix access: .get(row, col)
        let mut row_vals: Vec<(usize, f32)> = (0..n_features)
            .map(|j| (j, *components.get((i, j))))
            .collect();

        // Sort by abs value
        row_vals.sort_by(|a, b| b.1.abs().partial_cmp(&a.1.abs()).unwrap());

        for (idx, val) in row_vals.iter().take(5) {
            println!("  - {}: {:.3}", feature_names[*idx], val);
        }
    }

    // 4. Plot
    let root = BitMapBackend::new(&output_path, (1200, 1000)).into_drawing_area();
    root.fill(&WHITE)?;

    // Find ranges for Plotters
    // transformed is n_samples x 2
    let mut x_min = f32::INFINITY;
    let mut x_max = f32::NEG_INFINITY;
    let mut y_min = f32::INFINITY;
    let mut y_max = f32::NEG_INFINITY;

    for i in 0..n_samples {
        let x = *transformed.get((i, 0));
        let y = *transformed.get((i, 1));
        x_min = x_min.min(x);
        x_max = x_max.max(x);
        y_min = y_min.min(y);
        y_max = y_max.max(y);
    }

    let x_range = (x_min - 1.0)..(x_max + 1.0);
    let y_range = (y_min - 1.0)..(y_max + 1.0);

    let mut chart = ChartBuilder::on(&root)
        .caption("Audio Feature Space (PCA)", ("sans-serif", 30).into_font())
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(40)
        .build_cartesian_2d(x_range, y_range)?;

    chart.configure_mesh().x_desc("PC1").y_desc("PC2").draw()?;

    // Assign colors to unique files
    let mut unique_files = file_names.clone();
    unique_files.sort();
    unique_files.dedup();

    let colors = [RED, BLUE, GREEN, ORANGE, PURPLE, CYAN, MAGENTA];
    let mut color_map = HashMap::new();
    for (i, name) in unique_files.iter().enumerate() {
        color_map.insert(name, colors[i % colors.len()]);
    }

    for i in 0..n_samples {
        let x = *transformed.get((i, 0));
        let y = *transformed.get((i, 1));
        let name = &file_names[i];
        let color = color_map.get(name).unwrap();

        chart.draw_series(PointSeries::of_element(
            vec![(x, y)],
            5,
            color.filled(),
            &|coord, size, style| EmptyElement::at(coord) + Circle::new((0, 0), size, style),
        ))?;

        // Label
        chart.draw_series(std::iter::once(Text::new(
            name.clone(),
            (x, y + 0.1),
            ("sans-serif", 10).into_font(),
        )))?;
    }

    // Draw arrows (Loadings)
    // Scale vectors to fit plot
    let scale = (x_max - x_min).max(y_max - y_min) * 0.4;

    for j in 0..n_features {
        let x = *components.get((0, j)) * scale;
        let y = *components.get((1, j)) * scale;

        // Line
        chart.draw_series(LineSeries::new(vec![(0.0, 0.0), (x, y)], &GREY.mix(0.5)))?;

        chart.draw_series(std::iter::once(Text::new(
            feature_names[j].clone(),
            (x * 1.1, y * 1.1),
            ("sans-serif", 10).into_font().color(&GREY),
        )))?;
    }

    root.present()?;
    println!("Saved plot to {:?}", output_path);

    Ok(())
}
