#![allow(clippy::collapsible_if, clippy::collapsible_match)]

use crate::error::EngineError;
use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};
use std::collections::HashMap;
use std::path::Path;

/// High-level type of an interactive AcroForm field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormFieldType {
    /// Single-line or multi-line text input field (/Tx).
    Text,
    /// Toggle checkbox (/Btn without Pushbutton or Radio flags).
    Checkbox,
    /// Selection choice list or combo box (/Ch, or /Btn with Radio flag).
    Choice,
    /// Action trigger button (/Btn with Pushbutton flag).
    Button,
}

/// Metadata and state for an AcroForm field.
#[derive(Debug, Clone, PartialEq)]
pub struct FormFieldInfo {
    /// Fully qualified or partial field name (/T).
    pub name: String,
    /// Field type.
    pub field_type: FormFieldType,
    /// Current field value as string (/V).
    pub value: String,
    /// Whether the field is read-only according to field flags (/Ff bit 1).
    pub read_only: bool,
}

/// Read all interactive form fields from a PDF file.
pub fn read_form_fields(input_path: &Path) -> Result<Vec<FormFieldInfo>, EngineError> {
    let doc = Document::load(input_path)?;
    read_form_fields_doc(&doc)
}

/// In-memory variant of `read_form_fields`.
pub fn read_form_fields_bytes(input_bytes: &[u8]) -> Result<Vec<FormFieldInfo>, EngineError> {
    let doc = Document::load_mem(input_bytes)?;
    read_form_fields_doc(&doc)
}

fn read_form_fields_doc(doc: &Document) -> Result<Vec<FormFieldInfo>, EngineError> {
    let mut results = Vec::new();

    let Ok(catalog_id) = doc.trailer.get(b"Root").and_then(|o| o.as_reference()) else {
        return Ok(results);
    };
    let Ok(catalog) = doc.get_dictionary(catalog_id) else {
        return Ok(results);
    };

    let Some(acroform_obj) = catalog.get(b"AcroForm").ok() else {
        return Ok(results);
    };

    let acroform_dict = match acroform_obj {
        Object::Dictionary(d) => d,
        Object::Reference(id) => match doc.get_dictionary(*id) {
            Ok(d) => d,
            _ => return Ok(results),
        },
        _ => return Ok(results),
    };

    let Some(fields_obj) = acroform_dict.get(b"Fields").ok() else {
        return Ok(results);
    };

    let field_refs: Vec<ObjectId> = match fields_obj {
        Object::Array(arr) => arr.iter().filter_map(|o| o.as_reference().ok()).collect(),
        _ => Vec::new(),
    };

    for id in field_refs {
        collect_fields_recursive(doc, id, None, &mut results);
    }

    Ok(results)
}

fn collect_fields_recursive(
    doc: &Document,
    obj_id: ObjectId,
    parent_type: Option<FormFieldType>,
    out: &mut Vec<FormFieldInfo>,
) {
    let Ok(dict) = doc.get_dictionary(obj_id) else {
        return;
    };

    let mut current_type = parent_type;
    if let Ok(ft_obj) = dict.get(b"FT") {
        if let Object::Name(ft_name) = ft_obj {
            let flags = dict.get(b"Ff").and_then(|o| o.as_i64()).unwrap_or(0);
            current_type = Some(resolve_field_type(ft_name, flags));
        }
    }

    let has_kids = dict.has(b"Kids");
    let name_opt = dict
        .get(b"T")
        .ok()
        .and_then(|o| extract_string_from_obj(o, doc));

    if let (Some(name), Some(ft)) = (name_opt.clone(), current_type) {
        let value = dict
            .get(b"V")
            .ok()
            .and_then(|o| extract_string_from_obj(o, doc))
            .unwrap_or_default();

        let flags = dict.get(b"Ff").and_then(|o| o.as_i64()).unwrap_or(0);
        let read_only = (flags & 1) != 0;

        out.push(FormFieldInfo {
            name,
            field_type: ft,
            value,
            read_only,
        });
    }

    if has_kids {
        if let Ok(Object::Array(kids)) = dict.get(b"Kids") {
            for kid in kids {
                if let Ok(kid_id) = kid.as_reference() {
                    collect_fields_recursive(doc, kid_id, current_type, out);
                }
            }
        }
    }
}

