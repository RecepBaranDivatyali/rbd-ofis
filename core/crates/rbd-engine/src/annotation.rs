use crate::error::EngineError;
use crate::search::PdfRect;
use lopdf::{Dictionary, Document, Object, ObjectId, StringFormat};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// High-level kind of PDF annotation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AnnotationKind {
    /// Text highlight markup.
    Highlight,
    /// Text underline markup.
    Underline,
    /// Text strikeout / strike-through markup.
    StrikeOut,
    /// Sticky note comment.
    StickyNote,
    /// Geometric rectangle border / box.
    Rectangle,
    /// Freehand drawing / ink stroke(s).
    FreehandInk {
        points: Vec<(f32, f32)>,
        stroke_width: f32,
    },
    /// Direct text typed on page.
    FreeText { font_size: f32 },
}

/// A document annotation with identity, geometry, color, and textual content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Annotation {
    pub id: u64,
    pub page_index: u32,
    pub kind: AnnotationKind,
    pub rect: PdfRect,
    pub color_rgba: [u8; 4],
    pub contents: String,
}

impl Annotation {
    /// Create a new annotation with given parameters.
    pub fn new(
        id: u64,
        page_index: u32,
        kind: AnnotationKind,
        rect: PdfRect,
        color_rgba: [u8; 4],
        contents: String,
    ) -> Self {
        Self {
            id,
            page_index,
            kind,
            rect,
            color_rgba,
            contents,
        }
    }
}

/// Save annotations into a PDF document at `output_path`, reading base structure from `input_path`.
/// Creates standard ISO 32000-1 `/Annot` dictionaries (/Highlight, /Text, /Square, /Ink, /FreeText).
pub fn save_annotations_to_pdf(
    input_path: &Path,
    output_path: &Path,
    annotations: &[Annotation],
) -> Result<(), EngineError> {
    let mut doc = Document::load(input_path)?;
    apply_annotations_to_doc(&mut doc, annotations)?;
    doc.save(output_path)?;
    Ok(())
}

/// Save annotations into in-memory PDF bytes.
pub fn save_annotations_to_memory(
    input_bytes: &[u8],
    annotations: &[Annotation],
) -> Result<Vec<u8>, EngineError> {
    let mut doc = Document::load_mem(input_bytes)?;
    apply_annotations_to_doc(&mut doc, annotations)?;
    let mut out = Vec::new();
    doc.save_to(&mut out)?;
    Ok(out)
}

fn apply_annotations_to_doc(
    doc: &mut Document,
    annotations: &[Annotation],
) -> Result<(), EngineError> {
    let pages = doc.get_pages();

    for annot in annotations {
        let page_num = annot.page_index + 1;
        let Some(&page_id) = pages.get(&page_num) else {
            continue;
        };

        let annot_dict = build_annot_dict(annot);
        let annot_obj_id = doc.add_object(Object::Dictionary(annot_dict));

        // Insert reference into page's /Annots array
        let page_dict = doc.get_dictionary_mut(page_id)?;
        if page_dict.has(b"Annots") {
            let annots_obj = page_dict.get_mut(b"Annots")?;
            match annots_obj {
                Object::Array(arr) => {
                    arr.push(Object::Reference(annot_obj_id));
                }
                Object::Reference(ref_id) => {
                    let target_id = *ref_id;
                    if let Ok(Object::Array(arr)) = doc.get_object_mut(target_id) {
                        arr.push(Object::Reference(annot_obj_id));
                    }
                }
                _ => {
                    page_dict.set(
                        "Annots",
                        Object::Array(vec![Object::Reference(annot_obj_id)]),
                    );
                }
            }
        } else {
            page_dict.set(
                "Annots",
                Object::Array(vec![Object::Reference(annot_obj_id)]),
            );
        }
    }

    Ok(())
}

