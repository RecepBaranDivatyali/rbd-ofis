use rbd_engine::{
    ImageFormatType, ImageToPdfOptions, PageSize, PdfToImageOptions, create_default_engine,
    images_to_pdf, markdown_to_pdf, pdf_to_images,
};
use std::path::{Path, PathBuf};

fn get_corpus_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("../../../corpus/files")
}

fn create_temp_dir(prefix: &str) -> PathBuf {
    let temp = std::env::temp_dir().join(format!("rbd_test_{prefix}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp);
    std::fs::create_dir_all(&temp).unwrap();
    temp
}

fn create_dummy_png(path: &Path, width: u32, height: u32, color: [u8; 4]) {
    let mut img = image::RgbaImage::new(width, height);
    for pixel in img.pixels_mut() {
        *pixel = image::Rgba(color);
    }
    img.save_with_format(path, image::ImageFormat::Png).unwrap();
}

fn create_dummy_jpeg(path: &Path, width: u32, height: u32, color: [u8; 3]) {
    let mut img = image::RgbImage::new(width, height);
    for pixel in img.pixels_mut() {
        *pixel = image::Rgb(color);
    }
    img.save_with_format(path, image::ImageFormat::Jpeg)
        .unwrap();
}

#[test]
fn test_pdf_to_images_png() {
    let pdf_path = get_corpus_dir().join("pdf-basic/pdfium-hello-world-2-pages.pdf");
    assert!(
        pdf_path.exists(),
        "pdfium-hello-world-2-pages.pdf must exist"
    );

    let out_dir = create_temp_dir("pdf_to_png");
    let options = PdfToImageOptions {
        dpi: 150,
        format: ImageFormatType::Png,
        page_range: None,
    };

    let generated =
        pdf_to_images(&pdf_path, &options, &out_dir).expect("pdf_to_images should succeed");
    assert_eq!(generated.len(), 2, "Expected 2 rendered PNG images");

    for img_path in &generated {
        assert!(img_path.exists());
        assert!(img_path.extension().unwrap() == "png");
        let img = image::open(img_path).expect("Rendered PNG should be readable");
        assert!(img.width() > 0 && img.height() > 0);
    }

    let _ = std::fs::remove_dir_all(&out_dir);
}

#[test]
fn test_pdf_to_images_jpeg_and_page_range() {
    let pdf_path = get_corpus_dir().join("pdf-basic/pdfium-hello-world-2-pages.pdf");
    assert!(pdf_path.exists());

    let out_dir = create_temp_dir("pdf_to_jpeg");
    let options = PdfToImageOptions {
        dpi: 72,
        format: ImageFormatType::Jpeg,
        page_range: Some(vec![2]),
    };

    let generated = pdf_to_images(&pdf_path, &options, &out_dir).expect("render page 2 only");
    assert_eq!(generated.len(), 1, "Only page 2 should be rendered");
    assert!(generated[0].to_string_lossy().contains("page_2.jpg"));

    let img = image::open(&generated[0]).expect("Rendered JPEG should be readable");
    assert!(img.width() > 0 && img.height() > 0);

    let _ = std::fs::remove_dir_all(&out_dir);
}

#[test]
fn test_pdf_to_images_invalid_args() {
    let out_dir = create_temp_dir("pdf_to_img_inv");

    // 1. Non-existent PDF
    let non_existent = out_dir.join("missing.pdf");
    let res = pdf_to_images(&non_existent, &PdfToImageOptions::default(), &out_dir);
    assert!(res.is_err(), "Non-existent PDF should return error");

    // 2. DPI == 0
    let pdf_path = get_corpus_dir().join("pdf-basic/pdfium-hello-world.pdf");
    let invalid_dpi = PdfToImageOptions {
        dpi: 0,
        ..Default::default()
    };
    let res = pdf_to_images(&pdf_path, &invalid_dpi, &out_dir);
    assert!(res.is_err(), "DPI 0 must return error");

    // 3. Page out of range
    let out_of_range = PdfToImageOptions {
        page_range: Some(vec![999]),
        ..Default::default()
    };
    let res = pdf_to_images(&pdf_path, &out_of_range, &out_dir);
    assert!(res.is_err(), "Page 999 must return PageOutOfRange");

    let _ = std::fs::remove_dir_all(&out_dir);
}

#[test]
fn test_images_to_pdf_single_and_multi_page() {
    let temp_dir = create_temp_dir("img_to_pdf");
    let img1 = temp_dir.join("img1.png");
    let img2 = temp_dir.join("img2.jpg");

    create_dummy_png(&img1, 200, 100, [255, 0, 0, 255]);
    create_dummy_jpeg(&img2, 150, 150, [0, 128, 255]);

    let pdf_out = temp_dir.join("combined.pdf");
    let options = ImageToPdfOptions {
        page_size: PageSize::A4,
        margin_pt: 20.0,
    };

    images_to_pdf(&[img1.clone(), img2.clone()], &options, &pdf_out)
        .expect("images_to_pdf multi-page should succeed");

    assert!(pdf_out.exists());
    let engine = create_default_engine();
    let doc = engine
        .open_from_file(&pdf_out, None)
        .expect("Open generated PDF");
    assert_eq!(doc.page_count().unwrap(), 2, "Expected 2 pages in PDF");

    let page1 = doc.page(0).unwrap();
    let (w1, h1) = page1.dimensions_points().unwrap();
    assert!((w1 - 595.0).abs() < 1.0);
    assert!((h1 - 842.0).abs() < 1.0);

    // Test FitImage layout
    let fit_pdf_out = temp_dir.join("fit_image.pdf");
    let fit_options = ImageToPdfOptions {
        page_size: PageSize::FitImage,
        margin_pt: 10.0,
    };
    images_to_pdf(&[img1], &fit_options, &fit_pdf_out).expect("FitImage should succeed");
    let fit_doc = engine
        .open_from_file(&fit_pdf_out, None)
        .expect("Open fit PDF");
    assert_eq!(fit_doc.page_count().unwrap(), 1);
    let fit_page = fit_doc.page(0).unwrap();
    let (fw, fh) = fit_page.dimensions_points().unwrap();
    assert!((fw - 220.0).abs() < 1.0); // 200 + 2*10
    assert!((fh - 120.0).abs() < 1.0); // 100 + 2*10

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_images_to_pdf_invalid_args() {
    let temp_dir = create_temp_dir("img_to_pdf_inv");
    let pdf_out = temp_dir.join("out.pdf");

    // Empty list
    let res = images_to_pdf(&[], &ImageToPdfOptions::default(), &pdf_out);
    assert!(res.is_err(), "Empty images list must return error");

    // Non-existent image
    let missing = temp_dir.join("non_existent.png");
    let res = images_to_pdf(&[missing], &ImageToPdfOptions::default(), &pdf_out);
    assert!(res.is_err(), "Missing image must return error");

    // Negative margin
    let img = temp_dir.join("test.png");
    create_dummy_png(&img, 50, 50, [0, 0, 0, 255]);
    let neg_opt = ImageToPdfOptions {
        margin_pt: -5.0,
        ..Default::default()
    };
    let res = images_to_pdf(&[img], &neg_opt, &pdf_out);
    assert!(res.is_err(), "Negative margin must return error");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_markdown_to_pdf_basic_and_extraction() {
    let temp_dir = create_temp_dir("md_to_pdf_basic");
    let pdf_out = temp_dir.join("markdown_basic.pdf");

    let md_content = r#"
# RBD OFİS Converter Module

## Architectural Overview
This document is automatically converted from markdown to PDF using native pure-Rust lopdf primitives.

### Core Capabilities
* PDF to image conversion (PNG and JPEG)
* Image to PDF aggregation with A4 and FitImage sizing
* Markdown to paginated PDF layout

> RBD OFİS is completely offline, privacy-first, and AGPL-3.0 licensed.

---

End of summary section with paragraph wrapping to demonstrate line breaking.
"#;

    markdown_to_pdf(md_content, &pdf_out).expect("markdown_to_pdf should succeed");
    assert!(pdf_out.exists());

    let engine = create_default_engine();
    let doc = engine
        .open_from_file(&pdf_out, None)
        .expect("Open markdown PDF");
    assert_eq!(
        doc.page_count().unwrap(),
        1,
        "Short markdown should fit on 1 page"
    );

    let page = doc.page(0).unwrap();
    let text = page.extract_text().expect("Extract text from markdown PDF");
    assert!(
        text.contains("RBD OFIS") || text.contains("Converter"),
        "Extracted text was: {text}"
    );
    assert!(
        text.contains("Overview") || text.contains("Capabilities"),
        "Extracted text was: {text}"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_markdown_to_pdf_pagination() {
    let temp_dir = create_temp_dir("md_to_pdf_pages");
    let pdf_out = temp_dir.join("markdown_multipage.pdf");

    // Construct a markdown document long enough to span multiple A4 pages
    let mut long_md = String::new();
    long_md.push_str("# Multi-Page Document Benchmark\n\n");
    for i in 1..=40 {
        long_md.push_str(&format!("## Section {}\n", i));
        long_md.push_str("Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat.\n\n");
        long_md.push_str("* Bullet point item alpha\n");
        long_md.push_str("* Bullet point item beta with extra details\n\n");
        long_md.push_str("---\n\n");
    }

    markdown_to_pdf(&long_md, &pdf_out).expect("Long markdown to PDF should succeed");
    assert!(pdf_out.exists());

    let engine = create_default_engine();
    let doc = engine
        .open_from_file(&pdf_out, None)
        .expect("Open multipage markdown PDF");
    let count = doc.page_count().unwrap();
    assert!(
        count > 1,
        "Long document must produce more than 1 page (actual: {count})"
    );

    let page1_text = doc.page(0).unwrap().extract_text().unwrap();
    assert!(
        page1_text.contains("Multi-Page Document"),
        "Page 1 should have title"
    );

    let page2_text = doc.page(1).unwrap().extract_text().unwrap();
    assert!(
        !page2_text.trim().is_empty(),
        "Page 2 should contain content"
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_markdown_to_pdf_empty() {
    let temp_dir = create_temp_dir("md_to_pdf_empty");
    let pdf_out = temp_dir.join("empty.pdf");

    markdown_to_pdf("", &pdf_out).expect("Empty markdown should produce valid PDF");
    assert!(pdf_out.exists());

    let engine = create_default_engine();
    let doc = engine
        .open_from_file(&pdf_out, None)
        .expect("Open empty markdown PDF");
    assert_eq!(doc.page_count().unwrap(), 1);

    let _ = std::fs::remove_dir_all(&temp_dir);
}
