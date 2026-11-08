use rbd_engine::{EngineType, PixelFormat, create_engine};
use std::path::PathBuf;

fn get_corpus_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Manifest is in core/crates/rbd-engine
    manifest_dir.join("../../../corpus/files")
}

#[test]
fn test_pdfium_engine_open_and_render() {
    let engine = create_engine(EngineType::Pdfium).expect("PDFium engine should initialize");
    assert_eq!(engine.name(), "PDFium");

    let pdf_path = get_corpus_dir().join("pdf-basic/pdfium-hello-world.pdf");
    assert!(pdf_path.exists(), "pdf file must exist at {:?}", pdf_path);

    let doc = engine
        .open_from_file(&pdf_path, None)
        .expect("Open pdfium-hello-world.pdf");
    assert_eq!(doc.page_count().unwrap(), 1);

    let page = doc.page(0).expect("Get page 0");
    let (w_pt, h_pt) = page.dimensions_points().expect("Dimensions");
    assert!(w_pt > 0.0 && h_pt > 0.0);

    // Render full page at 100x100
    let mut buf = vec![0u8; 100 * 100 * 4];
    page.render_to_rgba(100, 100, &mut buf)
        .expect("Render to RGBA");

    let non_zero_count = buf.iter().filter(|&&b| b != 0).count();
    assert!(
        non_zero_count > 0,
        "Buffer should contain non-zero pixel data"
    );

    // Render 256x256 tile at 150 DPI
    let mut tile_buf = vec![0u8; 256 * 256 * 4];
    page.render_tile_to_rgba(0, 0, 256, 256, 150.0, &mut tile_buf)
        .expect("Render tile to RGBA");
    let tile_non_zero = tile_buf.iter().filter(|&&b| b != 0).count();
    assert!(
        tile_non_zero > 0,
        "Tile buffer should contain rendered pixels"
    );

    // Extract text
    let text = page.extract_text().expect("Extract text");
    assert!(text.contains("Hello, world!"), "Text was: {text}");
}

#[test]
fn test_hayro_pure_rust_engine_open_and_render() {
    let engine = create_engine(EngineType::PureRust).expect("Hayro engine should initialize");
    assert_eq!(engine.name(), "Hayro (Pure-Rust)");

    let pdf_path = get_corpus_dir().join("pdf-basic/pdfium-hello-world.pdf");
    assert!(pdf_path.exists(), "pdf file must exist at {:?}", pdf_path);

    let doc = engine
        .open_from_file(&pdf_path, None)
        .expect("Open pdfium-hello-world.pdf with Hayro");
    assert_eq!(doc.page_count().unwrap(), 1);

    let page = doc.page(0).expect("Get page 0");
    let (w_pt, h_pt) = page.dimensions_points().expect("Dimensions");
    assert!(w_pt > 0.0 && h_pt > 0.0);

    let mut buf = vec![0u8; 100 * 100 * 4];
    page.render_to_rgba(100, 100, &mut buf)
        .expect("Hayro render to RGBA");
    let non_zero = buf.iter().filter(|&&b| b != 0).count();
    assert!(non_zero > 0, "Hayro rendered non-zero pixels");

    let text = page.extract_text().expect("Hayro extract text");
    assert!(text.contains("Hello, world!"), "Text was: {text}");
}

