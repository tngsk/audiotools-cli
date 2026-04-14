use anyhow::Result;
use audiotools_core::audio::{load_audio, save_wav};
use audiotools_core::dsp::{spectral_flux, Stft};
use std::path::{Path, PathBuf};

pub struct AudioSegmenter {
    pub segment_len_sec: f32,
    pub top_db: i32,
}

impl AudioSegmenter {
    pub fn new(segment_len_sec: f32, top_db: i32) -> Self {
        Self {
            segment_len_sec,
            top_db,
        }
    }

    pub fn load_audio(&self, path: &Path) -> Result<(Vec<f32>, u32)> {
        load_audio(path)
    }

    pub fn segment_audio(&self, y: &[f32], sr: u32) -> Vec<Vec<f32>> {
        // 1. Trim silence
        let (start, end) = self.trim_silence(y, sr);
        let y_trimmed = &y[start..end];

        if y_trimmed.is_empty() {
            return Vec::new();
        }

        // 2. Detect Onsets
        let onsets = self.detect_onsets(y_trimmed, sr);

        // 3. Segment logic
        let mut segments = Vec::new();
        let segment_samples = (self.segment_len_sec * sr as f32) as usize;

        if !onsets.is_empty() {
            let mut last_end: i64 = -(segment_samples as i64);

            for &start_sample in &onsets {
                if (start_sample as i64) < last_end + (segment_samples as i64 / 2) {
                    continue;
                }
                let end_sample = start_sample + segment_samples;
                if end_sample <= y_trimmed.len() {
                    segments.push(y_trimmed[start_sample..end_sample].to_vec());
                    last_end = end_sample as i64;
                }
            }
        }

        // Fallback or fill gaps? Python logic says:
        // if not segments: ... split regularly
        if segments.is_empty() {
            for chunk in y_trimmed.chunks(segment_samples) {
                if chunk.len() == segment_samples {
                    segments.push(chunk.to_vec());
                }
            }
        }

        segments
    }

    fn trim_silence(&self, y: &[f32], _sr: u32) -> (usize, usize) {
        let frame_len = 2048;
        let hop_len = 512;

        // librosa.effects.trim uses top_db relative to peak.
        // top_db=20 means threshold is Max - 20dB.

        // Find max peak
        let max_val = y.iter().fold(0.0f32, |m, &x| m.max(x.abs()));
        if max_val < 1e-9 {
            return (0, y.len()); // Silence
        }
        let max_db = 20.0 * max_val.log10();
        let threshold = max_db - self.top_db as f32;
        // Convert back to linear for comparison? or db comparison

        // Calculate RMS profile
        let mut valid_indices = Vec::new();
        // Sliding window
        let mut i = 0;
        while i + frame_len <= y.len() {
            let chunk = &y[i..i + frame_len];
            let rms = (chunk.iter().map(|x| x * x).sum::<f32>() / chunk.len() as f32).sqrt();
            let db = 20.0 * rms.max(1e-9).log10();
            if db >= threshold {
                valid_indices.push(i);
            }
            i += hop_len;
        }

        if valid_indices.is_empty() {
            return (0, y.len()); // or empty?
        }

        let start = *valid_indices.first().unwrap();
        let end = *valid_indices.last().unwrap() + frame_len; // include the frame

        (start, end.min(y.len()))
    }

    fn detect_onsets(&self, y: &[f32], sr: u32) -> Vec<usize> {
        let n_fft = 2048;
        let hop_length = 512;

        let stft = Stft::new(n_fft, hop_length);
        let magnitudes = stft.compute_magnitude(y);
        let flux = spectral_flux(&magnitudes);

        // Peak picking
        // Simple heuristic: values > mean + 1.5 * std ?
        let mean_flux: f32 = flux.iter().sum::<f32>() / flux.len() as f32;
        let variance: f32 =
            flux.iter().map(|x| (x - mean_flux).powi(2)).sum::<f32>() / flux.len() as f32;
        let std_flux = variance.sqrt();

        let threshold = mean_flux + 1.0 * std_flux; // Tunable parameter

        let mut onsets = Vec::new();
        let wait_frames = (sr as f32 * 0.2 / hop_length as f32) as usize; // 200ms wait
        let mut last_onset = 0;

        for i in 1..flux.len() - 1 {
            if flux[i] > threshold && flux[i] > flux[i - 1] && flux[i] > flux[i + 1] {
                if i > last_onset + wait_frames {
                    onsets.push(i * hop_length);
                    last_onset = i;
                }
            }
        }

        onsets
    }

    pub fn apply_fade(&self, y: &[f32], sr: u32, fade_sec: f32) -> Vec<f32> {
        let fade_len = (sr as f32 * fade_sec) as usize;
        if y.len() < fade_len * 2 {
            return y.to_vec();
        }

        let mut y_faded = y.to_vec();

        for i in 0..fade_len {
            let gain_in = i as f32 / fade_len as f32;

            y_faded[i] *= gain_in;
            y_faded[y.len() - 1 - i] *= gain_in; // This is fade out using 1 - gain_in logic effectively
                                                 // wait, gain_out for end
                                                 // y[N-1] should go to 0.
                                                 // i=0 -> offset=0 from end -> gain 0.
        }
        y_faded
    }

    pub fn save_segments(
        &self,
        segments: &[Vec<f32>],
        sr: u32,
        original_path: &Path,
        output_dir: &Path,
        overwrite: bool,
    ) -> Result<()> {
        let stem = original_path.file_stem().unwrap().to_string_lossy();
        let ext = ".wav"; // Force wav for now as we use save_wav (hound)

        if !output_dir.exists() {
            std::fs::create_dir_all(output_dir)?;
        }

        for (i, seg) in segments.iter().enumerate() {
            let seg_faded = self.apply_fade(seg, sr, 0.05);
            let filename = format!("{}_{:03}{}", stem, i, ext);
            let output_path = output_dir.join(filename);

            if output_path.exists() && !overwrite {
                if i == 0 {
                    println!("Skipping {}... (File exists)", stem);
                }
                continue;
            }

            save_wav(&output_path, &seg_faded, sr)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trim_silence() {
        let segmenter = AudioSegmenter::new(1.0, 20);
        let mut data = vec![0.0; 10000];
        // Add signal in middle (frame sized)
        // 3000 to 7000
        for i in 3000..7000 {
            data[i] = 0.5;
        }

        // Pass dummy SR, it is unused
        let (start, end) = segmenter.trim_silence(&data, 44100);

        // start should be <= 3000
        // end should be >= 7000
        assert!(start <= 3000, "Start {} should be <= 3000", start);
        assert!(end >= 7000, "End {} should be >= 7000", end);
        assert!(
            start > 0,
            "Start {} should be > 0 (silence at start)",
            start
        );
    }

    #[test]
    fn test_segment_logic_fallback() {
        let segmenter = AudioSegmenter::new(1.0, 20); // 1 sec
        let sr = 100; // small SR
        let len = 350; // 3.5 sec
        let data = vec![0.5; len];

        let segments = segmenter.segment_audio(&data, sr as u32);

        // Expect 3 segments (0-100, 100-200, 200-300)
        assert_eq!(segments.len(), 3);
        assert_eq!(segments[0].len(), 100);
    }
}
