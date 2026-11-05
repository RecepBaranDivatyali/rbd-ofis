//! Document conversion module for RBD OFİS.
//!
//! Provides conversions between:
//! - PDF documents to image files (PNG, JPEG) with customizable resolution (DPI)
//! - Image files to standardized PDF documents (A4, Letter, FitImage)
//! - Markdown text to paginated PDF documents with Helvetica typography

#![allow(clippy::collapsible_if, clippy::collapsible_match)]

use crate::backend::create_default_engine;
use crate::error::EngineError;
use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document, Object, Stream};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Target image format for PDF-to-image rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ImageFormatType {
    #[default]
    Png,
    Jpeg,
}

/// Options controlling PDF page rendering to image files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PdfToImageOptions {
    /// Resolution in dots per inch (default: 150).
    pub dpi: u32,
    /// Output image format: PNG or JPEG (default: PNG).
    pub format: ImageFormatType,
    /// Optional subset of 1-based page indices. If `None`, renders all pages.
    pub page_range: Option<Vec<u32>>,
}

impl Default for PdfToImageOptions {
    fn default() -> Self {
        Self {
            dpi: 150,
            format: ImageFormatType::Png,
            page_range: None,
        }
    }
}

/// Output page dimensions for image-to-PDF conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PageSize {
    #[default]
    A4,
    Letter,
    FitImage,
}

/// Options controlling image embedding into PDF pages.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageToPdfOptions {
    /// Target page size layout (default: A4).
    pub page_size: PageSize,
    /// Margin in points around the image (default: 20.0 pt).
    pub margin_pt: f32,
}

impl Default for ImageToPdfOptions {
    fn default() -> Self {
        Self {
            page_size: PageSize::A4,
            margin_pt: 20.0,
        }
    }
}

/// Render pages from a PDF document to image files (PNG or JPEG).
///
/// Returns the list of paths to created image files in ascending page order.
pub fn pdf_to_images(
    input_path: &Path,
    options: &PdfToImageOptions,
    output_dir: &Path,
) -> Result<Vec<PathBuf>, EngineError> {
    if !input_path.exists() {
        return Err(EngineError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("PDF file not found: {}", input_path.display()),
        )));
    }

    if options.dpi == 0 {
        return Err(EngineError::InvalidParameter(
            "DPI must be greater than 0".into(),
        ));
    }

    if !output_dir.exists() {
        std::fs::create_dir_all(output_dir)?;
    }

    let engine = create_default_engine();
    let doc = engine.open_from_file(input_path, None)?;
    let total_pages = doc.page_count()?;

    if total_pages == 0 {
        return Err(EngineError::Corrupt("PDF contains zero pages".into()));
    }

    let pages_to_render: Vec<u32> = match &options.page_range {
        Some(range) => {
            if range.is_empty() {
                return Err(EngineError::InvalidParameter(
                    "Page range cannot be empty when specified".into(),
                ));
            }
            for &p in range {
                if p < 1 || p > total_pages {
                    return Err(EngineError::PageOutOfRange {
                        index: p,
                        total_pages,
                    });
                }
            }
            range.clone()
        }
        None => (1..=total_pages).collect(),
    };

    let file_stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("doc");
    let ext = match options.format {
        ImageFormatType::Png => "png",
        ImageFormatType::Jpeg => "jpg",
    };

    let scale = options.dpi as f32 / 72.0;
    let mut generated_paths = Vec::with_capacity(pages_to_render.len());

    for p in pages_to_render {
        let page = doc.page(p - 1)?;
        let (dim_w, dim_h) = page.dimensions_points()?;

        let width_px = ((dim_w * scale).round() as u32).max(1);
        let height_px = ((dim_h * scale).round() as u32).max(1);

        let buf_size = (width_px as usize) * (height_px as usize) * 4;
        let mut rgba_buf = vec![0u8; buf_size];

        page.render_tile_to_rgba(0, 0, width_px, height_px, options.dpi as f32, &mut rgba_buf)?;

        let out_filename = format!("{file_stem}_page_{p}.{ext}");
        let out_file_path = output_dir.join(out_filename);

        match options.format {
            ImageFormatType::Png => {
                let img =
                    image::RgbaImage::from_raw(width_px, height_px, rgba_buf).ok_or_else(|| {
                        EngineError::RenderError("Failed to construct image buffer".into())
                    })?;
                img.save_with_format(&out_file_path, image::ImageFormat::Png)
                    .map_err(|e| {
                        EngineError::Io(std::io::Error::other(format!(
                            "Failed to save PNG image: {e}"
                        )))
                    })?;
            }
            ImageFormatType::Jpeg => {
                let img =
                    image::RgbaImage::from_raw(width_px, height_px, rgba_buf).ok_or_else(|| {
                        EngineError::RenderError("Failed to construct image buffer".into())
                    })?;
                let rgb_img = image::DynamicImage::ImageRgba8(img).to_rgb8();
                rgb_img
                    .save_with_format(&out_file_path, image::ImageFormat::Jpeg)
                    .map_err(|e| {
                        EngineError::Io(std::io::Error::other(format!(
                            "Failed to save JPEG image: {e}"
                        )))
                    })?;
            }
        }

        generated_paths.push(out_file_path);
    }

    Ok(generated_paths)
}

