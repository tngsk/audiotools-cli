use crate::command::spectrum::core::config::{FrequencyPreset, DurationPreset};

pub fn frequency_preset(preset: FrequencyPreset, sample_rate: f32) -> (f32, f32) {
    let nyquist = sample_rate / 2.0;
    match preset {
        FrequencyPreset::Full => (20.0, nyquist),
        FrequencyPreset::AudioRange => (20.0, 20000.0_f32.min(nyquist)),
        FrequencyPreset::SpeechRange => (80.0, 8000.0_f32.min(nyquist)),
        FrequencyPreset::MusicRange => (80.0, 12000.0_f32.min(nyquist)),
        FrequencyPreset::Bass => (60.0, 250.0_f32.min(nyquist)),
    }
}

pub fn get_duration_preset(duration_ms: f32) -> DurationPreset {
    match duration_ms {
        d if d < 150.0 => DurationPreset::VeryShort,
        d if d < 500.0 => DurationPreset::Short,
        d if d < 2000.0 => DurationPreset::Medium,
        _ => DurationPreset::Long,
    }
}
