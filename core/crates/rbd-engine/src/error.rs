use thiserror::Error;

/// Structured error type for all PDF engine operations.
#[derive(Debug, Error)]
pub enum EngineError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Document is encrypted / password required: {0}")]
    Encrypted(String),

    #[error("Corrupt or invalid PDF document: {0}")]
    Corrupt(String),

    #[error("Page index {index} out of range (total pages: {total_pages})")]
    PageOutOfRange { index: u32, total_pages: u32 },

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("Buffer too small: provided {provided} bytes, required {required} bytes")]
    BufferTooSmall { provided: usize, required: usize },

    #[error("Rendering error: {0}")]
    RenderError(String),

    #[error("Unsupported PDF feature or format: {0}")]
    Unsupported(String),

    #[error("Internal engine error: {0}")]
    Internal(String),
}

impl From<lopdf::Error> for EngineError {
    fn from(err: lopdf::Error) -> Self {
        EngineError::Corrupt(err.to_string())
    }
}