fn build_annot_dict(annot: &Annotation) -> Dictionary {
    let mut dict = Dictionary::new();
    dict.set("Type", Object::Name(b"Annot".to_vec()));

    let (subtype, extra_entries) = match &annot.kind {
        AnnotationKind::Highlight => (b"Highlight".as_slice(), vec![]),
        AnnotationKind::Underline => (b"Underline".as_slice(), vec![]),
        AnnotationKind::StrikeOut => (b"StrikeOut".as_slice(), vec![]),
        AnnotationKind::StickyNote => (b"Text".as_slice(), vec![]),
        AnnotationKind::Rectangle => (
            b"Square".as_slice(),
            vec![(
                "BS",
                Object::Dictionary({
                    let mut bs = Dictionary::new();
                    bs.set("W", Object::Real(2.0));
                    bs
                }),
            )],
        ),
        AnnotationKind::FreehandInk {
            points,
            stroke_width,
        } => {
            let coords: Vec<Object> = points
                .iter()
                .flat_map(|(x, y)| vec![Object::Real(*x), Object::Real(*y)])
                .collect();
            let ink_list = Object::Array(vec![Object::Array(coords)]);
            let bs = Object::Dictionary({
                let mut bs_dict = Dictionary::new();
                bs_dict.set("W", Object::Real(*stroke_width));
                bs_dict
            });
            (b"Ink".as_slice(), vec![("InkList", ink_list), ("BS", bs)])
        }
        AnnotationKind::FreeText { font_size } => {
            let r = annot.color_rgba[0] as f32 / 255.0;
            let g = annot.color_rgba[1] as f32 / 255.0;
            let b = annot.color_rgba[2] as f32 / 255.0;
            let da = format!("/Helv {font_size} Tf {r:.2} {g:.2} {b:.2} rg");
            (
                b"FreeText".as_slice(),
                vec![("DA", Object::string_literal(da))],
            )
        }
    };

    dict.set("Subtype", Object::Name(subtype.to_vec()));

    // /Rect [x0 y0 x1 y1]
    let rect_array = vec![
        Object::Real(annot.rect.x0),
        Object::Real(annot.rect.y0),
        Object::Real(annot.rect.x1),
        Object::Real(annot.rect.y1),
    ];
    dict.set("Rect", Object::Array(rect_array));

    // /Contents
    if !annot.contents.is_empty() {
        dict.set(
            "Contents",
            Object::String(annot.contents.as_bytes().to_vec(), StringFormat::Literal),
        );
    }

    // /C [r g b] in 0.0..1.0
    let r = annot.color_rgba[0] as f32 / 255.0;
    let g = annot.color_rgba[1] as f32 / 255.0;
    let b = annot.color_rgba[2] as f32 / 255.0;
    let color_array = vec![Object::Real(r), Object::Real(g), Object::Real(b)];
    dict.set("C", Object::Array(color_array));

    // /CA opacity
    let alpha = annot.color_rgba[3] as f32 / 255.0;
    if alpha < 0.999 {
        dict.set("CA", Object::Real(alpha));
    }

    // Print flag (bit 3 = 4)
    dict.set("F", Object::Integer(4));

    // Add extra entries
    for (k, v) in extra_entries {
        dict.set(k, v);
    }

    dict
}

