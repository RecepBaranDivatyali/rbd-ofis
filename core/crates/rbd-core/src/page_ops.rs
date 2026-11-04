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

/// Reorder pages in the PDF document according to `new_order` (1-based page indices).
/// Delegates to `rbd_engine::page_ops::reorder_pages` in compliance with AGENTS.md Rule 2.
pub fn reorder_pages(
    input_path: &Path,
    output_path: &Path,
    new_order: &[u32],
) -> Result<(), CoreError> {
    rbd_engine::page_ops::reorder_pages(input_path, output_path, new_order)
        .map_err(CoreError::Engine)
}

/// Crop the specified 1-based `page_num` to `media_box: [x0, y0, x1, y1]`.
/// Delegates to `rbd_engine::page_ops::crop_page` in compliance with AGENTS.md Rule 2.
pub fn crop_page(
    input_path: &Path,
    output_path: &Path,
    page_num: u32,
    media_box: [f32; 4],
) -> Result<(), CoreError> {
    rbd_engine::page_ops::crop_page(input_path, output_path, page_num, media_box)
        .map_err(CoreError::Engine)
}

/// Insert a blank page with dimensions `width_pt` x `height_pt` after 1-based `after_page`.
/// Delegates to `rbd_engine::page_ops::insert_blank_page` in compliance with AGENTS.md Rule 2.
pub fn insert_blank_page(
    input_path: &Path,
    output_path: &Path,
    after_page: u32,
    width_pt: f32,
    height_pt: f32,
) -> Result<(), CoreError> {
    rbd_engine::page_ops::insert_blank_page(
        input_path,
        output_path,
        after_page,
        width_pt,
        height_pt,
    )
    .map_err(CoreError::Engine)
}

/// Decrypt a password-protected PDF document and write the unencrypted PDF to `output_path`.
/// Delegates to `rbd_engine::page_ops::decrypt_pdf` in compliance with AGENTS.md Rule 2.
pub fn decrypt_pdf(input_path: &Path, output_path: &Path, password: &str) -> Result<(), CoreError> {
    rbd_engine::page_ops::decrypt_pdf(input_path, output_path, password).map_err(CoreError::Engine)
}
