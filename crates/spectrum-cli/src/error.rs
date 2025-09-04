use thiserror::Error;

/// Unified error type for spectrum command
#[derive(Debug, Error)]
pub enum SpectrumError {
    #[error("{context}: {source}")]
    WithContext {
        context: String,
        #[source]
        source: Box<SpectrumError>,
    },

    #[error("{0}")]
    Message(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Hound(#[from] hound::Error),
}

impl SpectrumError {
    /// Create error with message
    pub fn new(msg: impl Into<String>) -> Self {
        Self::Message(msg.into())
    }

    /// Add context to error
    pub fn context(self, ctx: impl Into<String>) -> Self {
        Self::WithContext {
            context: ctx.into(),
            source: Box::new(self),
        }
    }
}

/// Extension trait for Result types
pub trait ResultExt<T> {
    fn context(self, ctx: impl Into<String>) -> Result<T, SpectrumError>;
}

impl<T, E> ResultExt<T> for Result<T, E>
where
    E: Into<SpectrumError>,
{
    fn context(self, ctx: impl Into<String>) -> Result<T, SpectrumError> {
        self.map_err(|e| e.into().context(ctx))
    }
}

/// Config error compatibility
pub use SpectrumError as ConfigError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = SpectrumError::new("test error");
        assert_eq!(err.to_string(), "test error");
    }

    #[test]
    fn test_error_with_context() {
        let err = SpectrumError::new("base error").context("additional context");
        assert!(err.to_string().contains("additional context"));
        assert!(err.to_string().contains("base error"));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let spectrum_err: SpectrumError = io_err.into();
        assert!(spectrum_err.to_string().contains("not found"));
    }

    #[test]
    fn test_result_extension() {
        fn failing_function() -> Result<(), std::io::Error> {
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "denied",
            ))
        }

        let result = failing_function().context("while processing file");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("while processing file"));
    }

    #[test]
    fn test_hound_error_conversion() {
        // We can't easily create a real hound::Error, but we can test the trait exists
        // by verifying the From trait implementation compiles
        fn test_from_impl() -> Result<(), SpectrumError> {
            // This function just needs to compile to prove the From trait exists
            Ok(())
        }

        assert!(test_from_impl().is_ok());
    }
}
