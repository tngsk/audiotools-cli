use plotters::prelude::RGBColor;

/// Convert power value to color with improved smooth mapping
pub fn power_to_color(power: f32) -> RGBColor {
    let p = power.clamp(0.0, 1.0);

    let (r, g, b) = match p {
        p if p < 0.05 => (0.0, 0.0, p * 640.0), // Black to dark blue
        p if p < 0.2 => {
            // Dark blue to blue
            let t = (p - 0.05) / 0.15;
            (0.0, t * 64.0, 32.0 + t * 96.0)
        }
        p if p < 0.5 => {
            // Blue to cyan
            let t = (p - 0.2) / 0.3;
            (0.0, 64.0 + t * 128.0, 128.0 + t * 127.0)
        }
        p if p < 0.8 => {
            // Cyan to yellow
            let t = (p - 0.5) / 0.3;
            (t * 255.0, 192.0 + t * 63.0, 255.0 * (1.0 - t))
        }
        _ => {
            // Yellow to white
            let t = (p - 0.8) / 0.2;
            (255.0, 255.0, 128.0 + t * 127.0)
        }
    };

    RGBColor(r as u8, g as u8, b as u8)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_to_color() {
        let very_low = power_to_color(0.0);
        let low = power_to_color(0.2);
        let medium = power_to_color(0.5);
        let high = power_to_color(1.0);

        // Test the adjusted color mapping with better distribution
        assert_eq!(very_low, RGBColor(0, 0, 0)); // Black for very low power
        assert_eq!(low, RGBColor(0, 64, 128)); // Blue for low power
        assert_eq!(medium, RGBColor(0, 192, 255)); // Cyan for medium power
        assert_eq!(high, RGBColor(255, 255, 255)); // White for high power
    }
}
