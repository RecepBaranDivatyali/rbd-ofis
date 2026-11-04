pub mod hayro;
pub mod mupdf_status;
pub mod pdfium;

pub use hayro::HayroEngine;
pub use pdfium::PdfiumEngine;

use crate::error::EngineError;
use crate::traits::PdfEngine;

/// PDF rendering engine backend selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineType {
    Pdfium,
    PureRust,
}

/// Create a PDF engine of the specified type.
pub fn create_engine(engine_type: EngineType) -> Result<Box<dyn PdfEngine>, EngineError> {
    match engine_type {
        EngineType::Pdfium => Ok(Box::new(PdfiumEngine::new()?)),
        EngineType::PureRust => Ok(Box::new(HayroEngine::new())),
    }
}

/// Create the default PDF engine.
///
/// Tries PDFium first (fast, full-featured C/C++ engine).
/// If PDFium library cannot be loaded, falls back cleanly to the pure-Rust backend.
pub fn create_default_engine() -> Box<dyn PdfEngine> {
    match PdfiumEngine::new() {
        Ok(engine) => Box::new(engine),
        Err(_) => Box::new(HayroEngine::new()),
    }
}
