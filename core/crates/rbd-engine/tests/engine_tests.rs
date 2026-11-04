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
