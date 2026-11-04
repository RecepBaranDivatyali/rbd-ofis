pub mod annotation;
pub mod backend;
pub mod error;
pub mod page_ops;
pub mod search;
pub mod traits;

pub use annotation::{
    Annotation, AnnotationKind, list_annotations_from_doc, list_annotations_from_pdf,
    save_annotations_to_memory, save_annotations_to_pdf,
};
pub use backend::{EngineType, HayroEngine, PdfiumEngine, create_default_engine, create_engine};
pub use error::EngineError;
pub use search::{PdfRect, SearchHit, find_matches, turkish_lowercase};
pub use traits::{DocumentMetadata, PdfDocument, PdfEngine, PdfPage, PixelFormat};
