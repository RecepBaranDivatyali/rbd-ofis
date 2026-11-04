use crate::error::EngineError;
use lopdf::{Dictionary, Document, Object, ObjectId};
use std::collections::BTreeMap;
use std::path::Path;

/// Rotate all pages in the PDF document by the specified angle (in degrees, must be a multiple of 90).
pub fn rotate(input_path: &Path, degrees: i32, output_path: &Path) -> Result<(), EngineError> {
    if degrees % 90 != 0 {
        return Err(EngineError::InvalidParameter(format!(
            "Invalid rotation angle: {degrees} (must be a multiple of 90)"
        )));
    }

    let mut doc = Document::load(input_path)?;
    let pages = doc.get_pages();

    for page_id in pages.values() {
        if let Ok(page_dict) = doc.get_dictionary_mut(*page_id) {
            let current = page_dict
                .get(b"Rotate")
                .and_then(|o| o.as_i64())
                .unwrap_or(0);
            let new_rot = (current + degrees as i64).rem_euclid(360);
            page_dict.set("Rotate", Object::Integer(new_rot));
        }
    }

    doc.save(output_path)?;
    Ok(())
}

/// Delete the specified 1-based page numbers from the PDF and write the result to `output_path`.
pub fn delete_pages(
    input_path: &Path,
    pages_to_delete: &[u32],
    output_path: &Path,
) -> Result<(), EngineError> {
    let mut doc = Document::load(input_path)?;
    doc.delete_pages(pages_to_delete);
    doc.prune_objects();
    doc.save(output_path)?;
    Ok(())
}

/// Split the document by extracting 1-based page range `[start_page, end_page]` (inclusive).
pub fn split(
    input_path: &Path,
    start_page: u32,
    end_page: u32,
    output_path: &Path,
) -> Result<(), EngineError> {
    if start_page < 1 || start_page > end_page {
        return Err(EngineError::InvalidParameter(format!(
            "Invalid page range: start={start_page}, end={end_page}"
        )));
    }

    let mut doc = Document::load(input_path)?;
    let total_pages = doc.get_pages().len() as u32;

    if start_page > total_pages {
        return Err(EngineError::InvalidParameter(format!(
            "Start page {start_page} exceeds total pages {total_pages}"
        )));
    }

    let effective_end = end_page.min(total_pages);
    let to_delete: Vec<u32> = (1..=total_pages)
        .filter(|&p| p < start_page || p > effective_end)
        .collect();

    doc.delete_pages(&to_delete);
    doc.prune_objects();
    doc.save(output_path)?;
    Ok(())
}

/// Merge multiple input PDF files into a single output PDF document.
pub fn merge(input_paths: &[&Path], output_path: &Path) -> Result<(), EngineError> {
    if input_paths.is_empty() {
        return Err(EngineError::InvalidParameter(
            "No input PDF paths provided for merge".into(),
        ));
    }

    if input_paths.len() == 1 {
        let mut doc = Document::load(input_paths[0])?;
        doc.save(output_path)?;
        return Ok(());
    }

    let mut documents = Vec::new();
    for path in input_paths {
        let doc = Document::load(path)?;
        documents.push(doc);
    }

    let mut merged = Document::with_version("1.5");
    let mut max_id = 1;
    let mut documents_pages = BTreeMap::new();
    let mut documents_objects = BTreeMap::new();

    for mut doc in documents {
        doc.renumber_objects_with(max_id);
        max_id = doc.max_id + 1;

        let pages = doc.get_pages();
        for object_id in pages.values() {
            if let Ok(obj) = doc.get_object(*object_id) {
                documents_pages.insert(*object_id, obj.clone());
            }
        }

        documents_objects.extend(doc.objects);
    }

    let mut catalog_object: Option<(ObjectId, Object)> = None;
    let mut pages_object: Option<(ObjectId, Object)> = None;

    for (object_id, object) in documents_objects.into_iter() {
        match object.type_name().unwrap_or(b"") {
            b"Catalog" => {
                if catalog_object.is_none() {
                    catalog_object = Some((object_id, object));
                }
            }
            b"Pages" => {
                if pages_object.is_none() {
                    pages_object = Some((object_id, object));
                }
            }
            b"Page" | b"Outlines" | b"Outline" => {}
            _ => {
                merged.objects.insert(object_id, object);
            }
        }
    }

    let (pages_id, pages_obj) =
        pages_object.ok_or_else(|| EngineError::Corrupt("Pages root not found".into()))?;
    let (catalog_id, catalog_obj) =
        catalog_object.ok_or_else(|| EngineError::Corrupt("Catalog root not found".into()))?;

    // Link all pages to the unified Pages root object
    for (object_id, object) in documents_pages.iter() {
        if let Ok(mut dictionary) = object.as_dict().cloned() {
            dictionary.set("Parent", pages_id);
            merged
                .objects
                .insert(*object_id, Object::Dictionary(dictionary));
        }
    }

    // Build the merged Pages dictionary
    let mut pages_dict = pages_obj
        .as_dict()
        .cloned()
        .unwrap_or_else(|_| Dictionary::new());
    pages_dict.remove(b"Parent");
    pages_dict.set("Count", documents_pages.len() as u32);
    pages_dict.set(
        "Kids",
        documents_pages
            .into_keys()
            .map(Object::Reference)
            .collect::<Vec<_>>(),
    );
    merged
        .objects
        .insert(pages_id, Object::Dictionary(pages_dict));

    // Build the merged Catalog dictionary
    let mut catalog_dict = catalog_obj
        .as_dict()
        .cloned()
        .unwrap_or_else(|_| Dictionary::new());
    catalog_dict.set("Pages", pages_id);
    catalog_dict.remove(b"Outlines");
    merged
        .objects
        .insert(catalog_id, Object::Dictionary(catalog_dict));

    merged.trailer.set("Root", catalog_id);
    merged.max_id = merged.objects.len() as u32;
    merged.renumber_objects();

    merged.save(output_path)?;
    Ok(())
}