#[test]
fn test_pdfium_and_hayro_pixel_format_channel_consistency() {
    let pdfium = create_engine(EngineType::Pdfium).expect("PDFium engine");
    let hayro = create_engine(EngineType::PureRust).expect("Hayro engine");

    let pdf_path = get_corpus_dir().join("pdf-basic/pdfium-hello-world.pdf");
    assert!(pdf_path.exists(), "pdf file must exist at {:?}", pdf_path);

    let doc_p = pdfium
        .open_from_file(&pdf_path, None)
        .expect("Open with PDFium");
    let doc_h = hayro
        .open_from_file(&pdf_path, None)
        .expect("Open with Hayro");

    let page_p = doc_p.page(0).expect("PDFium page 0");
    let page_h = doc_h.page(0).expect("Hayro page 0");

    let w = 64u32;
    let h = 64u32;
    let stride = (w * 4) as usize;

    // 1. Render in Bgra32
    let mut bgra_p = vec![0u8; stride * (h as usize)];
    let mut bgra_h = vec![0u8; stride * (h as usize)];

    page_p
        .render_tile_to_buffer(0, 0, w, h, 72.0, PixelFormat::Bgra32, stride, &mut bgra_p)
        .expect("PDFium render Bgra32");
    page_h
        .render_tile_to_buffer(0, 0, w, h, 72.0, PixelFormat::Bgra32, stride, &mut bgra_h)
        .expect("Hayro render Bgra32");

    // 2. Render in Rgba32
    let mut rgba_p = vec![0u8; stride * (h as usize)];
    let mut rgba_h = vec![0u8; stride * (h as usize)];

    page_p
        .render_tile_to_buffer(0, 0, w, h, 72.0, PixelFormat::Rgba32, stride, &mut rgba_p)
        .expect("PDFium render Rgba32");
    page_h
        .render_tile_to_buffer(0, 0, w, h, 72.0, PixelFormat::Rgba32, stride, &mut rgba_h)
        .expect("Hayro render Rgba32");

    // Verify self-consistency: in both engines, swapping channel 0 and 2 of Bgra32 must yield Rgba32!
    for (bgra, rgba) in bgra_p
        .as_chunks::<4>()
        .0
        .iter()
        .zip(rgba_p.as_chunks::<4>().0)
    {
        assert_eq!(bgra[0], rgba[2], "PDFium: B channel mismatch");
        assert_eq!(bgra[1], rgba[1], "PDFium: G channel mismatch");
        assert_eq!(bgra[2], rgba[0], "PDFium: R channel mismatch");
        assert_eq!(bgra[3], rgba[3], "PDFium: Alpha channel mismatch");
    }

    for (bgra, rgba) in bgra_h
        .as_chunks::<4>()
        .0
        .iter()
        .zip(rgba_h.as_chunks::<4>().0)
    {
        assert_eq!(bgra[0], rgba[2], "Hayro: B channel mismatch");
        assert_eq!(bgra[1], rgba[1], "Hayro: G channel mismatch");
        assert_eq!(bgra[2], rgba[0], "Hayro: R channel mismatch");
        assert_eq!(bgra[3], rgba[3], "Hayro: Alpha channel mismatch");
    }

    // Verify cross-engine channel alignment on non-empty white/black pixels
    // Both should render paper white (255, 255, 255, 255) for background
    assert_eq!(bgra_p[0..4], [255, 255, 255, 255]);
    assert_eq!(bgra_h[0..4], [255, 255, 255, 255]);
    assert_eq!(rgba_p[0..4], [255, 255, 255, 255]);
    assert_eq!(rgba_h[0..4], [255, 255, 255, 255]);
}

#[test]
fn test_turkish_character_extraction() {
    let engine = create_engine(EngineType::Pdfium).expect("PDFium engine");
    let pdf_path = get_corpus_dir().join("pdf-cjk-rtl-turkish/tr-generated-standard-fonts.pdf");
    assert!(pdf_path.exists(), "pdf file must exist at {:?}", pdf_path);

    let doc = engine
        .open_from_file(&pdf_path, None)
        .expect("Open turkish pdf");
    let page = doc.page(0).expect("Get page 0");
    let text = page.extract_text().expect("Extract text");

    // Verify Turkish characters presence
    let has_turkish = text.chars().any(|c| "İıŞşĞğÇçÖöÜü".contains(c));
    assert!(
        has_turkish,
        "Extracted text should preserve Turkish unicode characters. Sample text: {}",
        &text[..text.len().min(300)]
    );
}