/// Convert one or more image files into a single PDF document.
///
/// Images are embedded as individual pages in the order provided.
pub fn images_to_pdf(
    image_paths: &[PathBuf],
    options: &ImageToPdfOptions,
    output_path: &Path,
) -> Result<(), EngineError> {
    if image_paths.is_empty() {
        return Err(EngineError::InvalidParameter(
            "No image paths provided".into(),
        ));
    }

    if options.margin_pt < 0.0 {
        return Err(EngineError::InvalidParameter(
            "margin_pt cannot be negative".into(),
        ));
    }

    for path in image_paths {
        if !path.exists() {
            return Err(EngineError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Image file not found: {}", path.display()),
            )));
        }
    }

    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();
    let mut page_refs = Vec::with_capacity(image_paths.len());

    for path in image_paths {
        let (img_w, img_h, img_stream) = create_image_stream(path)?;
        let img_obj_id = doc.add_object(Object::Stream(img_stream));

        let (page_w, page_h, draw_w, draw_h, draw_x, draw_y) =
            compute_image_placement(img_w, img_h, options);

        // Content stream:
        // q draw_w 0 0 draw_h draw_x draw_y cm /Im1 Do Q
        let ops = vec![
            Operation::new("q", vec![]),
            Operation::new(
                "cm",
                vec![
                    Object::Real(draw_w),
                    Object::Real(0.0),
                    Object::Real(0.0),
                    Object::Real(draw_h),
                    Object::Real(draw_x),
                    Object::Real(draw_y),
                ],
            ),
            Operation::new("Do", vec![Object::Name(b"Im1".to_vec())]),
            Operation::new("Q", vec![]),
        ];

        let content = Content { operations: ops };
        let encoded_ops = content
            .encode()
            .map_err(|e| EngineError::Corrupt(format!("Failed to encode content ops: {e}")))?;
        let content_stream = Stream::new(Dictionary::new(), encoded_ops);
        let content_id = doc.add_object(Object::Stream(content_stream));

        let mut xobject_dict = Dictionary::new();
        xobject_dict.set("Im1", Object::Reference(img_obj_id));

        let mut resources_dict = Dictionary::new();
        resources_dict.set("XObject", Object::Dictionary(xobject_dict));
        resources_dict.set(
            "ProcSet",
            Object::Array(vec![
                Object::Name(b"PDF".to_vec()),
                Object::Name(b"ImageC".to_vec()),
                Object::Name(b"ImageI".to_vec()),
            ]),
        );

        let mut page_dict = Dictionary::new();
        page_dict.set("Type", Object::Name(b"Page".to_vec()));
        page_dict.set("Parent", Object::Reference(pages_id));
        page_dict.set(
            "MediaBox",
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(page_w),
                Object::Real(page_h),
            ]),
        );
        page_dict.set("Resources", Object::Dictionary(resources_dict));
        page_dict.set("Contents", Object::Reference(content_id));

        let page_id = doc.add_object(Object::Dictionary(page_dict));
        page_refs.push(Object::Reference(page_id));
    }

    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
    pages_dict.set("Count", Object::Integer(page_refs.len() as i64));
    pages_dict.set("Kids", Object::Array(page_refs));
    doc.objects.insert(pages_id, Object::Dictionary(pages_dict));

    let mut catalog_dict = Dictionary::new();
    catalog_dict.set("Type", Object::Name(b"Catalog".to_vec()));
    catalog_dict.set("Pages", Object::Reference(pages_id));
    let catalog_id = doc.add_object(Object::Dictionary(catalog_dict));

    doc.trailer.set("Root", Object::Reference(catalog_id));
    doc.trailer
        .set("Size", Object::Integer((doc.max_id + 1) as i64));

    doc.save(output_path)?;
    Ok(())
}

