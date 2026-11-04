#![allow(clippy::collapsible_if, clippy::collapsible_match)]

use crate::error::EngineError;
use crate::search::PdfRect;
use lopdf::content::{Content, Operation};
use lopdf::{Document, Object};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Redaction specification for a single page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PageRedaction {
    /// 0-based page index.
    pub page_index: u32,
    /// Rectangles to redact in PDF user-space point coordinates.
    pub rects: Vec<PdfRect>,
    /// Fill color for visual redaction blackout box (R, G, B in 0..=255).
    /// Defaults to (0, 0, 0) (solid black).
    pub fill_color: (u8, u8, u8),
}

impl PageRedaction {
    /// Create a new redaction target with default black fill color (0, 0, 0).
    pub fn new(page_index: u32, rects: Vec<PdfRect>) -> Self {
        Self {
            page_index,
            rects,
            fill_color: (0, 0, 0),
        }
    }

    /// Set a custom fill color for the visual blackout rectangle.
    pub fn with_fill_color(mut self, fill_color: (u8, u8, u8)) -> Self {
        self.fill_color = fill_color;
        self
    }
}

/// 2D affine transformation matrix for tracking PDF CTM and Text Matrix.
#[derive(Clone, Copy, Debug)]
struct Matrix2D {
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
}

impl Matrix2D {
    fn identity() -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: 0.0,
            f: 0.0,
        }
    }

    /// Multiply self by other: `self * other`.
    fn multiply(&self, other: &Matrix2D) -> Matrix2D {
        Matrix2D {
            a: self.a * other.a + self.b * other.c,
            b: self.a * other.b + self.b * other.d,
            c: self.c * other.a + self.d * other.c,
            d: self.c * other.b + self.d * other.d,
            e: self.e * other.a + self.f * other.c + other.e,
            f: self.e * other.b + self.f * other.d + other.f,
        }
    }

    fn transform_point(&self, x: f32, y: f32) -> (f32, f32) {
        (
            x * self.a + y * self.c + self.e,
            x * self.b + y * self.d + self.f,
        )
    }
}

/// Apply permanent redactions to a PDF document.
///
/// CRITICAL SECURITY REQUIREMENT:
/// Merely drawing a black box over text is NOT sufficient. This function:
/// 1. Purges underlying text strings from the content stream operations (clearing/replacing
///    redacted text with blanks/empty strings so it cannot be copied, selected, or searched).
/// 2. Deletes any matching annotations (StickyNotes, Highlights, etc.) intersecting the redaction zones.
/// 3. Appends an opaque blackout fill rectangle to the page content stream.
pub fn redact_document(
    input_path: &Path,
    redactions: &[PageRedaction],
    output_path: &Path,
) -> Result<(), EngineError> {
    let mut doc = Document::load(input_path)?;
    redact_doc_internal(&mut doc, redactions)?;
    doc.save(output_path)?;
    Ok(())
}

/// In-memory variant of `redact_document`.
pub fn redact_document_bytes(
    input_bytes: &[u8],
    redactions: &[PageRedaction],
) -> Result<Vec<u8>, EngineError> {
    let mut doc = Document::load_mem(input_bytes)?;
    redact_doc_internal(&mut doc, redactions)?;
    let mut out = Vec::new();
    doc.save_to(&mut out)?;
    Ok(out)
}

fn redact_doc_internal(
    doc: &mut Document,
    redactions: &[PageRedaction],
) -> Result<(), EngineError> {
    let pages = doc.get_pages();

    for redaction in redactions {
        if redaction.rects.is_empty() {
            continue;
        }

        let page_num = redaction.page_index + 1;
        let Some(&page_id) = pages.get(&page_num) else {
            continue;
        };

        // 1. Purge/remove annotations intersecting the redaction rectangles
        purge_annotations_in_rects(doc, page_id, &redaction.rects)?;

        // 2. Decode and sanitize page content stream
        let content_data = doc.get_page_content(page_id);
        let mut content = if !content_data.is_empty() {
            Content::decode(&content_data)?
        } else {
            Content {
                operations: Vec::new(),
            }
        };

        // Sanitize text inside content stream
        sanitize_content_text(&mut content, &redaction.rects);

        // 3. Append opaque blackout rectangle fill operators
        let r = redaction.fill_color.0 as f32 / 255.0;
        let g = redaction.fill_color.1 as f32 / 255.0;
        let b = redaction.fill_color.2 as f32 / 255.0;

        content.operations.push(Operation::new("q", vec![]));
        content.operations.push(Operation::new(
            "rg",
            vec![Object::Real(r), Object::Real(g), Object::Real(b)],
        ));

        for rect in &redaction.rects {
            let rx = rect.x0.min(rect.x1);
            let ry = rect.y0.min(rect.y1);
            let rw = (rect.x1 - rect.x0).abs();
            let rh = (rect.y1 - rect.y0).abs();

            content.operations.push(Operation::new(
                "re",
                vec![
                    Object::Real(rx),
                    Object::Real(ry),
                    Object::Real(rw),
                    Object::Real(rh),
                ],
            ));
            content.operations.push(Operation::new("f", vec![]));
        }

        content.operations.push(Operation::new("Q", vec![]));

        let encoded = content.encode()?;
        doc.change_page_content(page_id, encoded)?;
    }

    Ok(())
}