/// List existing annotations from a specific 0-indexed page in a PDF document.
pub fn list_annotations_from_doc(
    doc: &Document,
    page_index: u32,
) -> Result<Vec<Annotation>, EngineError> {
    let pages = doc.get_pages();
    let page_num = page_index + 1;
    let Some(&page_id) = pages.get(&page_num) else {
        return Ok(Vec::new());
    };

    let page_dict = doc.get_dictionary(page_id)?;
    let Ok(annots_obj) = page_dict.get(b"Annots") else {
        return Ok(Vec::new());
    };

    let annot_refs: Vec<ObjectId> = match annots_obj {
        Object::Array(arr) => arr
            .iter()
            .filter_map(|o| match o {
                Object::Reference(id) => Some(*id),
                _ => None,
            })
            .collect(),
        Object::Reference(id) => {
            if let Ok(Object::Array(arr)) = doc.get_object(*id) {
                arr.iter()
                    .filter_map(|o| match o {
                        Object::Reference(id) => Some(*id),
                        _ => None,
                    })
                    .collect()
            } else {
                Vec::new()
            }
        }
        _ => Vec::new(),
    };

    let mut annotations = Vec::new();
    let mut synthetic_id = ((page_index as u64) << 32) + 1;

    for obj_id in annot_refs {
        let Ok(dict) = doc.get_dictionary(obj_id) else {
            continue;
        };

        let subtype = dict
            .get(b"Subtype")
            .and_then(|o| o.as_name())
            .unwrap_or(b"");

        let kind = match subtype {
            b"Highlight" => AnnotationKind::Highlight,
            b"Underline" => AnnotationKind::Underline,
            b"StrikeOut" => AnnotationKind::StrikeOut,
            b"Text" => AnnotationKind::StickyNote,
            b"Square" => AnnotationKind::Rectangle,
            b"Ink" => {
                let mut points = Vec::new();
                if let Ok(ink_list) = dict.get(b"InkList").and_then(|o| o.as_array()) {
                    for path in ink_list {
                        if let Ok(coords) = path.as_array() {
                            let mut i = 0;
                            while i + 1 < coords.len() {
                                let x = coords[i].as_float().unwrap_or(0.0);
                                let y = coords[i + 1].as_float().unwrap_or(0.0);
                                points.push((x, y));
                                i += 2;
                            }
                        }
                    }
                }
                let stroke_width = dict
                    .get(b"BS")
                    .and_then(|o| o.as_dict())
                    .and_then(|d| d.get(b"W"))
                    .and_then(|w| w.as_float())
                    .unwrap_or(2.0);
                AnnotationKind::FreehandInk {
                    points,
                    stroke_width,
                }
            }
            b"FreeText" => {
                let font_size = dict
                    .get(b"DA")
                    .and_then(|o| o.as_str())
                    .ok()
                    .and_then(|da| {
                        // parse font size from DA string like "/Helv 12 Tf"
                        let s = String::from_utf8_lossy(da);
                        let parts: Vec<&str> = s.split_whitespace().collect();
                        parts.windows(2).find_map(|w| {
                            if w[1] == "Tf" {
                                w[0].parse::<f32>().ok()
                            } else {
                                None
                            }
                        })
                    })
                    .unwrap_or(12.0);
                AnnotationKind::FreeText { font_size }
            }
            _ => continue,
        };

        // Extract Rect: [x0 y0 x1 y1]
        let rect = if let Ok(rect_arr) = dict.get(b"Rect").and_then(|o| o.as_array())
            && rect_arr.len() >= 4
        {
            let x0 = rect_arr[0].as_float().unwrap_or(0.0);
            let y0 = rect_arr[1].as_float().unwrap_or(0.0);
            let x1 = rect_arr[2].as_float().unwrap_or(0.0);
            let y1 = rect_arr[3].as_float().unwrap_or(0.0);
            PdfRect::new(x0, y0, x1, y1)
        } else {
            PdfRect::ZERO
        };

        // Extract Contents
        let contents = dict
            .get(b"Contents")
            .and_then(|o| o.as_str())
            .ok()
            .map(|b| String::from_utf8_lossy(b).into_owned())
            .unwrap_or_default();

        // Extract Color /C [r g b] and /CA
        let mut color_rgba = [255, 235, 59, 255]; // default yellow
        if let Ok(c_arr) = dict.get(b"C").and_then(|o| o.as_array())
            && c_arr.len() >= 3
        {
            let r = (c_arr[0].as_float().unwrap_or(1.0) * 255.0).clamp(0.0, 255.0) as u8;
            let g = (c_arr[1].as_float().unwrap_or(1.0) * 255.0).clamp(0.0, 255.0) as u8;
            let b = (c_arr[2].as_float().unwrap_or(0.0) * 255.0).clamp(0.0, 255.0) as u8;
            let a = if let Ok(ca) = dict.get(b"CA").and_then(|o| o.as_float()) {
                (ca * 255.0).clamp(0.0, 255.0) as u8
            } else {
                255
            };
            color_rgba = [r, g, b, a];
        }

        annotations.push(Annotation {
            id: synthetic_id,
            page_index,
            kind,
            rect,
            color_rgba,
            contents,
        });
        synthetic_id += 1;
    }

    Ok(annotations)
}

/// List existing annotations from a PDF file on disk for the specified page.
pub fn list_annotations_from_pdf(
    input_path: &Path,
    page_index: u32,
) -> Result<Vec<Annotation>, EngineError> {
    let doc = Document::load(input_path)?;
    list_annotations_from_doc(&doc, page_index)
}
