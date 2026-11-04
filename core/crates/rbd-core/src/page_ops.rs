use crate::error::CoreError;
use std::path::Path;

/// Rotate all pages in the PDF document by the specified angle (in degrees, must be a multiple of 90).
/// Delegates to `rbd_engine::page_ops::rotate` in compliance with AGENTS.md Rule 2.
pub fn rotate(input_path: &Path, degrees: i32, output_path: &Path) -> Result<(), CoreError> {
    if degrees % 90 != 0 {
        return Err(CoreError::InvalidRotation(degrees));
    }
    rbd_engine::page_ops::rotate(input_path, degrees, output_path).map_err(CoreError::Engine)
}

/// Delete the specified 1-based page numbers from the PDF and write the result to `output_path`.
/// Delegates to `rbd_engine::page_ops::delete_pages` in compliance with AGENTS.md Rule 2.
pub fn delete_pages(
    input_path: &Path,
    pages_to_delete: &[u32],
    output_path: &Path,
) -> Result<(), CoreError> {
    rbd_engine::page_ops::delete_pages(input_path, pages_to_delete, output_path)
        .map_err(CoreError::Engine)
}

/// Split the document by extracting 1-based page range `[start_page, end_page]` (inclusive).
/// Delegates to `rbd_engine::page_ops::split` in compliance with AGENTS.md Rule 2.
pub fn split(
    input_path: &Path,
    start_page: u32,
    end_page: u32,
    output_path: &Path,
) -> Result<(), CoreError> {
    if start_page < 1 || start_page > end_page {
        return Err(CoreError::InvalidPageRange {
            start: start_page,
            end: end_page,
        });
    }
    rbd_engine::page_ops::split(input_path, start_page, end_page, output_path)
        .map_err(CoreError::Engine)
}

/// Merge multiple input PDF files into a single output PDF document.
/// Delegates to `rbd_engine::page_ops::merge` in compliance with AGENTS.md Rule 2.
pub fn merge(input_paths: &[&Path], output_path: &Path) -> Result<(), CoreError> {
    if input_paths.is_empty() {
        return Err(CoreError::Internal(
            "No input PDF paths provided for merge".into(),
        ));
    }
    rbd_engine::page_ops::merge(input_paths, output_path).map_err(CoreError::Engine)
}