fn create_image_stream(path: &Path) -> Result<(u32, u32, Stream), EngineError> {
    let raw_bytes = std::fs::read(path)?;

    // Check if file is a JPEG (magic bytes: 0xFF, 0xD8, 0xFF)
    let is_jpeg =
        raw_bytes.len() > 3 && raw_bytes[0] == 0xFF && raw_bytes[1] == 0xD8 && raw_bytes[2] == 0xFF;

    if is_jpeg {
        if let Ok(dyn_img) = image::load_from_memory(&raw_bytes) {
            if dyn_img.color() == image::ColorType::Rgb8 {
                let (w, h) = (dyn_img.width(), dyn_img.height());
                let mut dict = Dictionary::new();
                dict.set("Type", Object::Name(b"XObject".to_vec()));
                dict.set("Subtype", Object::Name(b"Image".to_vec()));
                dict.set("Width", Object::Integer(w as i64));
                dict.set("Height", Object::Integer(h as i64));
                dict.set("ColorSpace", Object::Name(b"DeviceRGB".to_vec()));
                dict.set("BitsPerComponent", Object::Integer(8));
                dict.set("Filter", Object::Name(b"DCTDecode".to_vec()));
                let stream = Stream::new(dict, raw_bytes);
                return Ok((w, h, stream));
            }
        }
    }

    // Fallback or non-JPEG: decode using `image` and convert to RGB8
    let dyn_img = image::open(path).map_err(|e| {
        EngineError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to decode image {}: {e}", path.display()),
        ))
    })?;

    let rgb_img = dyn_img.to_rgb8();
    let (w, h) = rgb_img.dimensions();
    let raw_rgb = rgb_img.into_raw();

    let mut dict = Dictionary::new();
    dict.set("Type", Object::Name(b"XObject".to_vec()));
    dict.set("Subtype", Object::Name(b"Image".to_vec()));
    dict.set("Width", Object::Integer(w as i64));
    dict.set("Height", Object::Integer(h as i64));
    dict.set("ColorSpace", Object::Name(b"DeviceRGB".to_vec()));
    dict.set("BitsPerComponent", Object::Integer(8));

    let mut stream = Stream::new(dict, raw_rgb);
    let _ = stream.compress();

    Ok((w, h, stream))
}

fn compute_image_placement(
    img_w: u32,
    img_h: u32,
    options: &ImageToPdfOptions,
) -> (f32, f32, f32, f32, f32, f32) {
    let (img_w_f, img_h_f) = (img_w as f32, img_h as f32);
    let margin = options.margin_pt;

    match options.page_size {
        PageSize::FitImage => {
            let page_w = img_w_f + 2.0 * margin;
            let page_h = img_h_f + 2.0 * margin;
            (page_w, page_h, img_w_f, img_h_f, margin, margin)
        }
        PageSize::A4 => {
            let page_w = 595.0;
            let page_h = 842.0;
            let avail_w = (page_w - 2.0 * margin).max(1.0);
            let avail_h = (page_h - 2.0 * margin).max(1.0);
            let scale = (avail_w / img_w_f).min(avail_h / img_h_f);
            let draw_w = img_w_f * scale;
            let draw_h = img_h_f * scale;
            let draw_x = margin + (avail_w - draw_w) / 2.0;
            let draw_y = margin + (avail_h - draw_h) / 2.0;
            (page_w, page_h, draw_w, draw_h, draw_x, draw_y)
        }
        PageSize::Letter => {
            let page_w = 612.0;
            let page_h = 792.0;
            let avail_w = (page_w - 2.0 * margin).max(1.0);
            let avail_h = (page_h - 2.0 * margin).max(1.0);
            let scale = (avail_w / img_w_f).min(avail_h / img_h_f);
            let draw_w = img_w_f * scale;
            let draw_h = img_h_f * scale;
            let draw_x = margin + (avail_w - draw_w) / 2.0;
            let draw_y = margin + (avail_h - draw_h) / 2.0;
            (page_w, page_h, draw_w, draw_h, draw_x, draw_y)
        }
    }
}

