use rbd_engine::EngineError;
use thiserror::Error;

/// Core library error types.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Engine error: {0}")]
    Engine(#[from] EngineError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid page index: {0}")]
    InvalidPage(u32),

    #[error("Invalid page range: start={start}, end={end}")]
    InvalidPageRange { start: u32, end: u32 },

    #[error("Invalid rotation angle: {0} degrees (must be a multiple of 90)")]
    InvalidRotation(i32),

    #[error("Buffer too small: provided {provided} bytes, required {required} bytes")]
    BufferTooSmall { provided: usize, required: usize },

    #[error("Command execution error: {0}")]
    CommandError(String),

    #[error("Internal core error: {0}")]
    Internal(String),
}
