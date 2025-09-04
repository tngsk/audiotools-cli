#[derive(Debug, Clone)]
pub struct AudioData {
    pub samples: Vec<f32>,
    pub sample_rate: f32,
    pub duration: f32,
    pub channels: u32,
}
