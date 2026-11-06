pub mod ai;
pub mod annotation;
pub mod backend;
pub mod compression;
pub mod converter;
pub mod error;
pub mod forms;
pub mod ooxml;
pub mod page_ops;
pub mod redaction;
pub mod reflow;
pub mod search;
pub mod stamp;
pub mod traits;

pub use ai::{
    DocumentAnswer, DocumentSummary, KeywordScore, ask_document, split_sentences,
    summarize_document, tokenize_words,
};
pub use annotation::{
    Annotation, AnnotationKind, list_annotations_from_doc, list_annotations_from_pdf,
    save_annotations_to_memory, save_annotations_to_pdf,
};
pub use backend::{EngineType, HayroEngine, PdfiumEngine, create_default_engine, create_engine};
pub use compression::{
    CompressionOptions, CompressionResult, compress_document, compress_document_bytes,
};
pub use converter::{
    ImageFormatType, ImageToPdfOptions, PageSize, PdfToImageOptions, images_to_pdf,
    markdown_to_pdf, pdf_to_images,
};
pub use error::EngineError;
pub use forms::{
    FormFieldInfo, FormFieldType, fill_form_fields, fill_form_fields_bytes, read_form_fields,
    read_form_fields_bytes,
};
pub use ooxml::{
    OfficeDocType, OfficeDocumentInfo, create_simple_docx, create_simple_docx_bytes,
    extract_office_text, extract_office_text_from_bytes, inspect_office_document,
};
pub use page_ops::{
    crop_page, delete_pages, insert_blank_page, merge, reorder_pages, rotate, split,
};
pub use redaction::{PageRedaction, redact_document, redact_document_bytes};
pub use reflow::{
    BoundingBox, ReflowParagraph, TextLineSegment, cluster_lines_into_paragraphs,
    reflow_text_to_width, replace_paragraph_text, replace_paragraph_text_doc,
};
pub use search::{PdfRect, SearchHit, find_matches, turkish_lowercase};
pub use stamp::{
    BatesOptions, PageNumberOptions, StampPosition, WatermarkOptions, apply_bates_stamp,
    apply_bates_stamp_bytes, apply_page_numbers, apply_page_numbers_bytes, apply_watermark,
    apply_watermark_bytes,
};
pub use traits::{DocumentMetadata, PdfDocument, PdfEngine, PdfPage, PixelFormat};
