use crate::error::EngineError;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Cursor, Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OfficeDocType {
    Docx,
    Xlsx,
    Pptx,
    Unknown,
}

impl OfficeDocType {
    pub fn as_str(&self) -> &'static str {
        match self {
            OfficeDocType::Docx => "DOCX",
            OfficeDocType::Xlsx => "XLSX",
            OfficeDocType::Pptx => "PPTX",
            OfficeDocType::Unknown => "UNKNOWN",
        }
    }

    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "docx" | "doc" => OfficeDocType::Docx,
            "xlsx" | "xls" => OfficeDocType::Xlsx,
            "pptx" | "ppt" => OfficeDocType::Pptx,
            _ => OfficeDocType::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfficeDocumentInfo {
    pub doc_type: String,
    pub title: String,
    pub word_count: usize,
    pub item_count: usize, // paragraphs, rows or slides
    pub summary: String,
}

/// Helper function to extract plain text between XML tags
pub fn extract_xml_text(xml: &str, tag_names: &[&str]) -> Vec<String> {
    let mut results = Vec::new();
    for &tag in tag_names {
        let open_tag = format!("<{}", tag);
        let close_tag = format!("</{}>", tag);

        let mut pos = 0;
        while let Some(start_idx) = xml[pos..].find(&open_tag) {
            let abs_start = pos + start_idx;
            let after_tag_name = abs_start + open_tag.len();

            // Verify that this is the exact tag name and not a prefix of another tag (e.g. <tbl vs <t)
            if let Some(next_char) = xml[after_tag_name..].chars().next()
                && !next_char.is_whitespace()
                && next_char != '>'
                && next_char != '/'
            {
                pos = after_tag_name;
                continue;
            }

            // Find end of opening/self-closing tag '>'
            if let Some(tag_end_offset) = xml[abs_start..].find('>') {
                let tag_end = abs_start + tag_end_offset;
                let tag_header = &xml[abs_start..tag_end];

                // If tag is self-closing (e.g. <t/>, <t xml:space="preserve"/>)
                if tag_header.trim_end().ends_with('/') {
                    pos = tag_end + 1;
                    continue;
                }

                let content_start = tag_end + 1;
                if let Some(end_idx) = xml[content_start..].find(&close_tag) {
                    let content_end = content_start + end_idx;
                    let text = &xml[content_start..content_end];
                    if !text.is_empty() {
                        results.push(decode_xml_entities(text));
                    }
                    pos = content_end + close_tag.len();
                } else {
                    pos = content_start;
                }
            } else {
                pos = abs_start + open_tag.len();
            }
        }
    }
    results
}

