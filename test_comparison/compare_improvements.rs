//! Comparison test to demonstrate spectrum improvements

use std::path::PathBuf;

/// Old color mapping function for comparison
fn old_power_to_color(normalized_power: f32) -> (u8, u8, u8) {
    let power = normalized_power.max(0.0).min(1.0);

    // Old simple heat map: black -> red -> yellow -> white
    if power < 0.33 {
        let ratio = power * 3.0;
        ((255.0 * ratio) as u8, 0, 0)
    } else if power < 0.66 {
        let ratio = (power - 0.33) * 3.0;
        (255, (255.0 * ratio) as u8, 0)
    } else {
        let ratio = (power - 0.66) * 3.0;
        (255, 255, (255.0 * ratio) as u8)
    }
}

/// New improved color mapping function
fn new_power_to_color(normalized_power: f32) -> (u8, u8, u8) {
    let power = normalized_power.max(0.0).min(1.0);

    // Improved smooth heat map with better contrast
    if power < 0.1 {
        // Very low power - dark blue to black
        let ratio = power * 10.0;
        (0, 0, (64.0 * ratio) as u8)
    } else if power < 0.4 {
        // Low power - blue to cyan
        let ratio = (power - 0.1) / 0.3;
        (0, (128.0 * ratio) as u8, 64 + (191.0 * ratio) as u8)
    } else if power < 0.7 {
        // Medium power - cyan to yellow
        let ratio = (power - 0.4) / 0.3;
        (
            (255.0 * ratio) as u8,
            128 + (127.0 * ratio) as u8,
            (255.0 * (1.0 - ratio)) as u8,
        )
    } else {
        // High power - yellow to white
        let ratio = (power - 0.7) / 0.3;
        (255, 255, (255.0 * ratio) as u8)
    }
}

/// Old hop size calculation
fn old_hop_size(window_size: usize, overlap: f32) -> usize {
    ((window_size as f32 * (1.0 - overlap)) as usize).max(1)
}

/// New fixed hop size calculation
fn new_hop_size(window_size: usize, _overlap: f32) -> usize {
    (window_size / 8).max(1)
}

fn main() {
    println!("Spectrum Improvement Comparison");
    println!("==============================");

    // Compare hop size calculations
    let window_size = 2048;
    let overlap = 0.75;

    let old_hop = old_hop_size(window_size, overlap);
    let new_hop = new_hop_size(window_size, overlap);

    println!("\nHop Size Comparison:");
    println!("Window size: {}", window_size);
    println!(
        "Old overlap-based hop size: {} ({}% overlap)",
        old_hop,
        (1.0 - old_hop as f32 / window_size as f32) * 100.0
    );
    println!(
        "New fixed hop size: {} ({}% overlap)",
        new_hop,
        (1.0 - new_hop as f32 / window_size as f32) * 100.0
    );
    println!(
        "Improvement: {}x higher time resolution",
        old_hop as f32 / new_hop as f32
    );

    // Compare color mappings
    println!("\nColor Mapping Comparison:");
    println!("Power Level | Old Color (R,G,B) | New Color (R,G,B)");
    println!("------------|-------------------|-------------------");

    let test_powers = [0.0, 0.1, 0.2, 0.33, 0.4, 0.5, 0.66, 0.7, 0.8, 0.9, 1.0];

    for &power in &test_powers {
        let old_color = old_power_to_color(power);
        let new_color = new_power_to_color(power);
        println!(
            "{:10.1} | {:>17} | {:>17}",
            power * 100.0,
            format!("({:3},{:3},{:3})", old_color.0, old_color.1, old_color.2),
            format!("({:3},{:3},{:3})", new_color.0, new_color.1, new_color.2)
        );
    }

    // Analysis of improvements
    println!("\nKey Improvements:");
    println!(
        "1. Time Resolution: {}x higher with fixed hop_size",
        old_hop as f32 / new_hop as f32
    );
    println!("2. Color Mapping: More gradual transitions with 4 distinct phases");
    println!("3. Low Power Detail: Better visibility of quiet signals");
    println!("4. Visual Continuity: Smoother appearance with overlapping rectangles");

    // Frequency processing improvements
    println!("\nFrequency Processing Improvements:");
    println!("- Process only frequency bins within specified range");
    println!("- Window compensation factor for better magnitude scaling");
    println!("- Partial frame processing for complete audio coverage");
    println!("- Lower power threshold (0.005 vs 0.01) for more detail");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hop_size_improvement() {
        let window_size = 2048;
        let overlap = 0.75;

        let old_hop = old_hop_size(window_size, overlap);
        let new_hop = new_hop_size(window_size, overlap);

        // New hop size should provide higher resolution
        assert!(new_hop < old_hop);
        assert_eq!(new_hop, 256); // window_size / 8
        assert_eq!(old_hop, 512); // 25% of window_size
    }

    #[test]
    fn test_color_mapping_improvements() {
        // Test smooth transitions
        let colors: Vec<_> = (0..=10)
            .map(|i| {
                let power = i as f32 / 10.0;
                new_power_to_color(power)
            })
            .collect();

        // Should have smooth transitions without sudden jumps
        for i in 1..colors.len() {
            let prev = colors[i - 1];
            let curr = colors[i];

            // Check that color changes are gradual
            let r_diff = (curr.0 as i16 - prev.0 as i16).abs();
            let g_diff = (curr.1 as i16 - prev.1 as i16).abs();
            let b_diff = (curr.2 as i16 - prev.2 as i16).abs();

            // No channel should change by more than 100 points between adjacent samples
            assert!(r_diff <= 100, "Red channel jump too large: {}", r_diff);
            assert!(g_diff <= 100, "Green channel jump too large: {}", g_diff);
            assert!(b_diff <= 100, "Blue channel jump too large: {}", b_diff);
        }
    }

    #[test]
    fn test_low_power_visibility() {
        // New color mapping should provide better visibility for low power signals
        let low_power_old = old_power_to_color(0.1);
        let low_power_new = new_power_to_color(0.1);

        // New mapping should have more color information for low power
        let old_brightness =
            low_power_old.0 as u16 + low_power_old.1 as u16 + low_power_old.2 as u16;
        let new_brightness =
            low_power_new.0 as u16 + low_power_new.1 as u16 + low_power_new.2 as u16;

        assert!(
            new_brightness > old_brightness,
            "New color mapping should be more visible for low power signals"
        );
    }
}
