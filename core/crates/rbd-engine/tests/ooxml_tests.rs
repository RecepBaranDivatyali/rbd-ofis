use rbd_engine::ooxml::{
    OfficeDocType, create_simple_docx, create_simple_docx_bytes, extract_office_text,
    extract_office_text_from_bytes, inspect_office_document,
};
use std::io::{Cursor, Write};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

#[test]
fn test_create_and_extract_docx() {
    let sample_text =
        "RBD OFİS Ofis Düzenleyici\nİkinci Paragraf: Türkçe karakterler çşğüöı.\nÜçüncü paragraf.";
    let docx_bytes = create_simple_docx_bytes(sample_text).expect("Failed to create DOCX bytes");
    assert!(!docx_bytes.is_empty());

    let (doc_type, extracted, words, paragraphs) =
        extract_office_text_from_bytes(&docx_bytes).expect("Failed to extract DOCX text");

    assert_eq!(doc_type, OfficeDocType::Docx);
    assert_eq!(paragraphs, 3);
    assert!(extracted.contains("RBD OFİS Ofis Düzenleyici"));
    assert!(extracted.contains("Türkçe karakterler çşğüöı"));
    assert_eq!(words, 11);
}

#[test]
fn test_docx_file_on_disk_and_inspect() {
    let temp_dir = std::env::temp_dir().join(format!("rbd_ooxml_test_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);
    let file_path = temp_dir.join("test_doc.docx");

    let content = "Paragraf bir.\nParagraf iki.";
    create_simple_docx(content, &file_path).expect("Failed to write DOCX");
    assert!(file_path.exists());

    let extracted = extract_office_text(&file_path).expect("Failed to extract from file");
    assert_eq!(extracted, "Paragraf bir.\n\nParagraf iki.");

    let info = inspect_office_document(&file_path).expect("Failed to inspect");
    assert_eq!(info.doc_type, "DOCX");
    assert_eq!(info.item_count, 2);
    assert_eq!(info.word_count, 4);
    assert_eq!(info.title, "test_doc");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_extract_xlsx_mock() {
    let mut buffer = Vec::new();
    let cursor = Cursor::new(&mut buffer);
    let mut zip = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default();

    // Add sharedStrings.xml
    zip.start_file("xl/sharedStrings.xml", options).unwrap();
    let ss_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="2">
  <si><t>Ürün Adı</t></si>
  <si><t>Fiyat</t></si>
</sst>"#;
    zip.write_all(ss_xml.as_bytes()).unwrap();

    // Add sheet1.xml
    zip.start_file("xl/worksheets/sheet1.xml", options).unwrap();
    let sheet_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>
    <row r="1">
      <c r="A1" t="s"><v>0</v></c>
      <c r="B1" t="s"><v>1</v></c>
    </row>
    <row r="2">
      <c r="A2" t="inlineStr"><is><t>Defter</t></is></c>
      <c r="B2"><v>150</v></c>
    </row>
  </sheetData>
</worksheet>"#;
    zip.write_all(sheet_xml.as_bytes()).unwrap();

    zip.finish().unwrap();

    let (doc_type, extracted, words, rows) =
        extract_office_text_from_bytes(&buffer).expect("Failed to extract XLSX");

    assert_eq!(doc_type, OfficeDocType::Xlsx);
    assert_eq!(rows, 2);
    assert!(extracted.contains("Ürün Adı\tFiyat"));
    assert!(extracted.contains("Defter\t150"));
    assert_eq!(words, 5);
}

#[test]
fn test_extract_pptx_mock() {
    let mut buffer = Vec::new();
    let cursor = Cursor::new(&mut buffer);
    let mut zip = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default();

    // Add slide1.xml
    zip.start_file("ppt/slides/slide1.xml", options).unwrap();
    let slide1 = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:spTree>
      <p:sp><p:txBody><a:p><a:r><a:t>RBD OFİS Sunum</a:t></a:r></a:p></p:txBody></p:sp>
    </p:spTree>
  </p:cSld>
</p:sld>"#;
    zip.write_all(slide1.as_bytes()).unwrap();

    zip.finish().unwrap();

    let (doc_type, extracted, words, slides) =
        extract_office_text_from_bytes(&buffer).expect("Failed to extract PPTX");

    assert_eq!(doc_type, OfficeDocType::Pptx);
    assert_eq!(slides, 1);
    assert!(extracted.contains("--- Slayt 1 ---"));
    assert!(extracted.contains("RBD OFİS Sunum"));
    assert_eq!(words, 7); // "---", "Slayt", "1", "---", "RBD", "OFİS", "Sunum"
}