#[test]
fn test_corrupt_pdf_handling() {
    let engine = create_engine(EngineType::Pdfium).expect("PDFium engine");
    let corrupt_path = get_corpus_dir().join("pdf-corrupt/corrupt-generated-truncated.pdf");
    assert!(
        corrupt_path.exists(),
        "pdf file must exist at {:?}",
        corrupt_path
    );

    let res = engine.open_from_file(&corrupt_path, None);
    assert!(res.is_err(), "Corrupt PDF must return error, not Ok");
}

#[test]
fn test_encrypted_pdf_handling() {
    let engine = create_engine(EngineType::Pdfium).expect("PDFium engine");
    let enc_path = get_corpus_dir().join("pdf-encrypted/pdfium-encrypted.pdf");
    assert!(enc_path.exists(), "pdf file must exist at {:?}", enc_path);

    let res = engine.open_from_file(&enc_path, None);
    assert!(
        res.is_err(),
        "Opening encrypted PDF without password must fail"
    );
}

#[test]
fn test_turkish_aware_search_with_bounding_boxes() {
    let engine = create_engine(EngineType::Pdfium).expect("PDFium engine");
    let pdf_path = get_corpus_dir().join("pdf-cjk-rtl-turkish/tr-generated-standard-fonts.pdf");
    assert!(pdf_path.exists(), "pdf file must exist at {:?}", pdf_path);

    let doc = engine
        .open_from_file(&pdf_path, None)
        .expect("Open turkish pdf");

    // Search query in lowercase: "şaşırtıcı"
    let hits_lower = doc
        .search_document("şaşırtıcı", false)
        .expect("Search lowercase");
    assert!(
        !hits_lower.is_empty(),
        "Should find matches for 'şaşırtıcı'"
    );

    // Search query in uppercase: "ŞAŞIRTICI"
    let hits_upper = doc
        .search_document("ŞAŞIRTICI", false)
        .expect("Search uppercase");
    assert_eq!(
        hits_lower.len(),
        hits_upper.len(),
        "Case-insensitive search should return same match count"
    );

    // Verify first hit has valid bounding box and snippet
    let first = &hits_lower[0];
    assert!(!first.snippet.is_empty(), "Snippet should not be empty");
    assert!(!first.rects.is_empty(), "Match should have bounding rects");
    assert!(first.rects[0].width() > 0.0, "Bounding box width > 0");
    assert!(first.rects[0].height() > 0.0, "Bounding box height > 0");

    // Pure-Rust Hayro search fallback
    let hayro_engine = create_engine(EngineType::PureRust).expect("Hayro engine");
    let hayro_doc = hayro_engine
        .open_from_file(&pdf_path, None)
        .expect("Open with Hayro");
    let hayro_hits = hayro_doc
        .search_document("Şaşırtıcı", false)
        .expect("Hayro search");
    assert!(!hayro_hits.is_empty(), "Hayro should also find hits");
    assert!(
        !hayro_hits[0].rects.is_empty(),
        "Hayro should provide fallback rect"
    );
}