/// Convert Markdown formatted text into a paginated PDF document.
///
/// Features:
/// - Headings (`#`, `##`, `###`) with distinct sizing and bold weight
/// - Bullet lists (`*`, `-`, `+`) with bullet indentation
/// - Numbered lists (`1.`) with aligned items
/// - Horizontal rules (`---`, `***`)
/// - Blockquotes (`>`) with quote border styling
/// - Word-wrapped paragraphs with proper line spacing
/// - Automatic multi-page pagination onto standard A4 pages
pub fn markdown_to_pdf(markdown_text: &str, output_path: &Path) -> Result<(), EngineError> {
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();

    // Register standard Type 1 fonts
    let mut helv = Dictionary::new();
    helv.set("Type", Object::Name(b"Font".to_vec()));
    helv.set("Subtype", Object::Name(b"Type1".to_vec()));
    helv.set("BaseFont", Object::Name(b"Helvetica".to_vec()));
    helv.set("Encoding", Object::Name(b"WinAnsiEncoding".to_vec()));
    let helv_id = doc.add_object(Object::Dictionary(helv));

    let mut helv_bold = Dictionary::new();
    helv_bold.set("Type", Object::Name(b"Font".to_vec()));
    helv_bold.set("Subtype", Object::Name(b"Type1".to_vec()));
    helv_bold.set("BaseFont", Object::Name(b"Helvetica-Bold".to_vec()));
    helv_bold.set("Encoding", Object::Name(b"WinAnsiEncoding".to_vec()));
    let helv_bold_id = doc.add_object(Object::Dictionary(helv_bold));

    let mut courier = Dictionary::new();
    courier.set("Type", Object::Name(b"Font".to_vec()));
    courier.set("Subtype", Object::Name(b"Type1".to_vec()));
    courier.set("BaseFont", Object::Name(b"Courier".to_vec()));
    courier.set("Encoding", Object::Name(b"WinAnsiEncoding".to_vec()));
    let courier_id = doc.add_object(Object::Dictionary(courier));

    let mut font_resources = Dictionary::new();
    font_resources.set("F_HELV", Object::Reference(helv_id));
    font_resources.set("F_HELV_BOLD", Object::Reference(helv_bold_id));
    font_resources.set("F_COURIER", Object::Reference(courier_id));

    const PAGE_WIDTH: f32 = 595.0;
    const PAGE_HEIGHT: f32 = 842.0;
    const MARGIN_LEFT: f32 = 50.0;
    const MARGIN_RIGHT: f32 = 50.0;
    const MARGIN_TOP: f32 = 50.0;
    const MARGIN_BOTTOM: f32 = 50.0;
    const PRINTABLE_WIDTH: f32 = PAGE_WIDTH - MARGIN_LEFT - MARGIN_RIGHT; // 495.0
    const TOP_Y: f32 = PAGE_HEIGHT - MARGIN_TOP; // 792.0
    const MIN_Y: f32 = MARGIN_BOTTOM; // 50.0

    let mut collector = PageCollector::new(TOP_Y, MIN_Y);

    for line in markdown_text.lines() {
        let trimmed = line.trim();

        // 1. Blank line
        if trimmed.is_empty() {
            collector.cursor_y -= 8.0;
            if collector.cursor_y < MIN_Y {
                collector.flush_page();
            }
            continue;
        }

        // 2. Horizontal rule (---, ***, ___)
        if (trimmed.starts_with("---") || trimmed.starts_with("***") || trimmed.starts_with("___"))
            && trimmed.chars().all(|c| c == '-' || c == '*' || c == '_')
        {
            collector.ensure_space(16.0);
            collector.cursor_y -= 8.0;

            // Draw horizontal rule line
            collector.add_op(Operation::new("q", vec![]));
            collector.add_op(Operation::new(
                "RG",
                vec![Object::Real(0.75), Object::Real(0.75), Object::Real(0.75)],
            ));
            collector.add_op(Operation::new("w", vec![Object::Real(1.0)]));
            collector.add_op(Operation::new(
                "m",
                vec![Object::Real(MARGIN_LEFT), Object::Real(collector.cursor_y)],
            ));
            collector.add_op(Operation::new(
                "l",
                vec![
                    Object::Real(PAGE_WIDTH - MARGIN_RIGHT),
                    Object::Real(collector.cursor_y),
                ],
            ));
            collector.add_op(Operation::new("S", vec![]));
            collector.add_op(Operation::new("Q", vec![]));

            collector.cursor_y -= 8.0;
            continue;
        }

        // 3. Headings
        if let Some(h1_text) = trimmed.strip_prefix("# ") {
            collector.render_wrapped_block(
                h1_text,
                "F_HELV_BOLD",
                20.0,
                25.0,
                12.0,
                8.0,
                (0.1, 0.1, 0.1),
                MARGIN_LEFT,
                PRINTABLE_WIDTH,
            );
            continue;
        }

        if let Some(h2_text) = trimmed.strip_prefix("## ") {
            collector.render_wrapped_block(
                h2_text,
                "F_HELV_BOLD",
                15.0,
                19.0,
                10.0,
                6.0,
                (0.15, 0.15, 0.15),
                MARGIN_LEFT,
                PRINTABLE_WIDTH,
            );
            continue;
        }

        if let Some(h3_text) = trimmed.strip_prefix("### ") {
            collector.render_wrapped_block(
                h3_text,
                "F_HELV_BOLD",
                12.0,
                16.0,
                8.0,
                4.0,
                (0.2, 0.2, 0.2),
                MARGIN_LEFT,
                PRINTABLE_WIDTH,
            );
            continue;
        }

        // 4. Bullet list items (*, -, +)
        let bullet_match = if let Some(t) = trimmed.strip_prefix("* ") {
            Some(t)
        } else if let Some(t) = trimmed.strip_prefix("- ") {
            Some(t)
        } else {
            trimmed.strip_prefix("+ ")
        };

        if let Some(item_text) = bullet_match {
            let font = "F_HELV";
            let font_size = 10.0;
            let line_height = 14.0;
            let indent = 15.0;

            collector.ensure_space(line_height + 4.0);
            collector.cursor_y -= line_height;

            // Draw bullet dot
            draw_text_line(
                "*",
                font,
                font_size,
                (0.2, 0.2, 0.2),
                MARGIN_LEFT + 2.0,
                collector.cursor_y,
                &mut collector.current_ops,
            );

            // Wrap item text
            let text_x = MARGIN_LEFT + indent;
            let text_w = PRINTABLE_WIDTH - indent;
            let wrapped = wrap_text(item_text, text_w, font_size);

            for (idx, w_line) in wrapped.iter().enumerate() {
                if idx > 0 {
                    collector.cursor_y -= line_height;
                    if collector.cursor_y < MIN_Y {
                        collector.flush_page();
                        collector.cursor_y -= line_height;
                    }
                }
                draw_text_line(
                    w_line,
                    font,
                    font_size,
                    (0.1, 0.1, 0.1),
                    text_x,
                    collector.cursor_y,
                    &mut collector.current_ops,
                );
            }

            collector.cursor_y -= 3.0; // item spacing
            continue;
        }

        // 5. Blockquotes (> )
        if let Some(quote_text) = trimmed.strip_prefix("> ") {
            let font = "F_HELV";
            let font_size = 10.0;
            let line_height = 14.0;
            let indent = 16.0;

            let text_x = MARGIN_LEFT + indent;
            let text_w = PRINTABLE_WIDTH - indent;
            let wrapped = wrap_text(quote_text, text_w, font_size);
            let block_height = (wrapped.len() as f32) * line_height;

            collector.ensure_space(block_height + 8.0);

            let start_y = collector.cursor_y;
            for w_line in &wrapped {
                collector.cursor_y -= line_height;
                draw_text_line(
                    w_line,
                    font,
                    font_size,
                    (0.3, 0.3, 0.3),
                    text_x,
                    collector.cursor_y,
                    &mut collector.current_ops,
                );
            }
            let end_y = collector.cursor_y;

            // Draw left quote line
            collector.add_op(Operation::new("q", vec![]));
            collector.add_op(Operation::new(
                "RG",
                vec![Object::Real(0.6), Object::Real(0.6), Object::Real(0.6)],
            ));
            collector.add_op(Operation::new("w", vec![Object::Real(2.0)]));
            collector.add_op(Operation::new(
                "m",
                vec![Object::Real(MARGIN_LEFT + 4.0), Object::Real(start_y)],
            ));
            collector.add_op(Operation::new(
                "l",
                vec![Object::Real(MARGIN_LEFT + 4.0), Object::Real(end_y)],
            ));
            collector.add_op(Operation::new("S", vec![]));
            collector.add_op(Operation::new("Q", vec![]));

            collector.cursor_y -= 4.0;
            continue;
        }

        // 6. Regular paragraph
        collector.render_wrapped_block(
            trimmed,
            "F_HELV",
            10.0,
            14.0,
            0.0,
            6.0,
            (0.1, 0.1, 0.1),
            MARGIN_LEFT,
            PRINTABLE_WIDTH,
        );
    }

    let pages = collector.into_pages();

    let mut page_refs = Vec::with_capacity(pages.len());

    for page_ops in pages {
        let content = Content {
            operations: page_ops,
        };
        let encoded_ops = content
            .encode()
            .map_err(|e| EngineError::Corrupt(format!("Failed to encode page content: {e}")))?;
        let content_stream = Stream::new(Dictionary::new(), encoded_ops);
        let content_id = doc.add_object(Object::Stream(content_stream));

        let mut resources_dict = Dictionary::new();
        resources_dict.set("Font", Object::Dictionary(font_resources.clone()));
        resources_dict.set(
            "ProcSet",
            Object::Array(vec![
                Object::Name(b"PDF".to_vec()),
                Object::Name(b"Text".to_vec()),
            ]),
        );

        let mut page_dict = Dictionary::new();
        page_dict.set("Type", Object::Name(b"Page".to_vec()));
        page_dict.set("Parent", Object::Reference(pages_id));
        page_dict.set(
            "MediaBox",
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(PAGE_WIDTH),
                Object::Real(PAGE_HEIGHT),
            ]),
        );
        page_dict.set("Resources", Object::Dictionary(resources_dict));
        page_dict.set("Contents", Object::Reference(content_id));

        let page_id = doc.add_object(Object::Dictionary(page_dict));
        page_refs.push(Object::Reference(page_id));
    }

    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
    pages_dict.set("Count", Object::Integer(page_refs.len() as i64));
    pages_dict.set("Kids", Object::Array(page_refs));
    doc.objects.insert(pages_id, Object::Dictionary(pages_dict));

    let mut catalog_dict = Dictionary::new();
    catalog_dict.set("Type", Object::Name(b"Catalog".to_vec()));
    catalog_dict.set("Pages", Object::Reference(pages_id));
    let catalog_id = doc.add_object(Object::Dictionary(catalog_dict));

    doc.trailer.set("Root", Object::Reference(catalog_id));
    doc.trailer
        .set("Size", Object::Integer((doc.max_id + 1) as i64));

    doc.save(output_path)?;
    Ok(())
}

