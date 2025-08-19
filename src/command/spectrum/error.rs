use thiserror::Error;

#[derive(Debug, Error)]
pub enum SpectrumError {
    #[error("Audio loading failed: {0}")]
    AudioLoad(String),
    
    #[error("Analysis failed: {0}")]
    Analysis(String),
    
    #[error("Rendering failed: {0}")]
    Render(String),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Time range processing failed: {0}")]
    TimeRange(String),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<hound::Error> for SpectrumError {
    fn from(err: hound::Error) -> Self {
        SpectrumError::AudioLoad(err.to_string())
    }
}

pub trait ErrorContext<T> {
    fn with_context<F>(self, _f: F) -> Result<T, SpectrumError>
    where
        F: FnOnce() -> String;
}

impl<T, E> ErrorContext<T> for Result<T, E>
where
    E: Into<SpectrumError>,
{
    fn with_context<F>(self, _f: F) -> Result<T, SpectrumError>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| {
            let error = e.into();
            // エラーコンテキストを追加
            error
        })
    }
}