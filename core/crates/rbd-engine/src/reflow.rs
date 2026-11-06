//! Fluid Paragraph Reflow & PDF Text Editing Engine.
//!
//! Solves the "Holy Grail" of PDF editing:
//! 1. Reconstructs structured, editable paragraphs from flat PDF text lines
//! 2. Reflows and wraps text dynamically within bounding boxes
//! 3. Replaces paragraph text in-place, rewriting content streams while preserving layout

#![allow(
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::vec_init_then_push
)]

use crate::error::EngineError;
use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document, Object, Stream};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Bounding rectangle in PDF points [x0, y0, x1, y1].
pub type BoundingBox = [f32; 4];

/// A detected fluid paragraph candidate within a PDF page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReflowParagraph {
    /// Unique identifier within the page.
    pub id: u32,
    /// 0-based page index.
    pub page_index: u32,
    /// Spatial boundary [x0, y0, x1, y1].
    pub bbox: BoundingBox,
    /// Combined multi-line text with normalized single spaces.
    pub text: String,
    /// Estimated font size in points.
    pub font_size: f32,
    /// Number of lines detected in this paragraph.
    pub line_count: usize,
}

/// Raw line segment candidate for paragraph reconstruction.
#[derive(Debug, Clone)]
pub struct TextLineSegment {
    pub text: String,
    pub bbox: BoundingBox,
    pub font_size: f32,
}

/// Group raw text lines into structured paragraphs based on proximity and line spacing.
pub fn cluster_lines_into_paragraphs(
    page_index: u32,
    lines: &[TextLineSegment],
) -> Vec<ReflowParagraph> {
    if lines.is_empty() {
        return Vec::new();
    }

    let mut sorted = lines.to_vec();
    // Sort primarily by Y descending (top to bottom), then by X ascending (left to right)
    sorted.sort_by(|a, b| {
        b.bbox[3]
            .partial_cmp(&a.bbox[3])
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.bbox[0].partial_cmp(&b.bbox[0]).unwrap())
    });

    let mut paragraphs: Vec<ReflowParagraph> = Vec::new();
    let mut current_lines: Vec<TextLineSegment> = Vec::new();
    let mut current_id = 1;

    for line in sorted {
        let trimmed = line.text.trim();
        if trimmed.is_empty() {
            continue;
        }

        if current_lines.is_empty() {
            current_lines.push(line);
            continue;
        }

        let last_line = current_lines.last().unwrap();
        let dy = (last_line.bbox[1] - line.bbox[3]).abs();
        let max_line_spacing = last_line.font_size * 2.2;

        // Same paragraph if vertical gap is reasonable and font sizes are similar
        let same_font = (last_line.font_size - line.font_size).abs() < 2.5;
        let is_adjacent = dy <= max_line_spacing;

        if same_font && is_adjacent {
            current_lines.push(line);
        } else {
            // Flush current paragraph
            paragraphs.push(build_paragraph(page_index, current_id, &current_lines));
            current_id += 1;
            current_lines.clear();
            current_lines.push(line);
        }
    }

    if !current_lines.is_empty() {
        paragraphs.push(build_paragraph(page_index, current_id, &current_lines));
    }

    paragraphs
}

fn build_paragraph(page_index: u32, id: u32, lines: &[TextLineSegment]) -> ReflowParagraph {
    let mut min_x = f32::MAX;
    let mut min_y = f32::MAX;
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;
    let mut texts = Vec::new();
    let mut avg_font_size = 0.0;

    for l in lines {
        min_x = min_x.min(l.bbox[0]);
        min_y = min_y.min(l.bbox[1]);
        max_x = max_x.max(l.bbox[2]);
        max_y = max_y.max(l.bbox[3]);
        texts.push(l.text.trim());
        avg_font_size += l.font_size;
    }

    avg_font_size /= (lines.len() as f32).max(1.0);

    ReflowParagraph {
        id,
        page_index,
        bbox: [min_x, min_y, max_x, max_y],
        text: texts.join(" "),
        font_size: (avg_font_size * 10.0).round() / 10.0,
        line_count: lines.len(),
    }
}

/// Wrap a string of words into lines fitting within `max_width` points.
pub fn reflow_text_to_width(text: &str, max_width: f32, font_size: f32) -> Vec<String> {
    let char_width = font_size * 0.52;
    let max_chars = ((max_width / char_width).floor() as usize).max(10);

    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() <= max_chars {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();
        }
    }

    if !current_line.is_empty() {
        lines.push(current_line);
    }

    lines
}

/// Replace a paragraph in a PDF file with new text, automatically reflowing lines.
pub fn replace_paragraph_text(
    input_path: &Path,
    page_index: u32,
    paragraph_bbox: BoundingBox,
    new_text: &str,
    font_size: f32,
    output_path: &Path,
) -> Result<(), EngineError> {
    let mut doc = Document::load(input_path)?;
    replace_paragraph_text_doc(&mut doc, page_index, paragraph_bbox, new_text, font_size)?;
    doc.save(output_path)?;
    Ok(())
}

