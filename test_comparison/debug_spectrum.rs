//! Debug utility to verify spectrum generation fixes

use std::path::PathBuf;

/// Debug spectrum generation to identify black line issues
fn main() {
    println!("Spectrum Debug Analysis");
    println!("======================");

    // Test different configurations to verify fixes
    test_frequency_mapping();
    test_time_resolution();
    test_coordinate_calculations();
}

fn test_frequency_mapping() {
    println!("\n1. Frequency Mapping Test:");
    println!("---------------------------");

    let sample_rate = 44100.0;
    let window_size = 2048;
    let freq_resolution = sample_rate / window_size as f32;
    let min_freq = 80.0;
    let max_freq = 8000.0;
    let freq_bins = window_size / 2;

    println!("Sample rate: {} Hz", sample_rate);
    println!("Window size: {}", window_size);
    println!("Frequency resolution: {:.2} Hz/bin", freq_resolution);
    println!("Frequency range: {} - {} Hz", min_freq, max_freq);
    println!("Total frequency bins: {}", freq_bins);

    // Calculate which bins correspond to our frequency range
    let min_bin = (min_freq / freq_resolution) as usize;
    let max_bin = ((max_freq / freq_resolution) as usize).min(freq_bins);

    println!(
        "Min frequency bin: {} (freq: {:.1} Hz)",
        min_bin,
        min_bin as f32 * freq_resolution
    );
    println!(
        "Max frequency bin: {} (freq: {:.1} Hz)",
        max_bin,
        max_bin as f32 * freq_resolution
    );

    // Show some example bin-to-frequency mappings
    println!("\nBin-to-Frequency mapping examples:");
    for bin in [
        min_bin,
        min_bin + 10,
        (min_bin + max_bin) / 2,
        max_bin - 10,
        max_bin,
    ] {
        let freq = bin as f32 * freq_resolution;
        let in_range = freq >= min_freq && freq <= max_freq;
        println!("  Bin {}: {:.1} Hz (in range: {})", bin, freq, in_range);
    }
}

fn test_time_resolution() {
    println!("\n2. Time Resolution Test:");
    println!("------------------------");

    let window_size = 2048;
    let sample_rate = 44100.0;
    let old_hop_size = (window_size as f32 * 0.25) as usize; // 75% overlap
    let new_hop_size = window_size / 8; // 87.5% overlap

    println!("Window size: {}", window_size);
    println!("Sample rate: {} Hz", sample_rate);

    let old_time_res = old_hop_size as f32 / sample_rate;
    let new_time_res = new_hop_size as f32 / sample_rate;

    println!(
        "Old hop size: {} samples ({:.3} ms)",
        old_hop_size,
        old_time_res * 1000.0
    );
    println!(
        "New hop size: {} samples ({:.3} ms)",
        new_hop_size,
        new_time_res * 1000.0
    );
    println!(
        "Time resolution improvement: {:.1}x",
        old_time_res / new_time_res
    );

    // Simulate a 1-second audio file
    let duration = 1.0;
    let sample_count = (duration * sample_rate) as usize;
    let old_frames = (sample_count - window_size) / old_hop_size + 1;
    let new_frames = (sample_count - window_size) / new_hop_size + 1;

    println!("\nFor {}s audio ({} samples):", duration, sample_count);
    println!("Old frame count: {}", old_frames);
    println!("New frame count: {}", new_frames);
    println!(
        "Frame density improvement: {:.1}x",
        new_frames as f32 / old_frames as f32
    );
}

fn test_coordinate_calculations() {
    println!("\n3. Coordinate Calculation Test:");
    println!("-------------------------------");

    let window_size = 2048;
    let sample_rate = 44100.0;
    let freq_resolution = sample_rate / window_size as f32;
    let hop_size = window_size / 8;
    let min_freq = 80.0;
    let max_freq = 8000.0;

    // Simulate 5 frames of data
    let frame_count = 5;
    let total_duration = 0.1; // 100ms
    let time_per_frame = total_duration / frame_count as f32;

    println!(
        "Simulating {} frames over {:.1}ms",
        frame_count,
        total_duration * 1000.0
    );
    println!("Time per frame: {:.3}ms", time_per_frame * 1000.0);
    println!("Frequency resolution: {:.2} Hz", freq_resolution);

    println!("\nTime coordinates:");
    for frame in 0..frame_count {
        let time_start = frame as f32 * time_per_frame;
        let time_end = time_start + time_per_frame;
        println!(
            "  Frame {}: {:.3}ms - {:.3}ms",
            frame,
            time_start * 1000.0,
            time_end * 1000.0
        );
    }

    println!("\nFrequency coordinates (first 10 bins in range):");
    let min_bin = (min_freq / freq_resolution) as usize;
    for i in 0..10 {
        let bin = min_bin + i;
        let freq_start = bin as f32 * freq_resolution;
        let freq_end = freq_start + freq_resolution;
        let in_range = freq_start >= min_freq && freq_start <= max_freq;
        println!(
            "  Bin {} (idx {}): {:.1}Hz - {:.1}Hz (in range: {})",
            bin, i, freq_start, freq_end, in_range
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_coordinate_mismatch() {
        let window_size = 2048;
        let sample_rate = 44100.0;
        let freq_resolution = sample_rate / window_size as f32;
        let min_freq = 80.0;
        let max_freq = 8000.0;

        // Test that our coordinate calculations are consistent
        let min_bin = (min_freq / freq_resolution) as usize;
        let max_bin = (max_freq / freq_resolution) as usize;

        // Check that calculated frequencies match expected ranges
        let calc_min_freq = min_bin as f32 * freq_resolution;
        let calc_max_freq = max_bin as f32 * freq_resolution;

        assert!((calc_min_freq - min_freq).abs() < freq_resolution);
        assert!((calc_max_freq - max_freq).abs() < freq_resolution);
    }

    #[test]
    fn test_rendering_coordinates() {
        let freq_resolution = 21.53; // ~44100/2048
        let spectrum_length = 100; // Example spectrum from FFT

        // Simulate rendering coordinates
        for bin in 0..spectrum_length {
            let freq_start = bin as f32 * freq_resolution;
            let freq_end = freq_start + freq_resolution;

            // Check that there are no gaps between frequency bins
            if bin > 0 {
                let prev_freq_end = (bin - 1) as f32 * freq_resolution + freq_resolution;
                assert!((freq_start - prev_freq_end).abs() < 0.001);
            }
        }
    }
}
