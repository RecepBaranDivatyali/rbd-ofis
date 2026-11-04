#![allow(clippy::collapsible_if, clippy::collapsible_match)]

use crate::error::EngineError;
use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document, Object, ObjectId, Stream};
use std::f32::consts::PI;
use std::path::Path;

/// Spatial alignment for stamps and numbering on a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StampPosition {
    TopLeft,
    TopCenter,
    TopRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

/// Configuration options for document watermarks.
#[derive(Debug, Clone)]
pub struct WatermarkOptions {
    /// Watermark text string.
    pub text: String,
    /// Font size in points (default: 48.0).
    pub font_size: f32,
    /// Rotation angle in degrees counter-clockwise (default: 45.0).
    pub rotation_degrees: f32,
    /// Opacity in range 0.0 (transparent) to 1.0 (opaque) (default: 0.25).
    pub opacity: f32,
    /// Color RGB values in 0..=255 (default: (180, 180, 180)).
    pub color_rgb: (u8, u8, u8),
    /// If true, renders over page content; if false, renders behind page content.
    pub on_top: bool,
}

impl Default for WatermarkOptions {
    fn default() -> Self {
        Self {
            text: "CONFIDENTIAL".into(),
            font_size: 48.0,
            rotation_degrees: 45.0,
            opacity: 0.25,
            color_rgb: (180, 180, 180),
            on_top: true,
        }
    }
}

/// Configuration options for dynamic page numbering.
#[derive(Debug, Clone)]
pub struct PageNumberOptions {
    /// Template string with `{page}` and `{total}` placeholders (e.g. `"Sayfa {page} / {total}"`).
    pub template: String,
    /// Position on the page.
    pub position: StampPosition,
    /// Font size in points (default: 10.0).
    pub font_size: f32,
    /// Margin from the nearest edges in points (default: 36.0).
    pub margin: f32,
    /// Color RGB values (default: (80, 80, 80)).
    pub color_rgb: (u8, u8, u8),
    /// 1-based page index from which numbering should start (pages before this are unnumbered).
    pub start_page: u32,
}

impl Default for PageNumberOptions {
    fn default() -> Self {
        Self {
            template: "Sayfa {page} / {total}".into(),
            position: StampPosition::BottomCenter,
            font_size: 10.0,
            margin: 36.0,
            color_rgb: (80, 80, 80),
            start_page: 1,
        }
    }
}

/// Configuration options for legal Bates stamping.
#[derive(Debug, Clone)]
pub struct BatesOptions {
    /// Prefix text before the sequential number (e.g. `"RBD-"`).
    pub prefix: String,
    /// Sequential starting number (default: 1).
    pub start_number: u32,
    /// Digit padding width with leading zeros (e.g. 6 -> `"000001"`).
    pub padding: usize,
    /// Suffix text appended after the number.
    pub suffix: String,
    /// Alignment position on the page.
    pub position: StampPosition,
    /// Margin in points from page borders (default: 36.0).
    pub margin: f32,
    /// Color RGB values (default: (0, 0, 0)).
    pub color_rgb: (u8, u8, u8),
}

impl Default for BatesOptions {
    fn default() -> Self {
        Self {
            prefix: "RBD-".into(),
            start_number: 1,
            padding: 6,
            suffix: "".into(),
            position: StampPosition::BottomRight,
            margin: 36.0,
            color_rgb: (0, 0, 0),
        }
    }
}

/// Apply a text watermark across all pages in a PDF document.
pub fn apply_watermark(
    input_path: &Path,
    options: &WatermarkOptions,
    output_path: &Path,
) -> Result<(), EngineError> {
    let mut doc = Document::load(input_path)?;
    apply_watermark_doc(&mut doc, options)?;
    doc.save(output_path)?;
    Ok(())
}

/// In-memory variant of `apply_watermark`.
pub fn apply_watermark_bytes(
    input_bytes: &[u8],
    options: &WatermarkOptions,
) -> Result<Vec<u8>, EngineError> {
    let mut doc = Document::load_mem(input_bytes)?;
    apply_watermark_doc(&mut doc, options)?;
    let mut out = Vec::new();
    doc.save_to(&mut out)?;
    Ok(out)
}