#[test]
fn test_annotation_pdf_persistence_roundtrip() {
    use rbd_engine::annotation::{
        Annotation, AnnotationKind, list_annotations_from_pdf, save_annotations_to_pdf,
    };
    use rbd_engine::search::PdfRect;

    let in_path = get_corpus_dir().join("pdf-basic/pdfium-hello-world.pdf");
    assert!(in_path.exists(), "pdf file must exist");

    let temp_dir = std::env::temp_dir();
    let out_path = temp_dir.join(format!("test_annot_{}.pdf", std::process::id()));

    let annotations = vec![
        Annotation::new(
            1,
            0,
            AnnotationKind::Highlight,
            PdfRect::new(50.0, 100.0, 200.0, 120.0),
            [255, 235, 59, 200],
            "Important note".into(),
        ),
        Annotation::new(
            2,
            0,
            AnnotationKind::StickyNote,
            PdfRect::new(220.0, 100.0, 240.0, 120.0),
            [33, 150, 243, 255],
            "Check this paragraph".into(),
        ),
        Annotation::new(
            3,
            0,
            AnnotationKind::Rectangle,
            PdfRect::new(10.0, 10.0, 80.0, 50.0),
            [76, 175, 80, 255],
            "Green boundary box".into(),
        ),
    ];

    save_annotations_to_pdf(&in_path, &out_path, &annotations).expect("Save annotations to PDF");
    assert!(out_path.exists(), "Saved PDF must exist");

    let loaded = list_annotations_from_pdf(&out_path, 0).expect("Load annotations from saved PDF");
    assert_eq!(loaded.len(), 3, "All 3 annotations should be recovered");

    assert!(
        loaded
            .iter()
            .any(|a| matches!(a.kind, AnnotationKind::Highlight) && a.contents == "Important note")
    );
    assert!(
        loaded
            .iter()
            .any(|a| matches!(a.kind, AnnotationKind::StickyNote)
                && a.contents == "Check this paragraph")
    );
    assert!(
        loaded
            .iter()
            .any(|a| matches!(a.kind, AnnotationKind::Rectangle))
    );

    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn test_page_ops_reorder_crop_insert_blank() {
    use rbd_engine::page_ops::{crop_page, insert_blank_page, reorder_pages};

    let in_path = get_corpus_dir().join("pdf-basic/pdfium-hello-world.pdf");
    assert!(in_path.exists(), "pdf file must exist");

    let temp_dir = std::env::temp_dir();
    let out_p1 = temp_dir.join(format!("test_blank_{}.pdf", std::process::id()));
    let out_p2 = temp_dir.join(format!("test_crop_{}.pdf", std::process::id()));
    let out_p3 = temp_dir.join(format!("test_reorder_{}.pdf", std::process::id()));

    // 1. Insert blank page (1 page -> 2 pages)
    insert_blank_page(&in_path, &out_p1, 1, 595.0, 842.0).expect("Insert blank page");
    let doc1 = lopdf::Document::load(&out_p1).expect("Load doc1");
    assert_eq!(
        doc1.get_pages().len(),
        2,
        "Should have 2 pages after insert"
    );

    // 2. Crop page 1
    crop_page(&out_p1, &out_p2, 1, [10.0, 20.0, 150.0, 180.0]).expect("Crop page");
    let doc2 = lopdf::Document::load(&out_p2).expect("Load doc2");
    assert_eq!(doc2.get_pages().len(), 2);

    // 3. Reorder pages: page 2, then page 1
    reorder_pages(&out_p2, &out_p3, &[2, 1]).expect("Reorder pages");
    let doc3 = lopdf::Document::load(&out_p3).expect("Load doc3");
    assert_eq!(doc3.get_pages().len(), 2);

    let _ = std::fs::remove_file(&out_p1);
    let _ = std::fs::remove_file(&out_p2);
    let _ = std::fs::remove_file(&out_p3);
}

#[test]
fn test_decrypt_pdf_error_handling() {
    use rbd_engine::page_ops::decrypt_pdf;

    let enc_path = get_corpus_dir().join("pdf-encrypted/pdfium-encrypted.pdf");
    assert!(enc_path.exists());

    let temp_dir = std::env::temp_dir();
    let out_path = temp_dir.join(format!("test_dec_{}.pdf", std::process::id()));

    let res = decrypt_pdf(&enc_path, &out_path, "wrong_password_123");
    assert!(res.is_err(), "Decryption with invalid password must fail");
}

#[test]
fn test_phase4_permanent_redaction_end_to_end() {
    use rbd_engine::{PageRedaction, PdfRect, redact_document};

    let in_path = get_corpus_dir().join("pdf-basic/pdfium-hello-world.pdf");
    if !in_path.exists() {
        eprintln!("Skipping test: pdfium-hello-world.pdf not found");
        return;
    }

    let temp_dir = std::env::temp_dir();
    let out_path = temp_dir.join(format!("test_redacted_{}.pdf", std::process::id()));

    let redaction = PageRedaction::new(0, vec![PdfRect::new(50.0, 650.0, 350.0, 750.0)]);

    redact_document(&in_path, &[redaction], &out_path).expect("Redact document");
    assert!(out_path.exists());

    // Verify text is purged from document
    let raw_bytes = std::fs::read(&out_path).unwrap();
    let raw_str = String::from_utf8_lossy(&raw_bytes);
    assert!(
        !raw_str.contains("Hello, world!"),
        "Redacted document must not contain original text"
    );

    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn test_phase4_compression_end_to_end() {
    use rbd_engine::{CompressionOptions, compress_document};

    let in_path = get_corpus_dir().join("pdf-basic/pdfium-hello-world.pdf");
    if !in_path.exists() {
        eprintln!("Skipping test: pdfium-hello-world.pdf not found");
        return;
    }

    let temp_dir = std::env::temp_dir();
    let out_path = temp_dir.join(format!("test_compressed_{}.pdf", std::process::id()));

    let options = CompressionOptions::standard();
    let res = compress_document(&in_path, &options, &out_path).expect("Compress document");
    assert!(out_path.exists());
    assert!(res.compressed_size > 0);

    // Document must be valid and readable
    let doc = lopdf::Document::load(&out_path).expect("Load compressed doc");
    assert_eq!(doc.get_pages().len(), 1);

    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn test_phase4_watermark_page_number_bates_end_to_end() {
    use rbd_engine::{
        BatesOptions, PageNumberOptions, StampPosition, WatermarkOptions, apply_bates_stamp,
        apply_page_numbers, apply_watermark,
    };

    let in_path = get_corpus_dir().join("pdf-basic/pdfium-hello-world.pdf");
    if !in_path.exists() {
        eprintln!("Skipping test: pdfium-hello-world.pdf not found");
        return;
    }

    let temp_dir = std::env::temp_dir();
    let p_wm = temp_dir.join(format!("test_wm_{}.pdf", std::process::id()));
    let p_num = temp_dir.join(format!("test_num_{}.pdf", std::process::id()));
    let p_bates = temp_dir.join(format!("test_bates_{}.pdf", std::process::id()));

    // 1. Watermark
    let wm_opts = WatermarkOptions {
        text: "RBD OFIS PRO".into(),
        ..Default::default()
    };
    apply_watermark(&in_path, &wm_opts, &p_wm).expect("Apply watermark");

    // 2. Page Number
    let num_opts = PageNumberOptions {
        template: "Sayfa {page} / {total}".into(),
        position: StampPosition::BottomCenter,
        font_size: 10.0,
        margin: 20.0,
        color_rgb: (60, 60, 60),
        start_page: 1,
    };
    apply_page_numbers(&p_wm, &num_opts, &p_num).expect("Apply page numbers");

    // 3. Bates Stamp
    let bates_opts = BatesOptions {
        prefix: "RBD-E2E-".into(),
        start_number: 42,
        padding: 5,
        suffix: "".into(),
        position: StampPosition::TopRight,
        margin: 20.0,
        color_rgb: (0, 0, 0),
    };
    apply_bates_stamp(&p_num, &bates_opts, &p_bates).expect("Apply Bates stamp");

    let final_bytes = std::fs::read(&p_bates).unwrap();
    let final_str = String::from_utf8_lossy(&final_bytes);
    assert!(final_str.contains("RBD OFIS PRO"));
    assert!(final_str.contains("Sayfa 1 / 1"));
    assert!(final_str.contains("RBD-E2E-00042"));

    let _ = std::fs::remove_file(&p_wm);
    let _ = std::fs::remove_file(&p_num);
    let _ = std::fs::remove_file(&p_bates);
}

#[test]
fn test_phase4_form_filling_and_flattening_end_to_end() {
    use rbd_engine::{fill_form_fields, read_form_fields};
    use std::collections::HashMap;

    let in_path = get_corpus_dir().join("pdf-forms/pdfium-text-form.pdf");
    if !in_path.exists() {
        eprintln!("Skipping test: pdfium-text-form.pdf not found");
        return;
    }

    let temp_dir = std::env::temp_dir();
    let out_path = temp_dir.join(format!("test_flattened_form_{}.pdf", std::process::id()));

    let fields_before = read_form_fields(&in_path).expect("Read initial fields");
    assert!(!fields_before.is_empty());

    let mut updates = HashMap::new();
    updates.insert(
        fields_before[0].name.clone(),
        "EndToEndFormValue".to_string(),
    );

    fill_form_fields(&in_path, &updates, true, &out_path).expect("Fill and flatten");
    assert!(out_path.exists());

    // After flattening, interactive fields are removed
    let fields_after = read_form_fields(&out_path).expect("Read fields after flatten");
    assert!(
        fields_after.is_empty(),
        "Flattened doc should have no interactive fields"
    );

    // But text is baked
    let raw_bytes = std::fs::read(&out_path).unwrap();
    let raw_str = String::from_utf8_lossy(&raw_bytes);
    assert!(raw_str.contains("EndToEndFormValue"));

    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn test_reflow_turkish_characters_roundtrip_and_pdfium_render_extract() {
    let in_path = get_corpus_dir().join("pdf-basic/pdfium-hello-world.pdf");
    assert!(in_path.exists());

    let temp_dir = std::env::temp_dir();
    let out_path = temp_dir.join(format!("test_reflow_turkish_{}.pdf", std::process::id()));

    let turkish_text = "Türkçe ışık Ğöçü İŞ ve çağlayanlar";
    rbd_engine::replace_paragraph_text(
        &in_path,
        0,
        [20.0, 50.0, 180.0, 150.0],
        turkish_text,
        12.0,
        &out_path,
    )
    .expect("Replace paragraph text");

    // 1. Re-open with lopdf
    let lopdf_doc = lopdf::Document::load(&out_path).expect("lopdf reload");
    assert_eq!(lopdf_doc.get_pages().len(), 1);

    // 2. Open with PDFium
    let engine = create_engine(EngineType::Pdfium).expect("PDFium engine");
    let doc = engine.open_from_file(&out_path, None).expect("PDFium open");
    let page = doc.page(0).expect("Get page 0");

    // 3. Render to RGBA with PDFium
    let mut buf = vec![0u8; 100 * 100 * 4];
    page.render_to_rgba(100, 100, &mut buf)
        .expect("Render to RGBA");
    let non_zero_count = buf.iter().filter(|&&b| b != 0).count();
    assert!(
        non_zero_count > 0,
        "Buffer should contain non-zero pixel data"
    );

    // 4. Extract text & search with PDFium
    let extracted = page.extract_text().expect("Extract text");
    assert!(
        extracted.contains("Türkçe")
            && extracted.contains("ışık")
            && extracted.contains("Ğöçü")
            && extracted.contains("İŞ")
            && extracted.contains("çağlayanlar"),
        "Extracted text missing Turkish characters: {extracted}"
    );

    let search_all = page.search_text(0, "Türkçe", true).expect("Search Türkçe");
    assert!(!search_all.is_empty());
    let search_isik = page.search_text(0, "ışık", true).expect("Search ışık");
    assert!(!search_isik.is_empty());
    let search_is = page.search_text(0, "İŞ", true).expect("Search İŞ");
    assert!(!search_is.is_empty());
    let search_gocu = page.search_text(0, "Ğöçü", true).expect("Search Ğöçü");
    assert!(!search_gocu.is_empty());

    // 5. Open and render with Hayro (Pure-Rust engine)
    let hayro_engine = create_engine(EngineType::PureRust).expect("Hayro engine");
    let hayro_doc = hayro_engine
        .open_from_file(&out_path, None)
        .expect("Hayro open");
    let hayro_page = hayro_doc.page(0).expect("Hayro page 0");
    let mut hbuf = vec![0u8; 100 * 100 * 4];
    hayro_page
        .render_to_rgba(100, 100, &mut hbuf)
        .expect("Hayro render");
    let h_non_zero = hbuf.iter().filter(|&&b| b != 0).count();
    assert!(h_non_zero > 0, "Hayro rendered buffer should have pixels");

    let _ = std::fs::remove_file(&out_path);
}