struct PageCollector {
    pages: Vec<Vec<Operation>>,
    current_ops: Vec<Operation>,
    cursor_y: f32,
    top_y: f32,
    min_y: f32,
}

impl PageCollector {
    fn new(top_y: f32, min_y: f32) -> Self {
        Self {
            pages: Vec::new(),
            current_ops: Vec::new(),
            cursor_y: top_y,
            top_y,
            min_y,
        }
    }

    fn flush_page(&mut self) {
        self.pages.push(std::mem::take(&mut self.current_ops));
        self.cursor_y = self.top_y;
    }

    fn ensure_space(&mut self, needed: f32) {
        if self.cursor_y - needed < self.min_y {
            self.flush_page();
        }
    }

    fn add_op(&mut self, op: Operation) {
        self.current_ops.push(op);
    }

    #[allow(clippy::too_many_arguments)]
    fn render_wrapped_block(
        &mut self,
        text: &str,
        font: &'static str,
        font_size: f32,
        line_height: f32,
        spacing_before: f32,
        spacing_after: f32,
        color: (f32, f32, f32),
        margin_x: f32,
        max_w: f32,
    ) {
        if self.cursor_y - (spacing_before + line_height) < self.min_y {
            self.flush_page();
        } else {
            self.cursor_y -= spacing_before;
        }

        let wrapped = wrap_text(text, max_w, font_size);
        for w_line in wrapped {
            if self.cursor_y - line_height < self.min_y {
                self.flush_page();
            }

            self.cursor_y -= line_height;
            draw_text_line(
                &w_line,
                font,
                font_size,
                color,
                margin_x,
                self.cursor_y,
                &mut self.current_ops,
            );
        }

        self.cursor_y -= spacing_after;
    }