/// Remove any annotations on `page_id` whose bounding box intersects `rects`.
fn purge_annotations_in_rects(
    doc: &mut Document,
    page_id: lopdf::ObjectId,
    rects: &[PdfRect],
) -> Result<(), EngineError> {
    let Ok(page_dict) = doc.get_dictionary(page_id) else {
        return Ok(());
    };

    let Some(annots_obj) = page_dict.get(b"Annots").ok() else {
        return Ok(());
    };

    let annot_refs: Vec<Object> = match annots_obj {
        Object::Array(arr) => arr.clone(),
        Object::Reference(id) => {
            if let Ok(Object::Array(arr)) = doc.get_object(*id) {
                arr.clone()
            } else {
                return Ok(());
            }
        }
        _ => return Ok(()),
    };

    let mut retained_annots = Vec::new();
    let mut to_delete = Vec::new();

    for obj in annot_refs {
        let mut intersects = false;
        if let Object::Reference(id) = obj {
            if let Ok(annot_dict) = doc.get_dictionary(id) {
                if let Ok(rect_obj) = annot_dict.get(b"Rect") {
                    if let Some(rect) = parse_pdf_rect(rect_obj) {
                        for r in rects {
                            if rects_intersect(&rect, r) {
                                intersects = true;
                                break;
                            }
                        }
                    }
                }
            }
            if intersects {
                to_delete.push(id);
            } else {
                retained_annots.push(Object::Reference(id));
            }
        } else if let Object::Dictionary(ref annot_dict) = obj {
            if let Ok(rect_obj) = annot_dict.get(b"Rect") {
                if let Some(rect) = parse_pdf_rect(rect_obj) {
                    for r in rects {
                        if rects_intersect(&rect, r) {
                            intersects = true;
                            break;
                        }
                    }
                }
            }
            if !intersects {
                retained_annots.push(obj);
            }
        }
    }

    for id in to_delete {
        doc.delete_object(id);
    }

    if let Ok(page_dict_mut) = doc.get_dictionary_mut(page_id) {
        if retained_annots.is_empty() {
            let _ = page_dict_mut.remove(b"Annots");
        } else {
            page_dict_mut.set("Annots", Object::Array(retained_annots));
        }
    }

    Ok(())
}

fn parse_pdf_rect(obj: &Object) -> Option<PdfRect> {
    if let Object::Array(arr) = obj {
        if arr.len() >= 4 {
            let x0 = arr[0].as_float().ok()?;
            let y0 = arr[1].as_float().ok()?;
            let x1 = arr[2].as_float().ok()?;
            let y1 = arr[3].as_float().ok()?;
            return Some(PdfRect::new(x0, y0, x1, y1));
        }
    }
    None
}

fn rects_intersect(a: &PdfRect, b: &PdfRect) -> bool {
    let ax_min = a.x0.min(a.x1);
    let ax_max = a.x0.max(a.x1);
    let ay_min = a.y0.min(a.y1);
    let ay_max = a.y0.max(a.y1);

    let bx_min = b.x0.min(b.x1);
    let bx_max = b.x0.max(b.x1);
    let by_min = b.y0.min(b.y1);
    let by_max = b.y0.max(b.y1);

    !(ax_max < bx_min || ax_min > bx_max || ay_max < by_min || ay_min > by_max)
}