/// Reorder pages in the PDF document according to `new_order` (1-based page indices).
/// For example, `&[3, 1, 2]` puts page 3 first, then page 1, then page 2.
pub fn reorder_pages(
    input_path: &Path,
    output_path: &Path,
    new_order: &[u32],
) -> Result<(), EngineError> {
    if new_order.is_empty() {
        return Err(EngineError::InvalidParameter(
            "Page order sequence cannot be empty".into(),
        ));
    }

    let mut doc = Document::load(input_path)?;
    let pages = doc.get_pages();
    let total_pages = pages.len() as u32;

    for &page_num in new_order {
        if page_num < 1 || page_num > total_pages {
            return Err(EngineError::InvalidParameter(format!(
                "Page number {page_num} out of bounds (1..={total_pages})"
            )));
        }
    }

    let catalog_id = doc
        .trailer
        .get(b"Root")
        .and_then(|o| o.as_reference())
        .map_err(|_| EngineError::Corrupt("Missing catalog root".into()))?;

    let pages_id = doc
        .get_dictionary(catalog_id)
        .and_then(|dict| dict.get(b"Pages"))
        .and_then(|o| o.as_reference())
        .map_err(|_| EngineError::Corrupt("Missing Pages root in catalog".into()))?;

    let mut new_kids: Vec<Object> = Vec::with_capacity(new_order.len());
    for &p in new_order {
        let page_id = pages
            .get(&p)
            .copied()
            .ok_or(EngineError::PageOutOfRange(p))?;
        new_kids.push(Object::Reference(page_id));
        if let Ok(page_dict) = doc.get_dictionary_mut(page_id) {
            page_dict.set("Parent", Object::Reference(pages_id));
        }
    }

    let pages_dict = doc.get_dictionary_mut(pages_id)?;
    pages_dict.set("Count", Object::Integer(new_order.len() as i64));
    pages_dict.set("Kids", Object::Array(new_kids));

    doc.save(output_path)?;
    Ok(())
}

/// Crop the specified 1-based `page_num` to `media_box: [x0, y0, x1, y1]`.
pub fn crop_page(
    input_path: &Path,
    output_path: &Path,
    page_num: u32,
    media_box: [f32; 4],
) -> Result<(), EngineError> {
    if page_num < 1 {
        return Err(EngineError::InvalidParameter(format!(
            "Invalid page number: {page_num} (pages are 1-based)"
        )));
    }

    let mut doc = Document::load(input_path)?;
    let pages = doc.get_pages();
    let total_pages = pages.len() as u32;

    if page_num > total_pages {
        return Err(EngineError::InvalidParameter(format!(
            "Page number {page_num} exceeds total pages {total_pages}"
        )));
    }

    let page_id = pages[&page_num];
    let page_dict = doc.get_dictionary_mut(page_id)?;

    let box_arr = Object::Array(vec![
        Object::Real(media_box[0]),
        Object::Real(media_box[1]),
        Object::Real(media_box[2]),
        Object::Real(media_box[3]),
    ]);
    page_dict.set("MediaBox", box_arr.clone());
    page_dict.set("CropBox", box_arr);

    doc.save(output_path)?;
    Ok(())
}