/// In-memory implementation of paragraph replacement.
pub fn replace_paragraph_text_doc(
    doc: &mut Document,
    page_index: u32,
    bbox: BoundingBox,
    new_text: &str,
    font_size: f32,
) -> Result<(), EngineError> {
    let pages = doc.get_pages();
    let page_num = page_index + 1;
    let Some(&page_id) = pages.get(&page_num) else {
        return Err(EngineError::PageOutOfRange {
            index: page_index,
            total_pages: pages.len() as u32,
        });
    };

    // 1. Ensure Helvetica font resource exists on page
    let font_obj_id = {
        let mut helv = Dictionary::new();
        helv.set("Type", Object::Name(b"Font".to_vec()));
        helv.set("Subtype", Object::Name(b"Type1".to_vec()));
        helv.set("BaseFont", Object::Name(b"Helvetica".to_vec()));
        helv.set("Encoding", Object::Name(b"WinAnsiEncoding".to_vec()));
        doc.add_object(Object::Dictionary(helv))
    };

    let page_dict = doc.get_dictionary_mut(page_id)?;
    if !page_dict.has(b"Resources") {
        page_dict.set("Resources", Dictionary::new());
    }
    let resources = page_dict.get_mut(b"Resources")?.as_dict_mut()?;
    if !resources.has(b"Font") {
        resources.set("Font", Dictionary::new());
    }
    let fonts = resources.get_mut(b"Font")?.as_dict_mut()?;
    fonts.set("F_REFLOW", Object::Reference(font_obj_id));

    // 2. Reflow text into lines
    let width = (bbox[2] - bbox[0]).max(50.0);
    let wrapped_lines = reflow_text_to_width(new_text, width, font_size);
    let line_height = font_size * 1.35;

    // 3. Construct clean-up whiteout rect and reflowed text operations
    #[allow(clippy::vec_init_then_push)]
    let mut ops: Vec<Operation> = Vec::new();

    // Clean up original box (draw opaque white background mask over target bbox)
    ops.push(Operation::new("q", vec![]));
    ops.push(Operation::new(
        "rg",
        vec![Object::Real(1.0), Object::Real(1.0), Object::Real(1.0)],
    ));
    ops.push(Operation::new(
        "re",
        vec![
            Object::Real(bbox[0] - 2.0),
            Object::Real(bbox[1] - 2.0),
            Object::Real((bbox[2] - bbox[0]) + 4.0),
            Object::Real((bbox[3] - bbox[1]) + 4.0),
        ],
    ));
    ops.push(Operation::new("f", vec![]));
    ops.push(Operation::new("Q", vec![]));

    // Draw reflowed lines
    let start_x = bbox[0];
    let mut current_y = bbox[3] - font_size;

    ops.push(Operation::new("q", vec![]));
    ops.push(Operation::new(
        "rg",
        vec![Object::Real(0.0), Object::Real(0.0), Object::Real(0.0)],
    ));
    ops.push(Operation::new("BT", vec![]));
    ops.push(Operation::new(
        "Tf",
        vec![Object::Name(b"F_REFLOW".to_vec()), Object::Real(font_size)],
    ));

    for line in &wrapped_lines {
        ops.push(Operation::new(
            "Td",
            vec![Object::Real(start_x), Object::Real(current_y)],
        ));
        ops.push(Operation::new(
            "Tj",
            vec![Object::string_literal(sanitize_text(line))],
        ));
        ops.push(Operation::new(
            "Td",
            vec![Object::Real(-start_x), Object::Real(-current_y)],
        ));
        current_y -= line_height;
    }

    ops.push(Operation::new("ET", vec![]));
    ops.push(Operation::new("Q", vec![]));

    // 4. Append new content stream to page
    let content_bytes = Content { operations: ops }.encode()?;
    let stream_obj = Stream::new(Dictionary::new(), content_bytes);
    let stream_id = doc.add_object(Object::Stream(stream_obj));

    let page_dict = doc.get_dictionary_mut(page_id)?;
    if let Ok(contents) = page_dict.get_mut(b"Contents") {
        match contents {
            Object::Array(arr) => {
                arr.push(Object::Reference(stream_id));
            }
            Object::Reference(old_id) => {
                let old_ref = *old_id;
                *contents = Object::Array(vec![
                    Object::Reference(old_ref),
                    Object::Reference(stream_id),
                ]);
            }
            _ => {
                *contents = Object::Reference(stream_id);
            }
        }
    } else {
        page_dict.set("Contents", Object::Reference(stream_id));
    }

    Ok(())
}

fn sanitize_text(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            'ı' => 'i',
            'İ' => 'I',
            'ğ' => 'g',
            'Ğ' => 'G',
            'ş' => 's',
            'Ş' => 'S',
            'ç' => 'c',
            'Ç' => 'C',
            'ö' => 'o',
            'Ö' => 'O',
            'ü' => 'u',
            'Ü' => 'U',
            c if c.is_ascii() => c,
            _ => '?',
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cluster_lines_into_paragraph() {
        let lines = vec![
            TextLineSegment {
                text: "RBD OFİS yüksek performanslı bir".into(),
                bbox: [72.0, 700.0, 300.0, 712.0],
                font_size: 12.0,
            },
            TextLineSegment {
                text: "açık kaynak ofis paketidir.".into(),
                bbox: [72.0, 684.0, 250.0, 696.0],
                font_size: 12.0,
            },
            TextLineSegment {
                text: "Ayrı bir başlık metni".into(),
                bbox: [72.0, 600.0, 200.0, 616.0],
                font_size: 16.0,
            },
        ];

        let paras = cluster_lines_into_paragraphs(0, &lines);
        assert_eq!(paras.len(), 2);
        assert_eq!(paras[0].line_count, 2);
        assert!(paras[0].text.contains("RBD OFİS"));
        assert!(paras[0].text.contains("ofis paketidir"));
        assert_eq!(paras[1].line_count, 1);
    }

    #[test]
    fn test_reflow_text_to_width() {
        let text = "Bu uzun bir paragraf metnidir ve belirtilen genişliğe göre kelime kelime alt satırlara bölünmelidir.";
        let lines = reflow_text_to_width(text, 150.0, 11.0);
        assert!(lines.len() >= 3);
        let joined = lines.join(" ");
        assert_eq!(joined, text);
    }
}
