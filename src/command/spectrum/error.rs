use std::fmt;

#[derive(Debug)]
pub enum SpectrogramError {
    Config(crate::command::spectrum::config::ConfigError),
    FFT(crate::command::spectrum::fft::FFTError),
    IO(std::io::Error),
    Audio(hound::Error),
    InvalidInput(String),
}

impl fmt::Display for SpectrogramError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpectrogramError::Config(err) => write!(f, "Configuration error: {}", err),
            SpectrogramError::FFT(err) => write!(f, "FFT processing error: {}", err),
            SpectrogramError::IO(err) => write!(f, "I/O error: {}", err),
            SpectrogramError::Audio(err) => write!(f, "Audio processing error: {}", err),
            SpectrogramError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
        }
    }
}

impl std::error::Error for SpectrogramError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SpectrogramError::Config(err) => Some(err),
            SpectrogramError::FFT(err) => Some(err),
            SpectrogramError::IO(err) => Some(err),
            SpectrogramError::Audio(err) => Some(err),
            SpectrogramError::InvalidInput(_) => None,
        }
    }
}

impl From<crate::command::spectrum::config::ConfigError> for SpectrogramError {
    fn from(err: crate::command::spectrum::config::ConfigError) -> Self {
        SpectrogramError::Config(err)
    }
}

impl From<crate::command::spectrum::fft::FFTError> for SpectrogramError {
    fn from(err: crate::command::spectrum::fft::FFTError) -> Self {
        SpectrogramError::FFT(err)
    }
}

impl From<std::io::Error> for SpectrogramError {
    fn from(err: std::io::Error) -> Self {
        SpectrogramError::IO(err)
    }
}

impl From<hound::Error> for SpectrogramError {
    fn from(err: hound::Error) -> Self {
        SpectrogramError::Audio(err)
    }
}

pub type Result<T> = std::result::Result<T, SpectrogramError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversion() {
        let config_error =
            crate::command::spectrum::config::ConfigError::InvalidWindowSize("test".to_string());
        let spectrogram_error: SpectrogramError = config_error.into();

        match spectrogram_error {
            SpectrogramError::Config(_) => (),
            _ => panic!("Expected Config error"),
        }
    }
}
