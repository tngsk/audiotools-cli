use hound::WavReader;
use rodio::Decoder;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct AutoStartDetection {
    pub threshold: f32,     // 振幅のスレッショルド値
    pub window_size: usize, // 検出用の移動平均ウィンドウサイズ
    pub min_duration: f32,  // 最小持続時間（秒）
}

impl Default for AutoStartDetection {
    fn default() -> Self {
        Self {
            threshold: 0.01,    // デフォルトのスレッショルド値（-40dB相当）
            window_size: 512,   // デフォルトのウィンドウサイズ
            min_duration: 0.01, // デフォルトの最小持続時間（10ms）
        }
    }
}

impl AutoStartDetection {
    // RMSエネルギーを計算する関数
    fn calculate_rms(window: &[f32]) -> f32 {
        let sum_squares: f32 = window.iter().map(|&x| x * x).sum();
        (sum_squares / window.len() as f32).sqrt()
    }

    // ゼロクロッシングを検出する関数
    fn is_zero_crossing(a: f32, b: f32) -> bool {
        (a < 0.0 && b >= 0.0) || (a >= 0.0 && b < 0.0)
    }

    pub fn detect_start_time(&self, samples: &[f32], sample_rate: f32) -> Option<f32> {
        let min_samples = (self.min_duration * sample_rate) as usize;
        let mut triggered = false;
        let mut potential_start = 0;

        for i in 0..samples.len().saturating_sub(self.window_size) {
            let window = &samples[i..i + self.window_size];
            let rms = Self::calculate_rms(window);

            if !triggered && rms > self.threshold {
                triggered = true;
                potential_start = i;
            } else if triggered {
                if i - potential_start >= min_samples {
                    for j in potential_start..i {
                        if j + 1 < samples.len()
                            && Self::is_zero_crossing(samples[j], samples[j + 1])
                        {
                            return Some(j as f32 / sample_rate);
                        }
                    }
                    return Some(potential_start as f32 / sample_rate);
                }
            }
        }

        None
    }
}

pub fn create_auto_start_config(
    enabled: bool,
    threshold: f32,
    window_size: usize,
    min_duration: f32,
) -> Option<AutoStartDetection> {
    if enabled {
        Some(AutoStartDetection {
            threshold,
            window_size,
            min_duration,
        })
    } else {
        None
    }
}

pub fn detect_peak_level(input: &PathBuf) -> Result<f32, Box<dyn std::error::Error>> {
    let mut max_peak = 0.0f32;

    if let Ok(reader) = WavReader::open(input) {
        // WAVファイルの場合
        let spec = reader.spec();
        match spec.sample_format {
            hound::SampleFormat::Float => {
                for sample in reader.into_samples::<f32>() {
                    if let Ok(sample) = sample {
                        max_peak = max_peak.max(sample.abs());
                    }
                }
            }
            hound::SampleFormat::Int => {
                let bits = spec.bits_per_sample;
                let max_value = (1 << (bits - 1)) as f32;

                for sample in reader.into_samples::<i32>() {
                    if let Ok(sample) = sample {
                        let normalized = sample as f32 / max_value;
                        max_peak = max_peak.max(normalized.abs());
                    }
                }
            }
        }
    } else {
        // WAV以外のフォーマットの場合（mp3, flac等）
        let file = File::open(input)?;
        let reader = BufReader::new(file);
        let decoder = Decoder::new(reader)?;

        // i16サンプルをf32に正規化(-1.0から1.0の範囲に)
        for sample in decoder {
            let normalized = sample as f32 / 32768.0; // i16の最大値で正規化
            max_peak = max_peak.max(normalized.abs());
        }
    }

    // ピーク値をdBFSに変換
    let peak_dbfs = 20.0 * max_peak.max(1e-20).log10();
    Ok(peak_dbfs)
}