fn resolve_field_type(ft_name: &[u8], flags: i64) -> FormFieldType {
    match ft_name {
        b"Tx" => FormFieldType::Text,
        b"Btn" => {
            const FLAG_PUSHBUTTON: i64 = 1 << 16;
            const FLAG_RADIO: i64 = 1 << 15;
            if (flags & FLAG_PUSHBUTTON) != 0 {
                FormFieldType::Button
            } else if (flags & FLAG_RADIO) != 0 {
                FormFieldType::Choice
            } else {
                FormFieldType::Checkbox
            }
        }
        b"Ch" => FormFieldType::Choice,
        _ => FormFieldType::Text,
    }
}

fn extract_string_from_obj(obj: &Object, doc: &Document) -> Option<String> {
    match obj {
        Object::String(bytes, _) => Some(decode_pdf_string(bytes)),
        Object::Name(bytes) => Some(String::from_utf8_lossy(bytes).to_string()),
        Object::Boolean(b) => Some(b.to_string()),
        Object::Integer(i) => Some(i.to_string()),
        Object::Real(r) => Some(r.to_string()),
        Object::Reference(id) => {
            if let Ok(derefed) = doc.get_object(*id) {
                extract_string_from_obj(derefed, doc)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn decode_pdf_string(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        let (chunks, _) = bytes[2..].as_chunks::<2>();
        let u16_chars: Vec<u16> = chunks
            .iter()
            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
            .collect();
        String::from_utf16_lossy(&u16_chars)
    } else {
        String::from_utf8_lossy(bytes).to_string()
    }
}

/// Fill interactive form fields in a PDF document with provided values.
/// If `flatten` is true, the form fields are permanently baked into the document page content,
/// interactive widget annotations are removed, and the AcroForm is stripped.
pub fn fill_form_fields(
    input_path: &Path,
    field_values: &HashMap<String, String>,
    flatten: bool,
    output_path: &Path,
) -> Result<(), EngineError> {
    let mut doc = Document::load(input_path)?;
    fill_form_fields_doc(&mut doc, field_values, flatten)?;
    doc.save(output_path)?;
    Ok(())
}

/// In-memory variant of `fill_form_fields`.
pub fn fill_form_fields_bytes(
    input_bytes: &[u8],
    field_values: &HashMap<String, String>,
    flatten: bool,
) -> Result<Vec<u8>, EngineError> {
    let mut doc = Document::load_mem(input_bytes)?;
    fill_form_fields_doc(&mut doc, field_values, flatten)?;
    let mut out = Vec::new();
    doc.save_to(&mut out)?;
    Ok(out)
}

fn fill_form_fields_doc(
    doc: &mut Document,
    field_values: &HashMap<String, String>,
    flatten: bool,
) -> Result<(), EngineError> {
    let mut widget_bakes: Vec<(Option<ObjectId>, [f32; 4], String)> = Vec::new();

    // 1. Traverse and update fields matching names in field_values
    let fields_to_update = find_fields_to_update(doc, field_values);

    for (field_id, new_value) in fields_to_update {
        if let Ok(dict) = doc.get_dictionary_mut(field_id) {
            let flags = dict.get(b"Ff").and_then(|o| o.as_i64()).unwrap_or(0);
            let ft_name = dict
                .get(b"FT")
                .and_then(|o| o.as_name())
                .unwrap_or(b"Tx")
                .to_vec();
            let ft = resolve_field_type(&ft_name, flags);

            match ft {
                FormFieldType::Checkbox => {
                    let is_checked = new_value.eq_ignore_ascii_case("true")
                        || new_value.eq_ignore_ascii_case("yes")
                        || new_value == "1"
                        || new_value.eq_ignore_ascii_case("on");

                    let val_name = if is_checked {
                        b"Yes".to_vec()
                    } else {
                        b"Off".to_vec()
                    };
                    dict.set("V", Object::Name(val_name.clone()));
                    dict.set("AS", Object::Name(val_name));
                }
                _ => {
                    dict.set("V", Object::string_literal(&*new_value));
                }
            }

            if flatten {
                if let Ok(Object::Array(rect_arr)) = dict.get(b"Rect") {
                    if rect_arr.len() >= 4 {
                        let x0 = rect_arr[0].as_float().unwrap_or(0.0);
                        let y0 = rect_arr[1].as_float().unwrap_or(0.0);
                        let x1 = rect_arr[2].as_float().unwrap_or(0.0);
                        let y1 = rect_arr[3].as_float().unwrap_or(0.0);
                        let page_ref = dict.get(b"P").ok().and_then(|p| p.as_reference().ok());
                        widget_bakes.push((page_ref, [x0, y0, x1, y1], new_value.clone()));
                    }
                }
            }
        }
    }

    // 2. Set /NeedAppearances true in AcroForm dictionary so standard viewers render appearances
    if let Ok(cat_id) = doc.trailer.get(b"Root").and_then(|o| o.as_reference()) {
        if let Ok(cat) = doc.get_dictionary_mut(cat_id) {
            if let Ok(acroform_obj) = cat.get_mut(b"AcroForm") {
                if let Object::Dictionary(d) = acroform_obj {
                    d.set("NeedAppearances", Object::Boolean(true));
                }
            }
        }
    }

    // 3. If flatten requested: bake text into pages, remove widgets from /Annots, remove /AcroForm
    if flatten {
        flatten_form_fields(doc, widget_bakes)?;
    }

    Ok(())
}

fn find_fields_to_update(
    doc: &Document,
    field_values: &HashMap<String, String>,
) -> Vec<(ObjectId, String)> {
    let mut matches = Vec::new();

    let Ok(catalog_id) = doc.trailer.get(b"Root").and_then(|o| o.as_reference()) else {
        return matches;
    };
    let Ok(catalog) = doc.get_dictionary(catalog_id) else {
        return matches;
    };
    let Some(acroform_obj) = catalog.get(b"AcroForm").ok() else {
        return matches;
    };

    let acroform_dict = match acroform_obj {
        Object::Dictionary(d) => d,
        Object::Reference(id) => match doc.get_dictionary(*id) {
            Ok(d) => d,
            _ => return matches,
        },
        _ => return matches,
    };

    let Some(fields_obj) = acroform_dict.get(b"Fields").ok() else {
        return matches;
    };

    let field_refs: Vec<ObjectId> = match fields_obj {
        Object::Array(arr) => arr.iter().filter_map(|o| o.as_reference().ok()).collect(),
        _ => Vec::new(),
    };

    for id in field_refs {
        find_fields_recursive(doc, id, field_values, &mut matches);
    }

    matches
}

fn find_fields_recursive(
    doc: &Document,
    obj_id: ObjectId,
    field_values: &HashMap<String, String>,
    out: &mut Vec<(ObjectId, String)>,
) {
    let Ok(dict) = doc.get_dictionary(obj_id) else {
        return;
    };

    if let Ok(name_obj) = dict.get(b"T") {
        if let Some(name) = extract_string_from_obj(name_obj, doc) {
            if let Some(new_val) = field_values.get(&name) {
                out.push((obj_id, new_val.clone()));
            }
        }
    }

    if let Ok(Object::Array(kids)) = dict.get(b"Kids") {
        for kid in kids {
            if let Ok(kid_id) = kid.as_reference() {
                find_fields_recursive(doc, kid_id, field_values, out);
            }
        }
    }
}

fn flatten_form_fields(
    doc: &mut Document,
    widget_bakes: Vec<(Option<ObjectId>, [f32; 4], String)>,
) -> Result<(), EngineError> {
    let pages = doc.get_pages();
    let first_page_id = pages.values().next().copied();

    // 1. Bake field text onto target pages
    for (page_opt, rect, value) in widget_bakes {
        if value.is_empty() {
            continue;
        }

        let target_page_id = page_opt.or(first_page_id);
        let Some(page_id) = target_page_id else {
            continue;
        };

        let font_size = ((rect[3] - rect[1]).abs() - 4.0).clamp(8.0, 12.0);
        let x = rect[0].min(rect[2]) + 2.0;
        let y = rect[1].min(rect[3]) + 3.0;

        ensure_flatten_font(doc, page_id)?;

        let ops = vec![
            Operation::new("q", vec![]),
            Operation::new(
                "rg",
                vec![Object::Real(0.0), Object::Real(0.0), Object::Real(0.0)],
            ),
            Operation::new("BT", vec![]),
            Operation::new(
                "Tf",
                vec![Object::Name(b"F_FLAT".to_vec()), Object::Real(font_size)],
            ),
            Operation::new(
                "Tm",
                vec![
                    Object::Real(1.0),
                    Object::Real(0.0),
                    Object::Real(0.0),
                    Object::Real(1.0),
                    Object::Real(x),
                    Object::Real(y),
                ],
            ),
            Operation::new("Tj", vec![Object::string_literal(&*value)]),
            Operation::new("ET", vec![]),
            Operation::new("Q", vec![]),
        ];

        let content = Content { operations: ops };
        let stream_bytes = content.encode()?;
        let stream = Stream::new(Dictionary::new(), stream_bytes);
        let bake_stream_id = doc.add_object(Object::Stream(stream));

        if let Ok(page_dict) = doc.get_dictionary_mut(page_id) {
            match page_dict.get_mut(b"Contents") {
                Ok(Object::Array(arr)) => {
                    arr.push(Object::Reference(bake_stream_id));
                }
                Ok(Object::Reference(existing_id)) => {
                    let old_ref = *existing_id;
                    page_dict.set(
                        "Contents",
                        Object::Array(vec![
                            Object::Reference(old_ref),
                            Object::Reference(bake_stream_id),
                        ]),
                    );
                }
                _ => {
                    page_dict.set("Contents", Object::Reference(bake_stream_id));
                }
            }
        }
    }

    // 2. Remove all Widget annotations from pages
    let page_ids: Vec<ObjectId> = doc.get_pages().values().copied().collect();
    for page_id in page_ids {
        let mut widget_annot_ids = Vec::new();
        if let Ok(page_dict) = doc.get_dictionary(page_id) {
            if let Ok(Object::Array(annots)) = page_dict.get(b"Annots") {
                for o in annots {
                    if let Ok(id) = o.as_reference() {
                        if let Ok(annot) = doc.get_dictionary(id) {
                            if annot.get(b"Subtype").ok().and_then(|s| s.as_name().ok())
                                == Some(b"Widget".as_slice())
                            {
                                widget_annot_ids.push(id);
                            }
                        }
                    }
                }
            }
        }

        if let Ok(page_dict) = doc.get_dictionary_mut(page_id) {
            if let Ok(Object::Array(annots)) = page_dict.get_mut(b"Annots") {
                annots.retain(|o| {
                    if let Ok(id) = o.as_reference() {
                        !widget_annot_ids.contains(&id)
                    } else {
                        true
                    }
                });
            }
        }
    }

    // 3. Remove /AcroForm from catalog
    if let Ok(cat_id) = doc.trailer.get(b"Root").and_then(|o| o.as_reference()) {
        if let Ok(cat) = doc.get_dictionary_mut(cat_id) {
            let _ = cat.remove(b"AcroForm");
        }
    }

    // 4. Prune unreferenced objects
    doc.prune_objects();

    Ok(())
}

fn ensure_flatten_font(doc: &mut Document, page_id: ObjectId) -> Result<(), EngineError> {
    let mut needs_font = true;
    if let Ok(page_dict) = doc.get_dictionary(page_id) {
        if let Ok(Object::Dictionary(res)) = page_dict.get(b"Resources") {
            if let Ok(Object::Dictionary(fonts)) = res.get(b"Font") {
                if fonts.has(b"F_FLAT") {
                    needs_font = false;
                }
            }
        }
    }

    let font_obj_id = if needs_font {
        let mut font_dict = Dictionary::new();
        font_dict.set("Type", Object::Name(b"Font".to_vec()));
        font_dict.set("Subtype", Object::Name(b"Type1".to_vec()));
        font_dict.set("BaseFont", Object::Name(b"Helvetica".to_vec()));
        Some(doc.add_object(Object::Dictionary(font_dict)))
    } else {
        None
    };

    if let Some(font_id) = font_obj_id {
        let page_dict = doc.get_dictionary_mut(page_id)?;
        if !page_dict.has(b"Resources") {
            page_dict.set("Resources", Object::Dictionary(Dictionary::new()));
        }
        if let Ok(Object::Dictionary(resources)) = page_dict.get_mut(b"Resources") {
            if !resources.has(b"Font") {
                resources.set("Font", Object::Dictionary(Dictionary::new()));
            }
            if let Ok(Object::Dictionary(fonts)) = resources.get_mut(b"Font") {
                fonts.set("F_FLAT", Object::Reference(font_id));
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_form_fields_from_corpus() {
        let path = Path::new("../../../corpus/files/pdf-forms/pdfium-text-form.pdf");
        let alt_path = Path::new("corpus/files/pdf-forms/pdfium-text-form.pdf");
        let real_path = if path.exists() {
            path
        } else if alt_path.exists() {
            alt_path
        } else {
            return;
        };

        let fields = read_form_fields(real_path).unwrap();
        assert!(
            !fields.is_empty(),
            "Should read fields from real corpus PDF"
        );
        assert!(
            fields.iter().any(|f| f.field_type == FormFieldType::Text),
            "Must have text field"
        );
    }

    #[test]
    fn test_fill_form_fields_and_read_back() {
        let path = Path::new("../../../corpus/files/pdf-forms/pdfium-text-form.pdf");
        let alt_path = Path::new("corpus/files/pdf-forms/pdfium-text-form.pdf");
        let target_path = if path.exists() {
            path
        } else if alt_path.exists() {
            alt_path
        } else {
            return;
        };

        let fields_before = read_form_fields(target_path).unwrap();
        assert!(!fields_before.is_empty());
        let field_name = &fields_before[0].name;

        let mut values = HashMap::new();
        values.insert(field_name.clone(), "Test Filled Content 123".to_string());

        let bytes = std::fs::read(target_path).unwrap();
        let filled_bytes = fill_form_fields_bytes(&bytes, &values, false).unwrap();

        let fields_after = read_form_fields_bytes(&filled_bytes).unwrap();
        let updated = fields_after.iter().find(|f| f.name == *field_name).unwrap();
        assert_eq!(updated.value, "Test Filled Content 123");
    }

    #[test]
    fn test_fill_form_fields_with_flatten() {
        let path = Path::new("../../../corpus/files/pdf-forms/pdfium-text-form.pdf");
        let alt_path = Path::new("corpus/files/pdf-forms/pdfium-text-form.pdf");
        let target_path = if path.exists() {
            path
        } else if alt_path.exists() {
            alt_path
        } else {
            return;
        };

        let fields_before = read_form_fields(target_path).unwrap();
        let field_name = &fields_before[0].name;

        let mut values = HashMap::new();
        values.insert(field_name.clone(), "Flattened Text Value".to_string());

        let bytes = std::fs::read(target_path).unwrap();
        let flattened_bytes = fill_form_fields_bytes(&bytes, &values, true).unwrap();

        // When flattened, AcroForm is removed so read_form_fields returns empty
        let fields_after = read_form_fields_bytes(&flattened_bytes).unwrap();
        assert!(
            fields_after.is_empty(),
            "Flattened PDF must have no interactive form fields remaining"
        );

        // But the text content must be baked into the document bytes
        let s = String::from_utf8_lossy(&flattened_bytes);
        assert!(
            s.contains("Flattened Text Value"),
            "Filled text must be permanently baked into page content stream"
        );
    }
}
