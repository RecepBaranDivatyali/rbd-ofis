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
    let pdf_path = get_corpus_dir().join("pdf-cjk-rtl-turkish/tr-vikipedi-kutuphane.pdf");
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