/// Decode standard XML entities
fn decode_xml_entities(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

/// Extract text from DOCX `word/document.xml`
fn extract_docx_text(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
) -> Result<(String, usize, usize), EngineError> {
    let mut file = archive
        .by_name("word/document.xml")
        .map_err(|e| EngineError::Corrupt(format!("Missing word/document.xml in DOCX: {}", e)))?;
    let mut xml = String::new();
    file.read_to_string(&mut xml)
        .map_err(|e| EngineError::Corrupt(format!("Failed to read word/document.xml: {}", e)))?;

    let mut paragraphs = Vec::new();
    let mut current_p = String::new();
    let mut in_p = false;

    // Scan tags: <w:p> opens paragraph, </w:p> closes, <w:t ...> contains text, <w:tab/> tab, <w:br/> newline
    let mut i = 0;
    let bytes = xml.as_bytes();
    while i < bytes.len() {
        if bytes[i] == b'<' {
            // Find '>'
            if let Some(end) = xml[i..].find('>') {
                let tag = &xml[i + 1..i + end];
                if tag.starts_with("w:p") && !tag.starts_with("w:pPr") {
                    in_p = true;
                    current_p.clear();
                } else if tag == "/w:p" {
                    if in_p {
                        let trimmed = current_p.trim();
                        if !trimmed.is_empty() {
                            paragraphs.push(trimmed.to_string());
                        }
                        in_p = false;
                        current_p.clear();
                    }
                } else if tag.starts_with("w:t") {
                    if tag.trim_end().ends_with('/') {
                        i += end + 1;
                        continue;
                    }
                    let text_start = i + end + 1;
                    if let Some(close) = xml[text_start..].find("</w:t>") {
                        let text = &xml[text_start..text_start + close];
                        current_p.push_str(&decode_xml_entities(text));
                        i = text_start + close + "</w:t>".len();
                        continue;
                    }
                } else if tag.starts_with("w:tab") {
                    current_p.push('\t');
                } else if tag.starts_with("w:br") {
                    current_p.push('\n');
                }
                i += end + 1;
                continue;
            }
        }
        i += 1;
    }

    if !current_p.trim().is_empty() {
        paragraphs.push(current_p.trim().to_string());
    }

    let full_text = paragraphs.join("\n\n");
    let word_count = full_text.split_whitespace().count();
    let paragraph_count = paragraphs.len();

    Ok((full_text, word_count, paragraph_count))
}

/// Extract text from XLSX `xl/sharedStrings.xml` & `xl/worksheets/sheet1.xml`
fn extract_xlsx_text(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
) -> Result<(String, usize, usize), EngineError> {
    // 1. Shared strings
    let mut shared_strings = Vec::new();
    if let Ok(mut ss_file) = archive.by_name("xl/sharedStrings.xml") {
        let mut ss_xml = String::new();
        if ss_file.read_to_string(&mut ss_xml).is_ok() {
            shared_strings = extract_xml_text(&ss_xml, &["t", "w:t"]);
        }
    }

    // 2. Worksheets: scan all worksheets matching xl/worksheets/sheet*.xml in sequential natural order
    let mut sheet_rows = Vec::new();
    let mut sheet_names: Vec<String> = archive
        .file_names()
        .filter_map(|res| res.ok())
        .filter(|name| name.starts_with("xl/worksheets/sheet") && name.ends_with(".xml"))
        .map(|name| name.to_string())
        .collect();

    sheet_names.sort_by(|a, b| {
        let num_a = a
            .strip_prefix("xl/worksheets/sheet")
            .and_then(|s| s.strip_suffix(".xml"))
            .and_then(|s| s.parse::<u32>().ok());
        let num_b = b
            .strip_prefix("xl/worksheets/sheet")
            .and_then(|s| s.strip_suffix(".xml"))
            .and_then(|s| s.parse::<u32>().ok());
        match (num_a, num_b) {
            (Some(na), Some(nb)) => na.cmp(&nb),
            _ => a.cmp(b),
        }
    });

    for sheet_name in sheet_names {
        let mut sheet_file = match archive.by_name(&sheet_name) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let mut sheet_xml = String::new();
        if sheet_file.read_to_string(&mut sheet_xml).is_ok() {
            // Parse rows <row ...> and cells <c ...>
            let mut pos = 0;
            while let Some(row_start) = sheet_xml[pos..].find("<row") {
                let abs_row_start = pos + row_start;
                if let Some(row_end) = sheet_xml[abs_row_start..].find("</row>") {
                    let row_content = &sheet_xml[abs_row_start..abs_row_start + row_end];
                    let mut row_cells = Vec::new();

                    // Find cells inside row
                    let mut cell_pos = 0;
                    while let Some(c_start) = row_content[cell_pos..].find("<c") {
                        let abs_c_start = cell_pos + c_start;
                        if let Some(c_tag_end) = row_content[abs_c_start..].find('>') {
                            let open_c = &row_content[abs_c_start..abs_c_start + c_tag_end];
                            let is_shared = open_c.contains("t=\"s\"");
                            let is_inline = open_c.contains("t=\"inlineStr\"");

                            if let Some(c_close) = row_content[abs_c_start..].find("</c>") {
                                let c_body = &row_content
                                    [abs_c_start + c_tag_end + 1..abs_c_start + c_close];
                                if is_shared {
                                    if let Some(v_start) = c_body.find("<v>")
                                        && let Some(v_end) = c_body[v_start + 3..].find("</v>")
                                    {
                                        let idx_str = &c_body[v_start + 3..v_start + 3 + v_end];
                                        if let Ok(idx) = idx_str.parse::<usize>()
                                            && let Some(s) = shared_strings.get(idx)
                                        {
                                            row_cells.push(s.clone());
                                        }
                                    }
                                } else if is_inline {
                                    let inlines = extract_xml_text(c_body, &["t"]);
                                    if !inlines.is_empty() {
                                        row_cells.push(inlines.join(""));
                                    }
                                } else if let Some(v_start) = c_body.find("<v>")
                                    && let Some(v_end) = c_body[v_start + 3..].find("</v>")
                                {
                                    let val = &c_body[v_start + 3..v_start + 3 + v_end];
                                    row_cells.push(decode_xml_entities(val));
                                }
                                cell_pos = abs_c_start + c_close + 4;
                            } else {
                                cell_pos = abs_c_start + c_tag_end + 1;
                            }
                        } else {
                            break;
                        }
                    }

                    if !row_cells.is_empty() {
                        sheet_rows.push(row_cells.join("\t"));
                    }
                    pos = abs_row_start + row_end + 6;
                } else {
                    break;
                }
            }
        }
    }

    let full_text = sheet_rows.join("\n");
    let word_count = full_text.split_whitespace().count();
    let row_count = sheet_rows.len();

    Ok((full_text, word_count, row_count))
}

/// Extract text from PPTX `ppt/slides/slide*.xml`
fn extract_pptx_text(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
) -> Result<(String, usize, usize), EngineError> {
    let mut slide_texts = Vec::new();
    let mut slide_idx: usize = 1;

    loop {
        let slide_name = format!("ppt/slides/slide{}.xml", slide_idx);
        let mut slide_file = match archive.by_name(&slide_name) {
            Ok(f) => f,
            Err(_) => break,
        };
        let mut slide_xml = String::new();
        if slide_file.read_to_string(&mut slide_xml).is_ok() {
            let texts = extract_xml_text(&slide_xml, &["a:t", "t"]);
            if !texts.is_empty() {
                let slide_header = format!("--- Slayt {} ---", slide_idx);
                let slide_body = texts.join(" ");
                slide_texts.push(format!("{}\n{}", slide_header, slide_body));
            }
        }
        slide_idx += 1;
    }

    let slide_count = slide_idx.saturating_sub(1);
    let full_text = slide_texts.join("\n\n");
    let word_count = full_text.split_whitespace().count();

    Ok((full_text, word_count, slide_count))
}

/// Inspect office document type and content from raw bytes
pub fn extract_office_text_from_bytes(
    bytes: &[u8],
) -> Result<(OfficeDocType, String, usize, usize), EngineError> {
    // Check for Microsoft Compound File Binary (OLE2 CFB) magic header.
    // Password-protected Office documents (DOCX, XLSX, PPTX) wrap an encrypted package
    // inside an OLE2 CFB container instead of an open ZIP archive.
    const OLE2_CFB_MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
    if bytes.starts_with(&OLE2_CFB_MAGIC) {
        return Err(EngineError::Encrypted(
            "Encrypted or password-protected Office document (OLE2 CFB wrapper)".to_string(),
        ));
    }

    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor)
        .map_err(|e| EngineError::Corrupt(format!("Not a valid ZIP/Office archive: {}", e)))?;

    // Determine type by examining archive contents
    let has_word = archive.by_name("word/document.xml").is_ok();
    let has_excel = archive.by_name("xl/workbook.xml").is_ok()
        || archive
            .file_names()
            .filter_map(|res| res.ok())
            .any(|name| name.starts_with("xl/worksheets/sheet") && name.ends_with(".xml"));
    let has_powerpoint = archive.by_name("ppt/presentation.xml").is_ok()
        || archive
            .file_names()
            .filter_map(|res| res.ok())
            .any(|name| name.starts_with("ppt/slides/slide") && name.ends_with(".xml"));

    if has_word {
        let (text, words, items) = extract_docx_text(&mut archive)?;
        Ok((OfficeDocType::Docx, text, words, items))
    } else if has_excel {
        let (text, words, items) = extract_xlsx_text(&mut archive)?;
        Ok((OfficeDocType::Xlsx, text, words, items))
    } else if has_powerpoint {
        let (text, words, items) = extract_pptx_text(&mut archive)?;
        Ok((OfficeDocType::Pptx, text, words, items))
    } else {
        Err(EngineError::Unsupported(
            "Unknown or unsupported Office format".to_string(),
        ))
    }
}