/// Check if a text position (or character point) falls inside any redaction rectangle.
fn is_point_in_redaction_rects(x: f32, y: f32, font_size: f32, rects: &[PdfRect]) -> bool {
    let y_bottom = y - 0.25 * font_size;
    let y_top = y + 0.95 * font_size;

    for rect in rects {
        let rx_min = rect.x0.min(rect.x1);
        let rx_max = rect.x0.max(rect.x1);
        let ry_min = rect.y0.min(rect.y1);
        let ry_max = rect.y0.max(rect.y1);

        let x_in = x >= (rx_min - 1.0) && x <= (rx_max + 1.0);
        let y_overlap = y_bottom <= ry_max && y_top >= ry_min;
        if x_in && y_overlap {
            return true;
        }
    }
    false
}

/// Sanitize text inside content stream operations by zeroing or blanking characters
/// whose coordinates fall within any redaction rectangle.
fn sanitize_content_text(content: &mut Content, rects: &[PdfRect]) {
    let mut ctm = Matrix2D::identity();
    let mut ctm_stack: Vec<Matrix2D> = Vec::new();

    let mut tm = Matrix2D::identity();
    let mut tlm = Matrix2D::identity();
    let mut font_size: f32 = 12.0;
    let mut leading: f32 = 0.0;

    for op in &mut content.operations {
        match op.operator.as_str() {
            "q" => {
                ctm_stack.push(ctm);
            }
            "Q" => {
                ctm = ctm_stack.pop().unwrap_or_else(Matrix2D::identity);
            }
            "cm" => {
                if op.operands.len() >= 6 {
                    let a = op.operands[0].as_float().unwrap_or(1.0);
                    let b = op.operands[1].as_float().unwrap_or(0.0);
                    let c = op.operands[2].as_float().unwrap_or(0.0);
                    let d = op.operands[3].as_float().unwrap_or(1.0);
                    let e = op.operands[4].as_float().unwrap_or(0.0);
                    let f = op.operands[5].as_float().unwrap_or(0.0);
                    let m = Matrix2D { a, b, c, d, e, f };
                    ctm = m.multiply(&ctm);
                }
            }
            "BT" => {
                tm = Matrix2D::identity();
                tlm = Matrix2D::identity();
            }
            "ET" => {
                tm = Matrix2D::identity();
                tlm = Matrix2D::identity();
            }
            "Tf" => {
                if op.operands.len() >= 2 {
                    font_size = op.operands[1].as_float().unwrap_or(12.0);
                }
            }
            "Tm" => {
                if op.operands.len() >= 6 {
                    let a = op.operands[0].as_float().unwrap_or(1.0);
                    let b = op.operands[1].as_float().unwrap_or(0.0);
                    let c = op.operands[2].as_float().unwrap_or(0.0);
                    let d = op.operands[3].as_float().unwrap_or(1.0);
                    let e = op.operands[4].as_float().unwrap_or(0.0);
                    let f = op.operands[5].as_float().unwrap_or(0.0);
                    tm = Matrix2D { a, b, c, d, e, f };
                    tlm = tm;
                }
            }
            "Td" => {
                if op.operands.len() >= 2 {
                    let tx = op.operands[0].as_float().unwrap_or(0.0);
                    let ty = op.operands[1].as_float().unwrap_or(0.0);
                    let trans = Matrix2D {
                        a: 1.0,
                        b: 0.0,
                        c: 0.0,
                        d: 1.0,
                        e: tx,
                        f: ty,
                    };
                    tlm = trans.multiply(&tlm);
                    tm = tlm;
                }
            }
            "TD" => {
                if op.operands.len() >= 2 {
                    let tx = op.operands[0].as_float().unwrap_or(0.0);
                    let ty = op.operands[1].as_float().unwrap_or(0.0);
                    leading = -ty;
                    let trans = Matrix2D {
                        a: 1.0,
                        b: 0.0,
                        c: 0.0,
                        d: 1.0,
                        e: tx,
                        f: ty,
                    };
                    tlm = trans.multiply(&tlm);
                    tm = tlm;
                }
            }
            "T*" => {
                let trans = Matrix2D {
                    a: 1.0,
                    b: 0.0,
                    c: 0.0,
                    d: 1.0,
                    e: 0.0,
                    f: -leading,
                };
                tlm = trans.multiply(&tlm);
                tm = tlm;
            }
            "Tj" | "'" | "\"" => {
                // If ' or ", there is an implicit newline
                if op.operator == "'" || op.operator == "\"" {
                    let trans = Matrix2D {
                        a: 1.0,
                        b: 0.0,
                        c: 0.0,
                        d: 1.0,
                        e: 0.0,
                        f: -leading,
                    };
                    tlm = trans.multiply(&tlm);
                    tm = tlm;
                }

                // The string operand is the last operand
                if let Some(target) = op.operands.last_mut() {
                    if let Object::String(bytes, _fmt) = target {
                        sanitize_string_bytes(bytes, &tm, &ctm, font_size, rects);
                    }
                }
            }
            "TJ" => {
                if let Some(Object::Array(arr)) = op.operands.first_mut() {
                    let mut local_x_offset: f32 = 0.0;
                    for item in arr.iter_mut() {
                        match item {
                            Object::String(bytes, _fmt) => {
                                let shifted_tm = Matrix2D {
                                    e: tm.e + local_x_offset * tm.a,
                                    f: tm.f + local_x_offset * tm.b,
                                    ..tm
                                };
                                let advance = sanitize_string_bytes(
                                    bytes,
                                    &shifted_tm,
                                    &ctm,
                                    font_size,
                                    rects,
                                );
                                local_x_offset += advance;
                            }
                            Object::Integer(k) => {
                                local_x_offset -= (*k as f32) / 1000.0 * font_size;
                            }
                            Object::Real(k) => {
                                local_x_offset -= (*k) / 1000.0 * font_size;
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Sanitize character bytes in a string.
/// Returns the approximate advance width of this string in points.
fn sanitize_string_bytes(
    bytes: &mut Vec<u8>,
    tm: &Matrix2D,
    ctm: &Matrix2D,
    font_size: f32,
    rects: &[PdfRect],
) -> f32 {
    let char_width = 0.52 * font_size;
    let mut any_redacted = false;
    let mut all_redacted = true;

    for (i, byte) in bytes.iter_mut().enumerate() {
        let local_x = i as f32 * char_width;
        let pt_text = (tm.e + local_x * tm.a, tm.f + local_x * tm.b);
        let pt_user = ctm.transform_point(pt_text.0, pt_text.1);

        if is_point_in_redaction_rects(pt_user.0, pt_user.1, font_size, rects) {
            *byte = b' ';
            any_redacted = true;
        } else {
            all_redacted = false;
        }
    }

    if all_redacted && any_redacted {
        bytes.clear();
    }

    bytes.len() as f32 * char_width
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::content::Operation;
    use lopdf::{Dictionary, Document, Object, Stream};

    fn create_test_pdf_with_text(text: &str) -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();

        let mut content = Content {
            operations: Vec::new(),
        };
        content.operations.push(Operation::new("BT", vec![]));
        content.operations.push(Operation::new(
            "Tf",
            vec![Object::Name(b"F1".to_vec()), Object::Real(12.0)],
        ));
        content.operations.push(Operation::new(
            "Tm",
            vec![
                Object::Real(1.0),
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(1.0),
                Object::Real(100.0),
                Object::Real(500.0),
            ],
        ));
        content
            .operations
            .push(Operation::new("Tj", vec![Object::string_literal(text)]));
        content.operations.push(Operation::new("ET", vec![]));

        let content_bytes = content.encode().unwrap();
        let stream = Stream::new(Dictionary::new(), content_bytes);
        let content_id = doc.add_object(Object::Stream(stream));

        let mut font_dict = Dictionary::new();
        font_dict.set("Type", Object::Name(b"Font".to_vec()));
        font_dict.set("Subtype", Object::Name(b"Type1".to_vec()));
        font_dict.set("BaseFont", Object::Name(b"Helvetica".to_vec()));
        let font_id = doc.add_object(Object::Dictionary(font_dict));

        let mut fonts = Dictionary::new();
        fonts.set("F1", Object::Reference(font_id));
        let mut resources = Dictionary::new();
        resources.set("Font", Object::Dictionary(fonts));

        let mut page_dict = Dictionary::new();
        page_dict.set("Type", Object::Name(b"Page".to_vec()));
        page_dict.set("Parent", Object::Reference(pages_id));
        page_dict.set(
            "MediaBox",
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(600.0),
                Object::Real(800.0),
            ]),
        );
        page_dict.set("Contents", Object::Reference(content_id));
        page_dict.set("Resources", Object::Dictionary(resources));
        let page_id = doc.add_object(Object::Dictionary(page_dict));

        let mut pages_dict = Dictionary::new();
        pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
        pages_dict.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
        pages_dict.set("Count", Object::Integer(1));
        doc.objects.insert(pages_id, Object::Dictionary(pages_dict));

        let mut catalog = Dictionary::new();
        catalog.set("Type", Object::Name(b"Catalog".to_vec()));
        catalog.set("Pages", Object::Reference(pages_id));
        let cat_id = doc.add_object(Object::Dictionary(catalog));
        doc.trailer.set("Root", Object::Reference(cat_id));

        let mut out = Vec::new();
        doc.save_to(&mut out).unwrap();
        out
    }

    #[test]
    fn test_redaction_text_purging_and_search() {
        let pdf_data = create_test_pdf_with_text("SECRET AGENT CONFIDENTIAL DATA");
        assert!(
            String::from_utf8_lossy(&pdf_data).contains("CONFIDENTIAL"),
            "Initial PDF must contain CONFIDENTIAL"
        );

        // Text is at x=100..around 250, y=500.
        // Let's redact the region x=100..300, y=490..520
        let redaction = PageRedaction::new(0, vec![PdfRect::new(90.0, 490.0, 310.0, 520.0)]);

        let redacted_bytes = redact_document_bytes(&pdf_data, &[redaction]).unwrap();
        let redacted_str = String::from_utf8_lossy(&redacted_bytes);

        // Security check: CONFIDENTIAL must NOT exist in the redacted PDF content!
        assert!(
            !redacted_str.contains("CONFIDENTIAL"),
            "Redacted text must be permanently purged from PDF bytes"
        );
        assert!(
            !redacted_str.contains("SECRET AGENT"),
            "Fully covered text must be purged"
        );
    }

    #[test]
    fn test_redaction_annotation_removal() {
        let base_pdf = create_test_pdf_with_text("Sample Document with Comments");
        let mut doc = Document::load_mem(&base_pdf).unwrap();

        // Add a StickyNote annotation at x=100..150, y=490..510
        let mut annot_dict = Dictionary::new();
        annot_dict.set("Type", Object::Name(b"Annot".to_vec()));
        annot_dict.set("Subtype", Object::Name(b"Text".to_vec()));
        annot_dict.set(
            "Rect",
            Object::Array(vec![
                Object::Real(100.0),
                Object::Real(490.0),
                Object::Real(150.0),
                Object::Real(510.0),
            ]),
        );
        annot_dict.set("Contents", Object::string_literal("Top secret author note"));
        let annot_id = doc.add_object(Object::Dictionary(annot_dict));

        let pages = doc.get_pages();
        let page_id = *pages.get(&1).unwrap();
        if let Ok(p_dict) = doc.get_dictionary_mut(page_id) {
            p_dict.set("Annots", Object::Array(vec![Object::Reference(annot_id)]));
        }

        let mut with_annot = Vec::new();
        doc.save_to(&mut with_annot).unwrap();
        assert!(String::from_utf8_lossy(&with_annot).contains("Top secret author note"));

        // Redact overlapping region
        let redaction = PageRedaction::new(0, vec![PdfRect::new(90.0, 480.0, 200.0, 520.0)]);
        let redacted_bytes = redact_document_bytes(&with_annot, &[redaction]).unwrap();
        let redacted_str = String::from_utf8_lossy(&redacted_bytes);

        assert!(
            !redacted_str.contains("Top secret author note"),
            "Annotation in redacted region must be completely purged"
        );
    }

    #[test]
    fn test_redaction_blackout_box_and_color() {
        let pdf_data = create_test_pdf_with_text("Sample Text");
        let redaction = PageRedaction::new(0, vec![PdfRect::new(50.0, 400.0, 200.0, 450.0)])
            .with_fill_color((255, 0, 0)); // Pure Red

        let redacted_bytes = redact_document_bytes(&pdf_data, &[redaction]).unwrap();
        let redacted_doc = Document::load_mem(&redacted_bytes).unwrap();
        let pages = redacted_doc.get_pages();
        let page_id = *pages.get(&1).unwrap();
        let page_content = redacted_doc.get_page_content(page_id);
        let content_str = String::from_utf8_lossy(&page_content);

        // Check for fill color (1 0 0 rg) and rectangle fill (re f)
        assert!(
            content_str.contains("1 0 0 rg") || content_str.contains("1.0 0.0 0.0 rg"),
            "Red fill color must be present in content stream: {content_str}"
        );
        assert!(
            content_str.contains("re\nf") || content_str.contains("re f"),
            "Rectangle path and fill operator must be present in content stream"
        );
    }
}
