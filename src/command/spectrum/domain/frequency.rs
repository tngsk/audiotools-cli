use crate::command::spectrum::error::SpectrumError;

/// Parse frequency annotation string (freq:label)
pub fn parse_frequency_annotation(s: &str) -> Result<(f32, String), SpectrumError> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return Err(SpectrumError::Config(
            "Annotation format should be 'frequency:label'".to_string(),
        ));
    }

    let freq = parts[0]
        .parse::<f32>()
        .map_err(|_| SpectrumError::Config("Invalid frequency value".to_string()))?;

    Ok((freq, parts[1].to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frequency_annotation() {
        let result = parse_frequency_annotation("440:A4");
        assert!(result.is_ok());
        let (freq, label) = result.unwrap();
        assert_eq!(freq, 440.0);
        assert_eq!(label, "A4");

        let result = parse_frequency_annotation("invalid");
        assert!(result.is_err());
    }
}