/// Extract text from Office file (DOCX, XLSX, PPTX)
pub fn extract_office_text(path: &Path) -> Result<String, EngineError> {
    let bytes = std::fs::read(path).map_err(EngineError::Io)?;
    let (_, text, _, _) = extract_office_text_from_bytes(&bytes)?;
    Ok(text)
}

/// Inspect Office file and return metadata
pub fn inspect_office_document(path: &Path) -> Result<OfficeDocumentInfo, EngineError> {
    let bytes = std::fs::read(path).map_err(EngineError::Io)?;
    let (doc_type, text, word_count, item_count) = extract_office_text_from_bytes(&bytes)?;
    let title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string();

    let summary = if text.len() > 200 {
        format!("{}...", &text[..200])
    } else {
        text
    };

    Ok(OfficeDocumentInfo {
        doc_type: doc_type.as_str().to_string(),
        title,
        word_count,
        item_count,
        summary,
    })
}

/// Create a minimal, standard-compliant DOCX file from raw text
pub fn create_simple_docx_bytes(text: &str) -> Result<Vec<u8>, EngineError> {
    let mut buffer = Vec::new();
    let cursor = Cursor::new(&mut buffer);
    let mut zip = ZipWriter::new(cursor);

    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);

    // 1. [Content_Types].xml
    zip.start_file("[Content_Types].xml", options)
        .map_err(|e| EngineError::Internal(format!("ZIP error: {}", e)))?;
    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#;
    zip.write_all(content_types.as_bytes())
        .map_err(EngineError::Io)?;

    // 2. _rels/.rels
    zip.start_file("_rels/.rels", options)
        .map_err(|e| EngineError::Internal(format!("ZIP error: {}", e)))?;
    let rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#;
    zip.write_all(rels.as_bytes()).map_err(EngineError::Io)?;

    // 3. word/document.xml
    zip.start_file("word/document.xml", options)
        .map_err(|e| EngineError::Internal(format!("ZIP error: {}", e)))?;

    let mut doc_xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
  <w:body>
