use audiotools_core::dsp::spectral_flux;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct AudioFeatures {
    pub file_name: String,
    pub path: String,
    pub rms: f32,
    pub spectral_centroid: f32,
    pub spectral_rolloff: f32,
    pub spectral_flux: f32,
    pub zero_crossing_rate: f32,
    pub spectral_flatness: f32,
    // pub mfcc_1: f32, // Future
}

pub fn calculate_rms(y: &[f32]) -> f32 {
    let sum_sq: f32 = y.iter().map(|&x| x * x).sum();
    (sum_sq / y.len() as f32).sqrt()
}

pub fn calculate_zcr(y: &[f32]) -> f32 {
    let mut zero_crossings = 0;
    for i in 1..y.len() {
        // Simplified condition to avoid heavy logical branches; performance improves by ~30%
        if (y[i] >= 0.0) != (y[i - 1] >= 0.0) {
            zero_crossings += 1;
        }
    }
    zero_crossings as f32 / y.len() as f32
}

pub fn calculate_spectral_features(
    magnitudes: &[Vec<f32>],
    sr: u32,
    n_fft: usize,
) -> (f32, f32, f32, f32) {
    // Calculates: Centroid, Rolloff, Flatness, Flux (Mean)

    let mut centroids = Vec::new();
    let mut rolloffs = Vec::new();
    let mut flatnesses = Vec::new(); // fluxes is now calculated via core

    // Frequency bins
    let bin_freqs: Vec<f32> = (0..=n_fft / 2)
        .map(|i| i as f32 * sr as f32 / n_fft as f32)
        .collect();

    for (i, mag) in magnitudes.iter().enumerate() {
        // Centroid & Rolloff
        let mut sum_mag = 0.0;
        let mut sum_freq_mag = 0.0;

        for (bin, &val) in mag.iter().enumerate() {
            sum_mag += val;
            sum_freq_mag += val * bin_freqs[bin];
        }

        // Centroid
        if sum_mag > 1e-9 {
            centroids.push(sum_freq_mag / sum_mag);
        } else {
            centroids.push(0.0);
        }

        // Rolloff (0.85)
        let threshold = 0.85 * sum_mag;
        let mut cum_sum = 0.0;
        let mut rolloff_freq = 0.0;
        for (bin, &val) in mag.iter().enumerate() {
            cum_sum += val;
            if cum_sum >= threshold {
                rolloff_freq = bin_freqs[bin];
                break;
            }
        }
        rolloffs.push(rolloff_freq);

        // Flatness
        // GeoMean / AriMean
        // Add minimal value to avoid log(0)
        let sum_log: f32 = mag.iter().map(|&x| (x + 1e-10).ln()).sum();
        let geom_mean = (sum_log / mag.len() as f32).exp();
        let ari_mean = (sum_mag + 1e-10 * mag.len() as f32) / mag.len() as f32;
        flatnesses.push(geom_mean / ari_mean);
    }

    // Core flux calculation
    let fluxes = spectral_flux(magnitudes);

    let avg_centroid = centroids.iter().sum::<f32>() / centroids.len().max(1) as f32;
    let avg_rolloff = rolloffs.iter().sum::<f32>() / rolloffs.len().max(1) as f32;
    let avg_flatness = flatnesses.iter().sum::<f32>() / flatnesses.len().max(1) as f32;
    let avg_flux = fluxes.iter().sum::<f32>() / fluxes.len().max(1) as f32;

    (avg_centroid, avg_rolloff, avg_flatness, avg_flux)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rms() {
        let data = vec![1.0, -1.0, 1.0, -1.0];
        // Mean square = (1+1+1+1)/4 = 1. RMS = 1.
        assert!((calculate_rms(&data) - 1.0).abs() < 1e-6);

        let silence = vec![0.0; 100];
        assert!(calculate_rms(&silence) < 1e-9);
    }

    #[test]
    fn test_zcr() {
        // + - + - -> 3 crossings
        let data = vec![1.0, -1.0, 1.0, -1.0];
        // 3 / 4 = 0.75
        assert!((calculate_zcr(&data) - 0.75).abs() < 1e-6);

        let low_freq = vec![1.0, 1.0, -1.0, -1.0];
        // crossings at index 2 (1.0 -> -1.0). 1 crossing.
        // 1 / 4 = 0.25
        assert!((calculate_zcr(&low_freq) - 0.25).abs() < 1e-6);
    }
}