/// Insert a blank page with dimensions `width_pt` x `height_pt` after 1-based `after_page`.
/// Use `after_page = 0` to prepend a blank page before page 1.
pub fn insert_blank_page(
    input_path: &Path,
    output_path: &Path,
    after_page: u32,
    width_pt: f32,
    height_pt: f32,
) -> Result<(), EngineError> {
    if width_pt <= 0.0 || height_pt <= 0.0 {
        return Err(EngineError::InvalidParameter(format!(
            "Invalid page dimensions: {width_pt} x {height_pt} (must be positive)"
        )));
    }

    let mut doc = Document::load(input_path)?;
    let pages = doc.get_pages();
    let total_pages = pages.len() as u32;

    if after_page > total_pages {
        return Err(EngineError::InvalidParameter(format!(
            "after_page {after_page} exceeds total pages {total_pages}"
        )));
    }

    let catalog_id = doc
        .trailer
        .get(b"Root")
        .and_then(|o| o.as_reference())
        .map_err(|_| EngineError::Corrupt("Missing catalog root".into()))?;

    let pages_id = doc
        .get_dictionary(catalog_id)
        .and_then(|dict| dict.get(b"Pages"))
        .and_then(|o| o.as_reference())
        .map_err(|_| EngineError::Corrupt("Missing Pages root in catalog".into()))?;

    // Create a new blank Page object
    let mut blank_page = Dictionary::new();
    blank_page.set("Type", Object::Name(b"Page".to_vec()));
    blank_page.set("Parent", Object::Reference(pages_id));
    blank_page.set(
        "MediaBox",
        Object::Array(vec![
            Object::Integer(0),
            Object::Integer(0),
            Object::Real(width_pt),
            Object::Real(height_pt),
        ]),
    );

    let mut resources = Dictionary::new();
    resources.set(
        "ProcSet",
        Object::Array(vec![Object::Name(b"PDF".to_vec())]),
    );
    blank_page.set("Resources", Object::Dictionary(resources));

    let new_page_id = doc.add_object(Object::Dictionary(blank_page));

    // Collect existing kids and insert at appropriate index
    let mut current_kids = Vec::new();
    for p in 1..=total_pages {
        current_kids.push(Object::Reference(pages[&p]));
    }

    let insert_pos = after_page as usize;
    current_kids.insert(insert_pos, Object::Reference(new_page_id));

    let pages_dict = doc.get_dictionary_mut(pages_id)?;
    pages_dict.set("Count", Object::Integer(current_kids.len() as i64));
    pages_dict.set("Kids", Object::Array(current_kids));

    doc.save(output_path)?;
    Ok(())
}

/// Decrypt a password-protected PDF document and write the unencrypted PDF to `output_path`.
pub fn decrypt_pdf(
    input_path: &Path,
    output_path: &Path,
    password: &str,
) -> Result<(), EngineError> {
    if !input_path.exists() {
        return Err(EngineError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Input file not found: {}", input_path.display()),
        )));
    }

    // Try PDFium first (handles full standard Adobe AES/RC4 encryption)
    if let Ok(pdfium) = crate::backend::pdfium::get_or_init_pdfium() {
        let doc = pdfium
            .load_pdf_from_file(input_path, Some(password))
            .map_err(|e| EngineError::Encrypted(format!("Decryption failed: {e:?}")))?;
        doc.save_to_file(output_path)
            .map_err(|e| EngineError::Internal(format!("Failed to save decrypted PDF: {e:?}")))?;
        return Ok(());
    }

    // Fallback: lopdf
    let mut doc = Document::load(input_path)?;
    if !doc.is_encrypted() {
        doc.save(output_path)?;
        return Ok(());
    }

    Err(EngineError::Encrypted(
        "PDF is encrypted; PDFium backend is required for decryption".into(),
    ))
}