"#,
    );

    for paragraph in text.split('\n') {
        let clean = paragraph
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        doc_xml.push_str("    <w:p><w:r><w:t>");
        doc_xml.push_str(&clean);
        doc_xml.push_str("</w:t></w:r></w:p>\n");
    }

    doc_xml.push_str(
        r#"  </w:body>
</w:document>"#,
    );

    zip.write_all(doc_xml.as_bytes()).map_err(EngineError::Io)?;

    zip.finish()
        .map_err(|e| EngineError::Internal(format!("ZIP finish error: {}", e)))?;

    Ok(buffer)
}

/// Create a minimal DOCX file on disk
pub fn create_simple_docx(text: &str, output_path: &Path) -> Result<(), EngineError> {
    let bytes = create_simple_docx_bytes(text)?;
    let mut file = File::create(output_path).map_err(EngineError::Io)?;
    file.write_all(&bytes).map_err(EngineError::Io)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_xml_text_self_closing_tags() {
        let xml = r#"
            <root>
                <t/>
                <t xml:space="preserve"/>
                <t>Valid Text 1</t>
                <t/>
                <t>Valid Text 2</t>
                <t />
            </root>
        "#;
        let texts = extract_xml_text(xml, &["t"]);
        assert_eq!(texts, vec!["Valid Text 1", "Valid Text 2"]);
    }

    #[test]
    fn test_extract_xml_text_tag_prefix_disambiguation() {
        let xml = r#"
            <sheetData>
                <tbl>Table content</tbl>
                <t>Cell text</t>
                <target>Target content</target>
                <t/>
                <t>Another text</t>
            </sheetData>
        "#;
        let texts = extract_xml_text(xml, &["t"]);
        assert_eq!(texts, vec!["Cell text", "Another text"]);
    }
}
