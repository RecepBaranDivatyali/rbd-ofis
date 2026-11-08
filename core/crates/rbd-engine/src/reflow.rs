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
            .total_cmp(&a.bbox[3])
            .then_with(|| a.bbox[0].total_cmp(&b.bbox[0]))
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

    // 1. Ensure Helvetica font resource exists on page with Turkish WinAnsi Differences and ToUnicode CMap
    let font_obj_id = {
        let mut enc = Dictionary::new();
        enc.set("Type", Object::Name(b"Encoding".to_vec()));
        enc.set("BaseEncoding", Object::Name(b"WinAnsiEncoding".to_vec()));
        enc.set(
            "Differences",
            Object::Array(vec![
                Object::Integer(128),
                Object::Name(b"dotlessi".to_vec()),   // 0x80 -> ı
                Object::Name(b"Idotaccent".to_vec()), // 0x81 -> İ
                Object::Name(b"gbreve".to_vec()),     // 0x82 -> ğ
                Object::Name(b"Gbreve".to_vec()),     // 0x83 -> Ğ
                Object::Name(b"scedilla".to_vec()),   // 0x84 -> ş
                Object::Name(b"Scedilla".to_vec()),   // 0x85 -> Ş
            ]),
        );

        let to_unicode_stream = {
            let cmap_data = b"/CIDInit /ProcSet findresource begin\n\
12 dict begin\n\
begincmap\n\
/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
/CMapName /Custom-ToUnicode def\n\
/CMapType 2 def\n\
1 begincodespacerange\n\
<00> <FF>\n\
endcodespacerange\n\
2 beginbfrange\n\
<20> <7E> <0020>\n\
<A0> <FF> <00A0>\n\
endbfrange\n\
6 beginbfchar\n\
<80> <0131>\n\
<81> <0130>\n\
<82> <011F>\n\
<83> <011E>\n\
<84> <015F>\n\
<85> <015E>\n\
endbfchar\n\
endcmap\n\
CMapName currentdict /CMap defineresource pop\n\
end\n\
end\n";
            let stream = Stream::new(Dictionary::new(), cmap_data.to_vec());
            doc.add_object(Object::Stream(stream))
        };

        let mut helv = Dictionary::new();
        helv.set("Type", Object::Name(b"Font".to_vec()));
        helv.set("Subtype", Object::Name(b"Type1".to_vec()));
        helv.set("BaseFont", Object::Name(b"Helvetica".to_vec()));
        helv.set("Encoding", Object::Dictionary(enc));
        helv.set("ToUnicode", Object::Reference(to_unicode_stream));
        doc.add_object(Object::Dictionary(helv))
    };

    // Attach F_REFLOW font into page resources (handling both direct and indirect dictionaries)
    let res_obj_id = match doc.get_dictionary(page_id)?.get(b"Resources") {
        Ok(Object::Reference(id)) => Some(*id),
        _ => None,
    };

    let font_dict_id = match res_obj_id {
        Some(res_id) => {
            let res_dict = doc.get_dictionary(res_id)?;
            match res_dict.get(b"Font") {
                Ok(Object::Reference(f_id)) => Some(*f_id),
                _ => None,
            }
        }
        None => {
            let page_dict = doc.get_dictionary(page_id)?;
            if let Ok(resources) = page_dict.get(b"Resources") {
                if let Ok(res_dict) = resources.as_dict() {
                    match res_dict.get(b"Font") {
                        Ok(Object::Reference(f_id)) => Some(*f_id),
                        _ => None,
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }
    };

    if let Some(f_id) = font_dict_id {
        let fonts = doc.get_dictionary_mut(f_id)?;
        fonts.set("F_REFLOW", Object::Reference(font_obj_id));
    } else if let Some(res_id) = res_obj_id {
        let res_dict = doc.get_dictionary_mut(res_id)?;
        if !res_dict.has(b"Font") {
            res_dict.set("Font", Dictionary::new());
        }
        let fonts = res_dict.get_mut(b"Font")?.as_dict_mut()?;
        fonts.set("F_REFLOW", Object::Reference(font_obj_id));
    } else {
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
    }

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

pub(crate) fn sanitize_text(input: &str) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(input.len());
    for c in input.chars() {
        match c {
            // Turkish characters mapped via custom /Differences in Helvetica Type1:
            'ı' => bytes.push(0x80),
            'İ' => bytes.push(0x81),
            'ğ' => bytes.push(0x82),
            'Ğ' => bytes.push(0x83),
            'ş' | 'ș' => bytes.push(0x84),
            'Ş' | 'Ș' => bytes.push(0x85),
            // WinAnsiEncoding (Windows-1252) Turkish characters encoded with exact single-byte codes:
            'ç' => bytes.push(0xE7),
            'Ç' => bytes.push(0xC7),
            'ö' => bytes.push(0xF6),
            'Ö' => bytes.push(0xD6),
            'ü' => bytes.push(0xFC),
            'Ü' => bytes.push(0xDC),
            // Circumflex vowels present in WinAnsi:
            'â' => bytes.push(0xE2),
            'Â' => bytes.push(0xC2),
            'î' => bytes.push(0xEE),
            'Î' => bytes.push(0xCE),
            'û' => bytes.push(0xFB),
            'Û' => bytes.push(0xDB),
            // Romanian comma-below t/T mapped to similar:
            'ț' => bytes.push(b't'),
            'Ț' => bytes.push(b'T'),
            // Common typographic symbols
            '“' | '”' => bytes.push(b'"'),
            '‘' | '’' => bytes.push(b'\''),
            '—' | '–' => bytes.push(b'-'),
            '•' => bytes.push(b'*'),
            '…' => bytes.extend_from_slice(b"..."),
            c if c.is_ascii() => bytes.push(c as u8),
            // Latin-1 Supplement (0xA0..=0xFF) matches WinAnsiEncoding directly:
            c if (c as u32) >= 0x00A0 && (c as u32) <= 0x00FF => bytes.push((c as u32) as u8),
            // Fallback safely to space (never '?')
            _ => bytes.push(b' '),
        }
    }
    bytes
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
    fn test_cluster_lines_with_nan_bbox_total_cmp_regression() {
        let lines = vec![
            TextLineSegment {
                text: "Line with normal bbox".into(),
                bbox: [72.0, 700.0, 300.0, 712.0],
                font_size: 12.0,
            },
            TextLineSegment {
                text: "Line with NaN bbox".into(),
                bbox: [f32::NAN, 684.0, 250.0, f32::NAN],
                font_size: 12.0,
            },
        ];

        // Must not panic when sorting with NaN coordinates
        let paras = cluster_lines_into_paragraphs(0, &lines);
        assert!(!paras.is_empty());
    }

    #[test]
    fn test_reflow_text_to_width() {
        let text = "Bu uzun bir paragraf metnidir ve belirtilen genişliğe göre kelime kelime alt satırlara bölünmelidir.";
        let lines = reflow_text_to_width(text, 150.0, 11.0);
        assert!(lines.len() >= 3);
        let joined = lines.join(" ");
        assert_eq!(joined, text);
    }

    #[test]
    fn test_sanitize_text_turkish_characters_regression() {
        let input = "Türkçe ışık Ğöçü İŞ!";
        let encoded = sanitize_text(input);

        // WinAnsi standard:
        // 'ü' => 0xFC, 'ç' => 0xE7, 'ö' => 0xF6, 'Ü' => 0xDC
        // Custom Differences mapped (no ASCII fallback):
        // 'ı' => 0x80, 'İ' => 0x81, 'ğ' => 0x82, 'Ğ' => 0x83, 'ş' => 0x84, 'Ş' => 0x85
        let expected = vec![
            b'T', 0xFC, b'r', b'k', 0xE7, b'e', b' ', 0x80, 0x84, 0x80, b'k', b' ', 0x83, 0xF6,
            0xE7, 0xFC, b' ', 0x81, 0x85, b'!',
        ];

        assert_eq!(encoded, expected);
        assert!(
            !encoded.contains(&b'?'),
            "Must never produce '?' for Turkish text"
        );
    }

    #[test]
    fn test_sanitize_text_all_turkish_and_extended_characters() {
        // Test all 6 pairs of Turkish special letters:
        // (ı, İ), (ğ, Ğ), (ş, Ş), (ç, Ç), (ö, Ö), (ü, Ü)
        let input = "ıİğĞşŞçÇöÖüÜ âÂîÎûÛ șȘ țȚ “test” – 100…";
        let encoded = sanitize_text(input);

        assert!(!encoded.contains(&b'?'));
        // Verify custom Differences slots:
        assert_eq!(encoded[0], 0x80); // ı
        assert_eq!(encoded[1], 0x81); // İ
        assert_eq!(encoded[2], 0x82); // ğ
        assert_eq!(encoded[3], 0x83); // Ğ
        assert_eq!(encoded[4], 0x84); // ş
        assert_eq!(encoded[5], 0x85); // Ş

        // Verify standard WinAnsi slots:
        assert_eq!(encoded[6], 0xE7); // ç
        assert_eq!(encoded[7], 0xC7); // Ç
        assert_eq!(encoded[8], 0xF6); // ö
        assert_eq!(encoded[9], 0xD6); // Ö
        assert_eq!(encoded[10], 0xFC); // ü
        assert_eq!(encoded[11], 0xDC); // Ü

        // Verify Romanian comma-below mappings:
        assert_eq!(encoded[20], 0x84); // ș -> 0x84
        assert_eq!(encoded[21], 0x85); // Ș -> 0x85
        assert_eq!(encoded[23], b't'); // ț -> 't'
        assert_eq!(encoded[24], b'T'); // Ț -> 'T'
    }

    #[test]
    fn test_replace_paragraph_text_doc_structure_and_font_encoding() {
        let mut doc = Document::with_version("1.7");
        let pages_id = doc.new_object_id();
        let page_id = doc.new_object_id();

        let mut page_dict = Dictionary::new();
        page_dict.set("Type", Object::Name(b"Page".to_vec()));
        page_dict.set("Parent", Object::Reference(pages_id));
        page_dict.set(
            "MediaBox",
            Object::Array(vec![
                Object::Integer(0),
                Object::Integer(0),
                Object::Integer(612),
                Object::Integer(792),
            ]),
        );
        doc.objects.insert(page_id, Object::Dictionary(page_dict));

        let mut pages_dict = Dictionary::new();
        pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
        pages_dict.set("Count", Object::Integer(1));
        pages_dict.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
        doc.objects.insert(pages_id, Object::Dictionary(pages_dict));

        let mut catalog = Dictionary::new();
        catalog.set("Type", Object::Name(b"Catalog".to_vec()));
        catalog.set("Pages", Object::Reference(pages_id));
        let catalog_id = doc.add_object(Object::Dictionary(catalog));
        doc.trailer.set("Root", Object::Reference(catalog_id));

        // Call replace_paragraph_text_doc
        let res = replace_paragraph_text_doc(
            &mut doc,
            0,
            [50.0, 500.0, 300.0, 600.0],
            "Yeni Türkçe Başlık: Çağdaş Işık Şelalesi",
            14.0,
        );
        assert!(res.is_ok());

        // Verify doc serializes and reloads with lopdf
        let mut bytes = Vec::new();
        doc.save_to(&mut bytes).expect("Save to buffer");
        let loaded = Document::load_mem(&bytes).expect("Reload with lopdf");

        // Find Helvetica font and inspect /Encoding and /ToUnicode
        let mut found_font = false;
        for obj in loaded.objects.values() {
            if let Object::Dictionary(d) = obj {
                if let Ok(subtype) = d.get(b"Subtype") {
                    if subtype == &Object::Name(b"Type1".to_vec()) {
                        found_font = true;
                        assert_eq!(
                            d.get(b"BaseFont").unwrap(),
                            &Object::Name(b"Helvetica".to_vec())
                        );
                        assert!(d.has(b"Encoding"), "Must have Encoding dictionary");
                        assert!(d.has(b"ToUnicode"), "Must have ToUnicode CMap stream");

                        let enc = d.get(b"Encoding").unwrap().as_dict().unwrap();
                        assert_eq!(
                            enc.get(b"BaseEncoding").unwrap(),
                            &Object::Name(b"WinAnsiEncoding".to_vec())
                        );
                        let diffs = enc.get(b"Differences").unwrap().as_array().unwrap();
                        assert_eq!(diffs[0], Object::Integer(128));
                        assert_eq!(diffs[1], Object::Name(b"dotlessi".to_vec()));
                        assert_eq!(diffs[2], Object::Name(b"Idotaccent".to_vec()));
                        assert_eq!(diffs[3], Object::Name(b"gbreve".to_vec()));
                        assert_eq!(diffs[4], Object::Name(b"Gbreve".to_vec()));
                        assert_eq!(diffs[5], Object::Name(b"scedilla".to_vec()));
                        assert_eq!(diffs[6], Object::Name(b"Scedilla".to_vec()));
                    }
                }
            }
        }
        assert!(found_font, "Must have added Helvetica Type1 font object");
    }
}