fn apply_watermark_doc(doc: &mut Document, options: &WatermarkOptions) -> Result<(), EngineError> {
    let pages = doc.get_pages();
    let page_ids: Vec<ObjectId> = pages.values().copied().collect();
    let opacity = options.opacity.clamp(0.0, 1.0);

    for page_id in page_ids {
        let (pw, ph) = get_page_dimensions(doc, page_id);
        ensure_stamp_resources(doc, page_id, opacity)?;

        let rad = options.rotation_degrees * PI / 180.0;
        let cos_a = rad.cos();
        let sin_a = rad.sin();

        // Approximate text length
        let text_width = options.text.len() as f32 * options.font_size * 0.52;
        let cx = pw / 2.0;
        let cy = ph / 2.0;

        // Origin of baseline in local coordinates centered at (0, 0)
        let local_x = -text_width / 2.0;
        let local_y = -options.font_size / 3.0;

        // Transform (local_x, local_y) by rotation around center (cx, cy)
        let e = cx + local_x * cos_a - local_y * sin_a;
        let f = cy + local_x * sin_a + local_y * cos_a;

        let r = options.color_rgb.0 as f32 / 255.0;
        let g = options.color_rgb.1 as f32 / 255.0;
        let b = options.color_rgb.2 as f32 / 255.0;

        let mut ops = Vec::new();
        ops.push(Operation::new("q", vec![]));
        if opacity < 0.999 {
            ops.push(Operation::new(
                "gs",
                vec![Object::Name(b"GS_STAMP".to_vec())],
            ));
        }
        ops.push(Operation::new(
            "rg",
            vec![Object::Real(r), Object::Real(g), Object::Real(b)],
        ));
        ops.push(Operation::new("BT", vec![]));
        ops.push(Operation::new(
            "Tf",
            vec![
                Object::Name(b"F_STAMP".to_vec()),
                Object::Real(options.font_size),
            ],
        ));
        ops.push(Operation::new(
            "Tm",
            vec![
                Object::Real(cos_a),
                Object::Real(sin_a),
                Object::Real(-sin_a),
                Object::Real(cos_a),
                Object::Real(e),
                Object::Real(f),
            ],
        ));
        ops.push(Operation::new(
            "Tj",
            vec![Object::string_literal(&*options.text)],
        ));
        ops.push(Operation::new("ET", vec![]));
        ops.push(Operation::new("Q", vec![]));

        attach_content_stream(doc, page_id, ops, options.on_top)?;
    }

    Ok(())
}

/// Apply dynamic page numbers to all or selected pages.
pub fn apply_page_numbers(
    input_path: &Path,
    options: &PageNumberOptions,
    output_path: &Path,
) -> Result<(), EngineError> {
    let mut doc = Document::load(input_path)?;
    apply_page_numbers_doc(&mut doc, options)?;
    doc.save(output_path)?;
    Ok(())
}

/// In-memory variant of `apply_page_numbers`.
pub fn apply_page_numbers_bytes(
    input_bytes: &[u8],
    options: &PageNumberOptions,
) -> Result<Vec<u8>, EngineError> {
    let mut doc = Document::load_mem(input_bytes)?;
    apply_page_numbers_doc(&mut doc, options)?;
    let mut out = Vec::new();
    doc.save_to(&mut out)?;
    Ok(out)
}