    fn into_pages(mut self) -> Vec<Vec<Operation>> {
        self.pages.push(self.current_ops);
        if self.pages.is_empty() {
            self.pages.push(Vec::new());
        }
        self.pages
    }
}

fn draw_text_line(
    text: &str,
    font: &str,
    font_size: f32,
    color: (f32, f32, f32),
    x: f32,
    y: f32,
    ops: &mut Vec<Operation>,
) {
    let sanitized = sanitize_for_pdf_type1(text);
    ops.push(Operation::new("BT", vec![]));
    ops.push(Operation::new(
        "Tf",
        vec![
            Object::Name(font.as_bytes().to_vec()),
            Object::Real(font_size),
        ],
    ));
    ops.push(Operation::new(
        "rg",
        vec![
            Object::Real(color.0),
            Object::Real(color.1),
            Object::Real(color.2),
        ],
    ));
    ops.push(Operation::new(
        "Tm",
        vec![
            Object::Real(1.0),
            Object::Real(0.0),
            Object::Real(0.0),
            Object::Real(1.0),
            Object::Real(x),
            Object::Real(y),
        ],
    ));
    ops.push(Operation::new(
        "Tj",
        vec![Object::string_literal(&*sanitized)],
    ));
    ops.push(Operation::new("ET", vec![]));
}

/// Word-wrap text to fit within `max_width_pt` points using estimated character metrics.
fn wrap_text(text: &str, max_width_pt: f32, font_size: f32) -> Vec<String> {
    let avg_char_w = font_size * 0.52;
    let max_chars = ((max_width_pt / avg_char_w).floor() as usize).max(10);

    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in words {
        if current_line.is_empty() {
            current_line.push_str(word);
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

/// Sanitize text for standard Type 1 fonts (Helvetica) ensuring clean ASCII rendering.
fn sanitize_for_pdf_type1(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            'ı' => out.push('i'),
            'İ' => out.push('I'),
            'ğ' => out.push('g'),
            'Ğ' => out.push('G'),
            'ş' => out.push('s'),
            'Ş' => out.push('S'),
            'ç' => out.push('c'),
            'Ç' => out.push('C'),
            'ö' => out.push('o'),
            'Ö' => out.push('O'),
            'ü' => out.push('u'),
            'Ü' => out.push('U'),
            '“' | '”' => out.push('"'),
            '‘' | '’' => out.push('\''),
            '—' | '–' => out.push('-'),
            '•' => out.push('*'),
            c if c.is_ascii() => out.push(c),
            _ => out.push('?'),
        }
    }
    out
}
