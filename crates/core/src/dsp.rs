use rustfft::{FftPlanner, num_complex::Complex};
use std::sync::Arc;

pub struct Stft {
    pub fft: Arc<dyn rustfft::Fft<f32>>,
    pub window: Vec<f32>,
    pub hop_size: usize,
    pub frame_size: usize,
}

impl Stft {
    pub fn new(frame_size: usize, hop_size: usize) -> Self {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(frame_size);
        
        // Hann window
        let window: Vec<f32> = (0..frame_size)
            .map(|i| 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (frame_size as f32 - 1.0)).cos()))
            .collect();

        Self {
            fft,
            window,
            hop_size,
            frame_size,
        }
    }

    pub fn compute_magnitude(&self, signal: &[f32]) -> Vec<Vec<f32>> {
        let mut magnitudes = Vec::new();
        let mut i = 0;
        
        while i + self.frame_size <= signal.len() {
            let mut buffer: Vec<Complex<f32>> = signal[i..i+self.frame_size]
                .iter()
                .zip(&self.window)
                .map(|(&x, &w)| Complex::new(x * w, 0.0))
                .collect();
            
            self.fft.process(&mut buffer);

            let mag: Vec<f32> = buffer.iter()
                .take(self.frame_size / 2 + 1)
                .map(|c| c.norm())
                .collect();
            
            magnitudes.push(mag);
            i += self.hop_size;
        }
        magnitudes
    }
}

pub fn spectral_flux(magnitudes: &[Vec<f32>]) -> Vec<f32> {
    let mut flux = Vec::with_capacity(magnitudes.len());
    flux.push(0.0);

    for i in 1..magnitudes.len() {
        let prev = &magnitudes[i-1];
        let curr = &magnitudes[i];
        
        let mut sum = 0.0;
        for (c, p) in curr.iter().zip(prev.iter()) {
            let diff = c - p;
            if diff > 0.0 {
                sum += diff;
            }
        }
        flux.push(sum);
    }
    flux
}