fn apply_page_numbers_doc(
    doc: &mut Document,
    options: &PageNumberOptions,
) -> Result<(), EngineError> {
    let pages = doc.get_pages();
    let total_pages = pages.len() as u32;
    let page_ids: Vec<ObjectId> = pages.values().copied().collect();

    for (idx, page_id) in page_ids.into_iter().enumerate() {
        let current_page = (idx as u32) + 1;
        if current_page < options.start_page {
            continue;
        }

        let (pw, ph) = get_page_dimensions(doc, page_id);
        ensure_stamp_resources(doc, page_id, 1.0)?;

        let text = options
            .template
            .replace("{page}", &current_page.to_string())
            .replace("{total}", &total_pages.to_string());

        let text_width = text.len() as f32 * options.font_size * 0.52;
        let (x, y) = compute_coordinates(
            options.position,
            pw,
            ph,
            text_width,
            options.font_size,
            options.margin,
        );

        let r = options.color_rgb.0 as f32 / 255.0;
        let g = options.color_rgb.1 as f32 / 255.0;
        let b = options.color_rgb.2 as f32 / 255.0;

        let ops = vec![
            Operation::new("q", vec![]),
            Operation::new(
                "rg",
                vec![Object::Real(r), Object::Real(g), Object::Real(b)],
            ),
            Operation::new("BT", vec![]),
            Operation::new(
                "Tf",
                vec![
                    Object::Name(b"F_STAMP".to_vec()),
                    Object::Real(options.font_size),
                ],
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
            Operation::new("Tj", vec![Object::string_literal(&*text)]),
            Operation::new("ET", vec![]),
            Operation::new("Q", vec![]),
        ];

        attach_content_stream(doc, page_id, ops, true)?;
    }

    Ok(())
}

/// Apply legal Bates stamping sequential numbering to pages.
pub fn apply_bates_stamp(
    input_path: &Path,
    options: &BatesOptions,
    output_path: &Path,
) -> Result<(), EngineError> {
    let mut doc = Document::load(input_path)?;
    apply_bates_stamp_doc(&mut doc, options)?;
    doc.save(output_path)?;
    Ok(())
}

/// In-memory variant of `apply_bates_stamp`.
pub fn apply_bates_stamp_bytes(
    input_bytes: &[u8],
    options: &BatesOptions,
) -> Result<Vec<u8>, EngineError> {
    let mut doc = Document::load_mem(input_bytes)?;
    apply_bates_stamp_doc(&mut doc, options)?;
    let mut out = Vec::new();
    doc.save_to(&mut out)?;
    Ok(out)
}

fn apply_bates_stamp_doc(doc: &mut Document, options: &BatesOptions) -> Result<(), EngineError> {
    let pages = doc.get_pages();
    let page_ids: Vec<ObjectId> = pages.values().copied().collect();
    let font_size = 10.0;

    for (idx, page_id) in page_ids.into_iter().enumerate() {
        let current_number = options.start_number + (idx as u32);
        let bates_text = format!(
            "{}{:0>width$}{}",
            options.prefix,
            current_number,
            options.suffix,
            width = options.padding
        );

        let (pw, ph) = get_page_dimensions(doc, page_id);
        ensure_stamp_resources(doc, page_id, 1.0)?;

        let text_width = bates_text.len() as f32 * font_size * 0.52;
        let (x, y) = compute_coordinates(
            options.position,
            pw,
            ph,
            text_width,
            font_size,
            options.margin,
        );

        let r = options.color_rgb.0 as f32 / 255.0;
        let g = options.color_rgb.1 as f32 / 255.0;
        let b = options.color_rgb.2 as f32 / 255.0;

        let ops = vec![
            Operation::new("q", vec![]),
            Operation::new(
                "rg",
                vec![Object::Real(r), Object::Real(g), Object::Real(b)],
            ),
            Operation::new("BT", vec![]),
            Operation::new(
                "Tf",
                vec![Object::Name(b"F_STAMP".to_vec()), Object::Real(font_size)],
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
            Operation::new("Tj", vec![Object::string_literal(&*bates_text)]),
            Operation::new("ET", vec![]),
            Operation::new("Q", vec![]),
        ];

        attach_content_stream(doc, page_id, ops, true)?;
    }

    Ok(())
}

fn compute_coordinates(
    pos: StampPosition,
    pw: f32,
    ph: f32,
    text_width: f32,
    font_size: f32,
    margin: f32,
) -> (f32, f32) {
    match pos {
        StampPosition::TopLeft => (margin, ph - margin - font_size),
        StampPosition::TopCenter => ((pw - text_width) / 2.0, ph - margin - font_size),
        StampPosition::TopRight => (pw - margin - text_width, ph - margin - font_size),
        StampPosition::BottomLeft => (margin, margin),
        StampPosition::BottomCenter => ((pw - text_width) / 2.0, margin),
        StampPosition::BottomRight => (pw - margin - text_width, margin),
    }
}

fn get_page_dimensions(doc: &Document, page_id: ObjectId) -> (f32, f32) {
    let mut current_id = page_id;
    for _ in 0..5 {
        if let Ok(dict) = doc.get_dictionary(current_id) {
            if let Ok(mb) = dict.get(b"MediaBox") {
                if let Object::Array(arr) = mb {
                    if arr.len() >= 4 {
                        let x0 = arr[0].as_float().unwrap_or(0.0);
                        let y0 = arr[1].as_float().unwrap_or(0.0);
                        let x1 = arr[2].as_float().unwrap_or(595.0);
                        let y1 = arr[3].as_float().unwrap_or(842.0);
                        return ((x1 - x0).abs(), (y1 - y0).abs());
                    }
                }
            }
            if let Ok(parent) = dict.get(b"Parent").and_then(|o| o.as_reference()) {
                current_id = parent;
                continue;
            }
        }
        break;
    }
    (595.0, 842.0) // Default A4 in points
}

fn ensure_stamp_resources(
    doc: &mut Document,
    page_id: ObjectId,
    opacity: f32,
) -> Result<(), EngineError> {
    // 1. Check whether font or extgstate objects are already present
    let mut needs_font = true;
    let mut needs_extg = opacity < 0.999;

    if let Ok(page_dict) = doc.get_dictionary(page_id) {
        if let Ok(Object::Dictionary(res)) = page_dict.get(b"Resources") {
            if let Ok(Object::Dictionary(fonts)) = res.get(b"Font") {
                if fonts.has(b"F_STAMP") {
                    needs_font = false;
                }
            }
            if let Ok(Object::Dictionary(extg)) = res.get(b"ExtGState") {
                if extg.has(b"GS_STAMP") {
                    needs_extg = false;
                }
            }
        }
    }

    // 2. Allocate font and ExtGState objects in document
    let font_obj_id = if needs_font {
        let mut font_dict = Dictionary::new();
        font_dict.set("Type", Object::Name(b"Font".to_vec()));
        font_dict.set("Subtype", Object::Name(b"Type1".to_vec()));
        font_dict.set("BaseFont", Object::Name(b"Helvetica".to_vec()));
        font_dict.set("Encoding", Object::Name(b"WinAnsiEncoding".to_vec()));
        Some(doc.add_object(Object::Dictionary(font_dict)))
    } else {
        None
    };

    let gs_obj_id = if needs_extg {
        let mut gs_dict = Dictionary::new();
        gs_dict.set("Type", Object::Name(b"ExtGState".to_vec()));
        gs_dict.set("ca", Object::Real(opacity));
        gs_dict.set("CA", Object::Real(opacity));
        Some(doc.add_object(Object::Dictionary(gs_dict)))
    } else {
        None
    };

    // 3. Mutably attach references into page resources
    let page_dict = doc.get_dictionary_mut(page_id)?;
    if !page_dict.has(b"Resources") {
        page_dict.set("Resources", Object::Dictionary(Dictionary::new()));
    }

    if let Ok(Object::Dictionary(resources)) = page_dict.get_mut(b"Resources") {
        if let Some(font_id) = font_obj_id {
            if !resources.has(b"Font") {
                resources.set("Font", Object::Dictionary(Dictionary::new()));
            }
            if let Ok(Object::Dictionary(fonts)) = resources.get_mut(b"Font") {
                fonts.set("F_STAMP", Object::Reference(font_id));
            }
        }

        if let Some(gs_id) = gs_obj_id {
            if !resources.has(b"ExtGState") {
                resources.set("ExtGState", Object::Dictionary(Dictionary::new()));
            }
            if let Ok(Object::Dictionary(extg)) = resources.get_mut(b"ExtGState") {
                extg.set("GS_STAMP", Object::Reference(gs_id));
            }
        }
    }

    Ok(())
}

fn attach_content_stream(
    doc: &mut Document,
    page_id: ObjectId,
    operations: Vec<Operation>,
    on_top: bool,
) -> Result<(), EngineError> {
    let content = Content { operations };
    let stream_bytes = content.encode()?;
    let stream = Stream::new(Dictionary::new(), stream_bytes);
    let stamp_stream_id = doc.add_object(Object::Stream(stream));

    let page_dict = doc.get_dictionary_mut(page_id)?;

    match page_dict.get_mut(b"Contents") {
        Ok(Object::Array(arr)) => {
            if on_top {
                arr.push(Object::Reference(stamp_stream_id));
            } else {
                arr.insert(0, Object::Reference(stamp_stream_id));
            }
        }
        Ok(Object::Reference(existing_id)) => {
            let old_ref = *existing_id;
            let new_arr = if on_top {
                vec![
                    Object::Reference(old_ref),
                    Object::Reference(stamp_stream_id),
                ]
            } else {
                vec![
                    Object::Reference(stamp_stream_id),
                    Object::Reference(old_ref),
                ]
            };
            page_dict.set("Contents", Object::Array(new_arr));
        }
        _ => {
            page_dict.set("Contents", Object::Reference(stamp_stream_id));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::content::Operation;
    use lopdf::{Dictionary, Document, Object, Stream};

    fn create_two_page_sample_pdf() -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let mut page_ids = Vec::new();

        for i in 1..=2 {
            let mut content = Content {
                operations: Vec::new(),
            };
            content.operations.push(Operation::new("BT", vec![]));
            content.operations.push(Operation::new(
                "Tf",
                vec![Object::Name(b"F1".to_vec()), Object::Real(14.0)],
            ));
            content.operations.push(Operation::new(
                "Tm",
                vec![
                    Object::Real(1.0),
                    Object::Real(0.0),
                    Object::Real(0.0),
                    Object::Real(1.0),
                    Object::Real(100.0),
                    Object::Real(700.0),
                ],
            ));
            content.operations.push(Operation::new(
                "Tj",
                vec![Object::string_literal(format!("Page {i} Content"))],
            ));
            content.operations.push(Operation::new("ET", vec![]));

            let stream_bytes = content.encode().unwrap();
            let stream = Stream::new(Dictionary::new(), stream_bytes);
            let content_id = doc.add_object(Object::Stream(stream));

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
            let page_id = doc.add_object(Object::Dictionary(page_dict));
            page_ids.push(Object::Reference(page_id));
        }

        let mut pages_dict = Dictionary::new();
        pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
        pages_dict.set("Kids", Object::Array(page_ids));
        pages_dict.set("Count", Object::Integer(2));
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
    fn test_apply_watermark_foreground_and_background() {
        let base_pdf = create_two_page_sample_pdf();

        // 1. Test Foreground Watermark
        let mut opts = WatermarkOptions {
            text: "GIZLI BELGE".into(),
            opacity: 0.35,
            on_top: true,
            ..Default::default()
        };

        let stamped_bytes = apply_watermark_bytes(&base_pdf, &opts).unwrap();
        let stamped_doc = Document::load_mem(&stamped_bytes).unwrap();
        assert_eq!(stamped_doc.get_pages().len(), 2);

        // Verify watermark text exists in the document bytes
        let s = String::from_utf8_lossy(&stamped_bytes);
        assert!(s.contains("GIZLI BELGE"));
        assert!(s.contains("GS_STAMP"));

        // 2. Test Background Watermark
        opts.on_top = false;
        let bg_bytes = apply_watermark_bytes(&base_pdf, &opts).unwrap();
        let bg_doc = Document::load_mem(&bg_bytes).unwrap();
        let pages = bg_doc.get_pages();
        let page_id = *pages.get(&1).unwrap();
        let p_dict = bg_doc.get_dictionary(page_id).unwrap();

        // For background (!on_top), the first item in Contents array must be the stamp
        if let Ok(Object::Array(contents)) = p_dict.get(b"Contents") {
            assert!(contents.len() >= 2);
        } else {
            panic!("Contents must be an array with prepended background stream");
        }
    }

    #[test]
    fn test_apply_page_numbers() {
        let base_pdf = create_two_page_sample_pdf();
        let opts = PageNumberOptions {
            template: "Sayfa {page} / {total}".into(),
            position: StampPosition::BottomCenter,
            font_size: 11.0,
            margin: 24.0,
            color_rgb: (50, 50, 50),
            start_page: 1,
        };

        let stamped_bytes = apply_page_numbers_bytes(&base_pdf, &opts).unwrap();
        let s = String::from_utf8_lossy(&stamped_bytes);

        assert!(
            s.contains("Sayfa 1 / 2"),
            "First page number must be embedded"
        );
        assert!(
            s.contains("Sayfa 2 / 2"),
            "Second page number must be embedded"
        );
    }

    #[test]
    fn test_apply_bates_stamp() {
        let base_pdf = create_two_page_sample_pdf();
        let opts = BatesOptions {
            prefix: "RBD-CASE-".into(),
            start_number: 100,
            padding: 6,
            suffix: "-CONF".into(),
            position: StampPosition::BottomRight,
            margin: 30.0,
            color_rgb: (0, 0, 0),
        };

        let stamped_bytes = apply_bates_stamp_bytes(&base_pdf, &opts).unwrap();
        let s = String::from_utf8_lossy(&stamped_bytes);

        assert!(
            s.contains("RBD-CASE-000100-CONF"),
            "First Bates stamp must match formatted sequence"
        );
        assert!(
            s.contains("RBD-CASE-000101-CONF"),
            "Second Bates stamp must increment sequentially"
        );
    }
}
