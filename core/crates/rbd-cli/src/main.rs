use clap::{Args, Parser, Subcommand};
use image::ExtendedColorType;
use rbd_core::{
    BatesOptions, CompressionOptions, DocumentSession, ImageFormatType, ImageToPdfOptions,
    PageNumberOptions, PageSize, PdfToImageOptions, StampPosition, WatermarkOptions,
    apply_bates_stamp, apply_page_numbers, apply_watermark, compress_document, create_simple_docx,
    crop_page, decrypt_pdf, delete_pages, extract_office_text, fill_form_fields, images_to_pdf,
    insert_blank_page, inspect_office_document, markdown_to_pdf, merge, pdf_to_images,
    read_form_fields, reorder_pages, rotate, split,
};
use rbd_engine::{Annotation, AnnotationKind, PdfEngine, PdfRect, create_default_engine};
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Parser)]
#[command(name = "rbd")]
#[command(about = "RBD OFİS PDF CLI & Benchmark Utility", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Display PDF document information and metadata
    Info(InfoArgs),
    /// Render a page to an image (PNG)
    Render(RenderArgs),
    /// Extract text from one or all pages
    Text(TextArgs),
    /// Search document text with bounding boxes
    Search(SearchArgs),
    /// Add an annotation to a PDF page and save
    Annotate(AnnotateArgs),
    /// Reorder pages in a PDF document
    Reorder(ReorderArgs),
    /// Crop a page to specific bounding box coordinates
    Crop(CropArgs),
    /// Insert a blank page into a PDF document
    InsertBlank(InsertBlankArgs),
    /// Delete specified pages from PDF document
    DeletePages(DeletePagesArgs),
    /// Decrypt a password-protected PDF document
    Decrypt(DecryptArgs),
    /// Merge multiple PDF documents into one
    Merge(MergeArgs),
    /// Split a PDF by page range
    Split(SplitArgs),
    /// Rotate PDF pages
    Rotate(RotateArgs),
    /// Compress a PDF document to reduce file size
    Compress(CompressArgs),
    /// Apply a text watermark across document pages
    Watermark(WatermarkArgs),
    /// Apply dynamic page numbers ({page} / {total})
    PageNumbers(PageNumbersArgs),
    /// Apply legal Bates stamping across pages
    Bates(BatesArgs),
    /// Inspect interactive AcroForm form fields
    FormRead(FormReadArgs),
    /// Fill interactive form fields and optionally flatten
    FormFill(FormFillArgs),
    /// Convert PDF pages to image files (PNG/JPEG)
    PdfToImages(PdfToImagesArgs),
    /// Convert image files to a standardized PDF
    ImagesToPdf(ImagesToPdfArgs),
    /// Convert Markdown document into a styled PDF
    MarkdownToPdf(MarkdownToPdfArgs),
    /// Extract text from OOXML Office documents (DOCX, XLSX, PPTX)
    OfficeExtract(OfficeExtractArgs),
    /// Inspect structure & metadata of OOXML Office documents
    OfficeInspect(OfficeInspectArgs),
    /// Create a minimal DOCX document from text
    OfficeCreateDocx(OfficeCreateDocxArgs),
    /// Summarize document using offline AI natural language processing
    AiSummarize(AiSummarizeArgs),
    /// Ask natural language questions about document content with citations
    AiAsk(AiAskArgs),
    /// Extract fluid, structured paragraphs from a page for reflow editing
    ReflowText(ReflowTextArgs),
    /// Replace paragraph text with dynamic multi-line reflow
    ReplaceParagraph(ReplaceParagraphArgs),
    /// Benchmark cold open, first-page render, and 256x256 tile rendering latency
    Bench(BenchArgs),
}

#[derive(Args)]
struct SearchArgs {
    /// Path to the PDF file
    file: PathBuf,
    /// Search query text
    query: String,
    /// Case-sensitive search
    #[arg(long)]
    case_sensitive: bool,
}

#[derive(Args)]
struct AnnotateArgs {
    /// Input PDF file path
    input: PathBuf,
    /// Output PDF file path
    #[arg(short, long)]
    out: PathBuf,
    /// 1-based page number
    #[arg(short, long)]
    page: u32,
    /// Annotation kind: highlight, note, rect, ink, freetext
    #[arg(short, long)]
    kind: String,
    /// Bounding rectangle "x0,y0,x1,y1" in points
    #[arg(short, long)]
    rect: String,
    /// Text contents or note body
    #[arg(short, long)]
    text: Option<String>,
}

#[derive(Args)]
struct ReorderArgs {
    /// Input PDF file path
    input: PathBuf,
    /// Comma-separated 1-based page order, e.g. "1,3,2"
    #[arg(long)]
    order: String,
    /// Output PDF file path
    #[arg(short, long)]
    out: PathBuf,
}

#[derive(Args)]
struct CropArgs {
    /// Input PDF file path
    input: PathBuf,
    /// 1-based page number
    #[arg(short, long)]
    page: u32,
    /// Bounding box "x0,y0,x1,y1" in points
    #[arg(long = "box")]
    r#box: String,
    /// Output PDF file path
    #[arg(short, long)]
    out: PathBuf,
}

#[derive(Args)]
struct InsertBlankArgs {
    /// Input PDF file path
    input: PathBuf,
    /// Insert after 1-based page number (0 to prepend before page 1)
    #[arg(short, long)]
    after: u32,
    /// Page width in points (default: 595.0 / A4)
    #[arg(long, default_value_t = 595.0)]
    width: f32,
    /// Page height in points (default: 842.0 / A4)
    #[arg(long, default_value_t = 842.0)]
    height: f32,
    /// Output PDF file path
    #[arg(short, long)]
    out: PathBuf,
}

#[derive(Args)]
struct DeletePagesArgs {
    /// Input PDF file path
    input: PathBuf,
    /// Comma-separated 1-based page numbers to delete, e.g. "2,4"
    #[arg(short, long)]
    pages: String,
    /// Output PDF file path
    #[arg(short, long)]
    out: PathBuf,
}

#[derive(Args)]
struct DecryptArgs {
    /// Input encrypted PDF file path
    input: PathBuf,
    /// Document password
    #[arg(short, long)]
    password: String,
    /// Output decrypted PDF file path
    #[arg(short, long)]
    out: PathBuf,
}

#[derive(Args)]
struct InfoArgs {
    /// Path to the PDF file
    file: PathBuf,
}

#[derive(Args)]
struct RenderArgs {
    /// Path to the PDF file
    file: PathBuf,
    /// 1-based page number to render
    #[arg(short, long, default_value_t = 1)]
    page: u32,
    /// DPI resolution
    #[arg(short, long, default_value_t = 150.0)]
    dpi: f32,
    /// Output image path (.png)
    #[arg(short, long)]
    out: PathBuf,
}

#[derive(Args)]
struct TextArgs {
    /// Path to the PDF file
    file: PathBuf,
    /// Optional 1-based page number (if omitted, extracts all pages)
    #[arg(short, long)]
    page: Option<u32>,
}

#[derive(Args)]
struct MergeArgs {
    /// Output merged PDF file path
    out: PathBuf,
    /// Input PDF files to merge (in order)
    #[arg(required = true)]
    inputs: Vec<PathBuf>,
}

#[derive(Args)]
struct SplitArgs {
    /// Path to the PDF file
    file: PathBuf,
    /// 1-based page range, e.g. "1-5" or "3"
    #[arg(short, long)]
    range: String,
    /// Output PDF file path
    #[arg(short, long)]
    out: PathBuf,
}

#[derive(Args)]
struct RotateArgs {
    /// Path to the PDF file
    file: PathBuf,
    /// Rotation degrees clockwise (90, 180, or 270)
    #[arg(short, long)]
    degrees: i32,
    /// Output PDF file path
    #[arg(short, long)]
    out: PathBuf,
}

#[derive(Args)]
struct CompressArgs {
    /// Input PDF file path
    input: PathBuf,
    /// Output compressed PDF path
    #[arg(short, long)]
    out: PathBuf,
    /// Strip document metadata (/Info, /Metadata)
    #[arg(long)]
    strip_metadata: bool,
}

#[derive(Args)]
struct WatermarkArgs {
    /// Input PDF file path
    input: PathBuf,
    /// Output PDF path
    #[arg(short, long)]
    out: PathBuf,
    /// Watermark text string
    #[arg(short, long)]
    text: String,
    /// Font size in points (default: 48.0)
    #[arg(long, default_value_t = 48.0)]
    font_size: f32,
    /// Rotation angle in degrees (default: 45.0)
    #[arg(long, default_value_t = 45.0)]
    angle: f32,
    /// Opacity from 0.0 to 1.0 (default: 0.25)
    #[arg(long, default_value_t = 0.25)]
    opacity: f32,
}

#[derive(Args)]
struct PageNumbersArgs {
    /// Input PDF file path
    input: PathBuf,
    /// Output PDF path
    #[arg(short, long)]
    out: PathBuf,
    /// Template string with {page} and {total} placeholders
    #[arg(long, default_value = "Sayfa {page} / {total}")]
    template: String,
    /// Position (topleft, topcenter, topright, bottomleft, bottomcenter, bottomright)
    #[arg(long, default_value = "bottomcenter")]
    position: String,
    /// 1-based start page
    #[arg(long, default_value_t = 1)]
    start_page: u32,
}

#[derive(Args)]
struct BatesArgs {
    /// Input PDF file path
    input: PathBuf,
    /// Output PDF path
    #[arg(short, long)]
    out: PathBuf,
    /// Prefix (e.g. "RBD-")
    #[arg(long, default_value = "RBD-")]
    prefix: String,
    /// Suffix (optional)
    #[arg(long)]
    suffix: Option<String>,
    /// Number of digits to pad with zeros
    #[arg(long, default_value_t = 6)]
    digits: usize,
    /// Starting number
    #[arg(long, default_value_t = 1)]
    start: u32,
    /// Position (topleft, topcenter, topright, bottomleft, bottomcenter, bottomright)
    #[arg(long, default_value = "bottomright")]
    position: String,
}

#[derive(Args)]
struct FormReadArgs {
    /// Input PDF file path
    input: PathBuf,
    /// Output as JSON
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
struct FormFillArgs {
    /// Input PDF file path
    input: PathBuf,
    /// Output PDF path
    #[arg(short, long)]
    out: PathBuf,
    /// Comma-separated "key=value,key2=value2" or JSON format
    #[arg(short, long)]
    data: String,
    /// Flatten form fields permanently into page content
    #[arg(long)]
    flatten: bool,
}

#[derive(Args)]
struct PdfToImagesArgs {
    /// Input PDF file path
    input: PathBuf,
    /// Output directory for rendered images
    #[arg(short, long)]
    out_dir: PathBuf,
    /// Image DPI resolution (default: 150)
    #[arg(long, default_value_t = 150)]
    dpi: u32,
    /// Format: png or jpeg (default: png)
    #[arg(long, default_value = "png")]
    format: String,
}

#[derive(Args)]
struct ImagesToPdfArgs {
    /// Output PDF file path
    #[arg(short, long)]
    out: PathBuf,
    /// Input image files
    #[arg(required = true)]
    inputs: Vec<PathBuf>,
    /// Page size: a4, letter, fit (default: a4)
    #[arg(long, default_value = "a4")]
    page_size: String,
    /// Margin in points (default: 20.0)
    #[arg(long, default_value_t = 20.0)]
    margin: f32,
}

#[derive(Args)]
struct MarkdownToPdfArgs {
    /// Input Markdown file path
    input: PathBuf,
    /// Output PDF file path
    #[arg(short, long)]
    out: PathBuf,
}

#[derive(Args)]
struct OfficeExtractArgs {
    /// Path to DOCX, XLSX, or PPTX file
    input: PathBuf,
}

#[derive(Args)]
struct OfficeInspectArgs {
    /// Path to DOCX, XLSX, or PPTX file
    input: PathBuf,
    /// Output as JSON
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
struct OfficeCreateDocxArgs {
    /// Output DOCX file path
    #[arg(short, long)]
    out: PathBuf,
    /// Text content for the document
    #[arg(short, long)]
    text: String,
}

#[derive(Args)]
struct AiSummarizeArgs {
    /// Path to the PDF file
    file: PathBuf,
    /// Target maximum sentence count
    #[arg(short, long, default_value_t = 5)]
    sentences: usize,
    /// Output as JSON
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
struct AiAskArgs {
    /// Path to the PDF file
    file: PathBuf,
    /// Natural language question
    query: String,
    /// Output as JSON
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
struct ReflowTextArgs {
    /// Path to the PDF file
    file: PathBuf,
    /// 1-based page number
    #[arg(short, long, default_value_t = 1)]
    page: u32,
    /// Output as JSON
    #[arg(long)]
    json: bool,
}

#[derive(Args)]
struct ReplaceParagraphArgs {
    /// Input PDF file path
    input: PathBuf,
    /// 1-based page number
    #[arg(short, long)]
    page: u32,
    /// Paragraph bounding box "x0,y0,x1,y1"
    #[arg(long = "box")]
    bbox: String,
    /// Replacement text
    #[arg(short, long)]
    text: String,
    /// Font size in points
    #[arg(long, default_value_t = 11.0)]
    font_size: f32,
    /// Output PDF file path
    #[arg(short, long)]
    out: PathBuf,
}

#[derive(Args)]
struct BenchArgs {
    /// Path to the PDF file
    file: PathBuf,
    /// Output results as JSON
    #[arg(long)]
    json: bool,
    /// Number of tile benchmark iterations
    #[arg(long, default_value_t = 50)]
    tiles: usize,
}

#[derive(Serialize)]
struct BenchmarkReport {
    file: String,
    total_pages: u32,
    engine: String,
    cold_open_ms: f64,
    first_page_render_ms: f64,
    first_page_open_and_render_ms: f64,
    gate_300ms_passed: bool,
    tile_count: usize,
    tile_min_ms: f64,
    tile_avg_ms: f64,
    tile_max_ms: f64,
    tile_p95_ms: f64,
    tile_60fps_passed: bool,
}

fn parse_stamp_pos(pos: &str) -> StampPosition {
    match pos.to_lowercase().replace(['-', '_'], "").as_str() {
        "topleft" => StampPosition::TopLeft,
        "topcenter" => StampPosition::TopCenter,
        "topright" => StampPosition::TopRight,
        "bottomleft" => StampPosition::BottomLeft,
        "bottomcenter" => StampPosition::BottomCenter,
        "bottomright" => StampPosition::BottomRight,
        _ => StampPosition::BottomCenter,
    }
}

fn parse_page_sz(sz: &str) -> PageSize {
    match sz.to_lowercase().as_str() {
        "letter" => PageSize::Letter,
        "fit" | "fitimage" => PageSize::FitImage,
        _ => PageSize::A4,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let engine = create_default_engine();

    match cli.command {
        Commands::Info(args) => handle_info(&args.file, engine.as_ref())?,
        Commands::Render(args) => handle_render(&args, engine.as_ref())?,
        Commands::Text(args) => handle_text(&args, engine.as_ref())?,
        Commands::Search(args) => handle_search(&args, engine.as_ref())?,
        Commands::Annotate(args) => handle_annotate(&args, engine.as_ref())?,
        Commands::Reorder(args) => handle_reorder(&args)?,
        Commands::Crop(args) => handle_crop(&args)?,
        Commands::InsertBlank(args) => handle_insert_blank(&args)?,
        Commands::DeletePages(args) => handle_delete_pages(&args)?,
        Commands::Decrypt(args) => handle_decrypt(&args)?,
        Commands::Merge(args) => handle_merge(&args)?,
        Commands::Split(args) => handle_split(&args)?,
        Commands::Rotate(args) => handle_rotate(&args)?,
        Commands::Compress(args) => handle_compress(&args)?,
        Commands::Watermark(args) => handle_watermark(&args)?,
        Commands::PageNumbers(args) => handle_page_numbers(&args)?,
        Commands::Bates(args) => handle_bates(&args)?,
        Commands::FormRead(args) => handle_form_read(&args)?,
        Commands::FormFill(args) => handle_form_fill(&args)?,
        Commands::PdfToImages(args) => handle_pdf_to_images(&args)?,
        Commands::ImagesToPdf(args) => handle_images_to_pdf(&args)?,
        Commands::MarkdownToPdf(args) => handle_markdown_to_pdf(&args)?,
        Commands::OfficeExtract(args) => handle_office_extract(&args)?,
        Commands::OfficeInspect(args) => handle_office_inspect(&args)?,
        Commands::OfficeCreateDocx(args) => handle_office_create_docx(&args)?,
        Commands::AiSummarize(args) => handle_ai_summarize(&args, engine.as_ref())?,
        Commands::AiAsk(args) => handle_ai_ask(&args, engine.as_ref())?,
        Commands::ReflowText(args) => handle_reflow_text(&args, engine.as_ref())?,
        Commands::ReplaceParagraph(args) => handle_replace_paragraph(&args, engine.as_ref())?,
        Commands::Bench(args) => handle_bench(&args, engine.as_ref())?,
    }

    Ok(())
}

fn handle_compress(args: &CompressArgs) -> Result<(), Box<dyn std::error::Error>> {
    let opts = CompressionOptions {
        remove_unused_objects: true,
        recompress_streams: true,
        strip_metadata: args.strip_metadata,
    };
    let res = compress_document(&args.input, &opts, &args.out)?;
    println!(
        "Successfully compressed {}: {} -> {} bytes (saved {:.1}%)",
        args.input.display(),
        res.original_size,
        res.compressed_size,
        res.savings_percent
    );
    Ok(())
}

fn handle_watermark(args: &WatermarkArgs) -> Result<(), Box<dyn std::error::Error>> {
    let opts = WatermarkOptions {
        text: args.text.clone(),
        font_size: args.font_size,
        rotation_degrees: args.angle,
        opacity: args.opacity,
        color_rgb: (180, 180, 180),
        on_top: true,
    };
    apply_watermark(&args.input, &opts, &args.out)?;
    println!(
        "Successfully applied watermark '{}' to {}",
        args.text,
        args.out.display()
    );
    Ok(())
}

fn handle_page_numbers(args: &PageNumbersArgs) -> Result<(), Box<dyn std::error::Error>> {
    let opts = PageNumberOptions {
        template: args.template.clone(),
        position: parse_stamp_pos(&args.position),
        font_size: 10.0,
        margin: 36.0,
        color_rgb: (80, 80, 80),
        start_page: args.start_page,
    };
    apply_page_numbers(&args.input, &opts, &args.out)?;
    println!(
        "Successfully applied page numbers to {}",
        args.out.display()
    );
    Ok(())
}

fn handle_bates(args: &BatesArgs) -> Result<(), Box<dyn std::error::Error>> {
    let opts = BatesOptions {
        prefix: args.prefix.clone(),
        start_number: args.start,
        padding: args.digits,
        suffix: args.suffix.clone().unwrap_or_default(),
        position: parse_stamp_pos(&args.position),
        margin: 36.0,
        color_rgb: (60, 60, 60),
    };
    apply_bates_stamp(&args.input, &opts, &args.out)?;
    println!("Successfully applied Bates stamp to {}", args.out.display());
    Ok(())
}

fn handle_form_read(args: &FormReadArgs) -> Result<(), Box<dyn std::error::Error>> {
    let fields = read_form_fields(&args.input)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&fields)?);
    } else {
        println!(
            "Found {} form field(s) in {}:",
            fields.len(),
            args.input.display()
        );
        for (i, f) in fields.iter().enumerate() {
            println!(
                "  [{}] Name: '{}' | Type: {:?} | Value: '{}' | ReadOnly: {}",
                i + 1,
                f.name,
                f.field_type,
                f.value,
                f.read_only
            );
        }
    }
    Ok(())
}

fn handle_form_fill(args: &FormFillArgs) -> Result<(), Box<dyn std::error::Error>> {
    let values: HashMap<String, String> = if args.data.trim_start().starts_with('{') {
        serde_json::from_str(&args.data)?
    } else {
        let mut map = HashMap::new();
        for pair in args.data.split(',') {
            if let Some((k, v)) = pair.split_once('=') {
                map.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
        map
    };

    fill_form_fields(&args.input, &values, args.flatten, &args.out)?;
    println!(
        "Successfully filled {} field(s) into {} (flatten: {})",
        values.len(),
        args.out.display(),
        args.flatten
    );
    Ok(())
}

fn handle_pdf_to_images(args: &PdfToImagesArgs) -> Result<(), Box<dyn std::error::Error>> {
    let format = match args.format.to_lowercase().as_str() {
        "jpeg" | "jpg" => ImageFormatType::Jpeg,
        _ => ImageFormatType::Png,
    };
    let opts = PdfToImageOptions {
        dpi: args.dpi,
        format,
        page_range: None,
    };
    let files = pdf_to_images(&args.input, &opts, &args.out_dir)?;
    println!(
        "Successfully converted {} to {} images in {}",
        args.input.display(),
        files.len(),
        args.out_dir.display()
    );
    Ok(())
}

fn handle_images_to_pdf(args: &ImagesToPdfArgs) -> Result<(), Box<dyn std::error::Error>> {
    let opts = ImageToPdfOptions {
        page_size: parse_page_sz(&args.page_size),
        margin_pt: args.margin,
    };
    images_to_pdf(&args.inputs, &opts, &args.out)?;
    println!(
        "Successfully combined {} image(s) into {}",
        args.inputs.len(),
        args.out.display()
    );
    Ok(())
}

fn handle_markdown_to_pdf(args: &MarkdownToPdfArgs) -> Result<(), Box<dyn std::error::Error>> {
    let markdown_text = std::fs::read_to_string(&args.input)?;
    markdown_to_pdf(&markdown_text, &args.out)?;
    println!(
        "Successfully converted Markdown {} to PDF {}",
        args.input.display(),
        args.out.display()
    );
    Ok(())
}

fn handle_office_extract(args: &OfficeExtractArgs) -> Result<(), Box<dyn std::error::Error>> {
    let text = extract_office_text(&args.input)?;
    println!("{}", text);
    Ok(())
}

fn handle_office_inspect(args: &OfficeInspectArgs) -> Result<(), Box<dyn std::error::Error>> {
    let info = inspect_office_document(&args.input)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        println!("=== Office Document Info: {} ===", args.input.display());
        println!("Document Type: {:?}", info.doc_type);
        println!("Title: {}", info.title);
        println!("Word Count: {}", info.word_count);
        println!("Item Count: {}", info.item_count);
        println!("Summary: {}", info.summary);
    }
    Ok(())
}

fn handle_office_create_docx(
    args: &OfficeCreateDocxArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    create_simple_docx(&args.text, &args.out)?;
    println!("Successfully created DOCX: {}", args.out.display());
    Ok(())
}

fn handle_ai_summarize(
    args: &AiSummarizeArgs,
    engine: &dyn PdfEngine,
) -> Result<(), Box<dyn std::error::Error>> {
    let session = DocumentSession::open_file(&args.file, None, engine, None)?;
    let summary = session.summarize_document(args.sentences)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        println!("=== Document Summary: {} ===", args.file.display());
        println!(
            "Words: {} | Reading Time: ~{:.1} min | Sentences: {}",
            summary.word_count, summary.reading_time_minutes, summary.total_sentences
        );
        println!("\nSummary:\n{}", summary.summary_text);
        if !summary.top_keywords.is_empty() {
            let kw: Vec<&str> = summary
                .top_keywords
                .iter()
                .map(|k| k.word.as_str())
                .collect();
            println!("\nTop Keywords: {:?}", kw);
        }
    }
    Ok(())
}

fn handle_ai_ask(
    args: &AiAskArgs,
    engine: &dyn PdfEngine,
) -> Result<(), Box<dyn std::error::Error>> {
    let session = DocumentSession::open_file(&args.file, None, engine, None)?;
    let answer = session.ask_document(&args.query)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&answer)?);
    } else {
        println!("Q: {}", answer.query);
        if answer.found {
            println!(
                "A: {} (Page {}, Confidence: {:.0}%)",
                answer.answer_text,
                answer.page_number,
                answer.confidence * 100.0
            );
            if !answer.context_excerpt.is_empty() {
                println!("\nContext:\n{}", answer.context_excerpt);
            }
        } else {
            println!("A: {}", answer.answer_text);
        }
    }
    Ok(())
}

fn handle_reflow_text(
    args: &ReflowTextArgs,
    engine: &dyn PdfEngine,
) -> Result<(), Box<dyn std::error::Error>> {
    let session = DocumentSession::open_file(&args.file, None, engine, None)?;
    let page_idx = args.page.saturating_sub(1);
    let paragraphs = session.extract_page_paragraphs(page_idx)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&paragraphs)?);
    } else {
        println!(
            "Found {} paragraph(s) on page {}:",
            paragraphs.len(),
            args.page
        );
        for p in &paragraphs {
            println!(
                "  [{}] [{:.1}, {:.1}, {:.1}, {:.1}] (font {:.1}pt): {}",
                p.id, p.bbox[0], p.bbox[1], p.bbox[2], p.bbox[3], p.font_size, p.text
            );
        }
    }
    Ok(())
}

fn handle_replace_paragraph(
    args: &ReplaceParagraphArgs,
    engine: &dyn PdfEngine,
) -> Result<(), Box<dyn std::error::Error>> {
    let session = DocumentSession::open_file(&args.input, None, engine, None)?;
    let page_idx = args.page.saturating_sub(1);
    let parts: Vec<f32> = args
        .bbox
        .split(',')
        .map(|s| s.trim().parse::<f32>())
        .collect::<Result<_, _>>()?;
    if parts.len() != 4 {
        eprintln!("Error: bbox must be 'x0,y0,x1,y1'");
        std::process::exit(1);
    }
    let bbox = [parts[0], parts[1], parts[2], parts[3]];
    session.replace_paragraph_text(
        &args.input,
        page_idx,
        bbox,
        &args.text,
        args.font_size,
        &args.out,
    )?;
    println!(
        "Successfully replaced paragraph text and reflowed into: {}",
        args.out.display()
    );
    Ok(())
}

fn handle_search(
    args: &SearchArgs,
    engine: &dyn PdfEngine,
) -> Result<(), Box<dyn std::error::Error>> {
    let session = DocumentSession::open_file(&args.file, None, engine, None)?;
    let hits = session.search_document(&args.query, args.case_sensitive)?;

    println!(
        "Found {} match(es) for '{}' in {}:",
        hits.len(),
        args.query,
        args.file.display()
    );
    for (i, hit) in hits.iter().enumerate() {
        println!(
            "  [{}] Page {}: \"{}\" | Rects: {:?}",
            i + 1,
            hit.page_index + 1,
            hit.snippet,
            hit.rects
        );
    }
    Ok(())
}

fn handle_annotate(
    args: &AnnotateArgs,
    engine: &dyn PdfEngine,
) -> Result<(), Box<dyn std::error::Error>> {
    if args.page < 1 {
        eprintln!("Error: page number must be >= 1");
        std::process::exit(1);
    }

    let rect_parts: Vec<f32> = args
        .rect
        .split(',')
        .map(|s| s.trim().parse::<f32>())
        .collect::<Result<_, _>>()?;

    if rect_parts.len() != 4 {
        eprintln!("Error: rect must be 'x0,y0,x1,y1'");
        std::process::exit(1);
    }

    let rect = PdfRect::new(rect_parts[0], rect_parts[1], rect_parts[2], rect_parts[3]);
    let kind = match args.kind.to_lowercase().as_str() {
        "highlight" => AnnotationKind::Highlight,
        "underline" => AnnotationKind::Underline,
        "strikeout" => AnnotationKind::StrikeOut,
        "note" | "stickynote" => AnnotationKind::StickyNote,
        "rect" | "rectangle" | "square" => AnnotationKind::Rectangle,
        "ink" => AnnotationKind::FreehandInk {
            points: vec![(rect.x0, rect.y0), (rect.x1, rect.y1)],
            stroke_width: 2.0,
        },
        "freetext" | "text" => AnnotationKind::FreeText { font_size: 12.0 },
        other => {
            eprintln!("Unknown annotation kind: '{other}'");
            std::process::exit(1);
        }
    };

    let mut session = DocumentSession::open_file(&args.input, None, engine, None)?;
    let annot = Annotation::new(
        1,
        args.page - 1,
        kind,
        rect,
        [255, 235, 59, 255],
        args.text.clone().unwrap_or_default(),
    );
    session.add_annotation_undoable(annot)?;
    session.save_annotations_to_pdf(&args.input, &args.out)?;

    println!(
        "Successfully added annotation to {} and saved to {}",
        args.input.display(),
        args.out.display()
    );
    Ok(())
}

fn handle_reorder(args: &ReorderArgs) -> Result<(), Box<dyn std::error::Error>> {
    let order: Vec<u32> = args
        .order
        .split(',')
        .map(|s| s.trim().parse::<u32>())
        .collect::<Result<_, _>>()?;

    reorder_pages(&args.input, &args.out, &order)?;
    println!(
        "Successfully reordered pages of {} into {}",
        args.input.display(),
        args.out.display()
    );
    Ok(())
}

fn handle_crop(args: &CropArgs) -> Result<(), Box<dyn std::error::Error>> {
    let parts: Vec<f32> = args
        .r#box
        .split(',')
        .map(|s| s.trim().parse::<f32>())
        .collect::<Result<_, _>>()?;

    if parts.len() != 4 {
        eprintln!("Error: box must be 'x0,y0,x1,y1'");
        std::process::exit(1);
    }

    crop_page(
        &args.input,
        &args.out,
        args.page,
        [parts[0], parts[1], parts[2], parts[3]],
    )?;
    println!(
        "Successfully cropped page {} of {} into {}",
        args.page,
        args.input.display(),
        args.out.display()
    );
    Ok(())
}

fn handle_insert_blank(args: &InsertBlankArgs) -> Result<(), Box<dyn std::error::Error>> {
    insert_blank_page(&args.input, &args.out, args.after, args.width, args.height)?;
    println!(
        "Successfully inserted blank page ({:.1}x{:.1} pt) after page {} into {}",
        args.width,
        args.height,
        args.after,
        args.out.display()
    );
    Ok(())
}

fn handle_delete_pages(args: &DeletePagesArgs) -> Result<(), Box<dyn std::error::Error>> {
    let pages: Vec<u32> = args
        .pages
        .split(',')
        .map(|s| s.trim().parse::<u32>())
        .collect::<Result<_, _>>()?;

    delete_pages(&args.input, &pages, &args.out)?;
    println!(
        "Successfully deleted pages {:?} from {} into {}",
        pages,
        args.input.display(),
        args.out.display()
    );
    Ok(())
}

fn handle_decrypt(args: &DecryptArgs) -> Result<(), Box<dyn std::error::Error>> {
    decrypt_pdf(&args.input, &args.out, &args.password)?;
    println!(
        "Successfully decrypted {} into {}",
        args.input.display(),
        args.out.display()
    );
    Ok(())
}

fn handle_info(file: &Path, engine: &dyn PdfEngine) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== PDF Information: {} ===", file.display());
    println!("Engine: {}", engine.name());

    let session = DocumentSession::open_file(file, None, engine, None)?;
    let count = session.page_count()?;
    println!("Total Pages: {}", count);

    if count > 0 {
        let (w, h) = session.page_dimensions(0)?;
        let w_in = w / 72.0;
        let h_in = h / 72.0;
        let w_mm = w_in * 25.4;
        let h_mm = h_in * 25.4;
        println!(
            "Page 1 Dimensions: {:.2} x {:.2} pt ({:.2} x {:.2} in / {:.1} x {:.1} mm)",
            w, h, w_in, h_in, w_mm, h_mm
        );
    }

    let meta = session.metadata()?;
    println!("--- Metadata ---");
    if let Some(t) = meta.title {
        println!("Title: {}", t);
    }
    if let Some(a) = meta.author {
        println!("Author: {}", a);
    }
    if let Some(s) = meta.subject {
        println!("Subject: {}", s);
    }
    if let Some(k) = meta.keywords {
        println!("Keywords: {}", k);
    }
    if let Some(c) = meta.creator {
        println!("Creator: {}", c);
    }
    if let Some(p) = meta.producer {
        println!("Producer: {}", p);
    }
    if let Some(d) = meta.creation_date {
        println!("Creation Date: {}", d);
    }
    if let Some(m) = meta.mod_date {
        println!("Mod Date: {}", m);
    }

    Ok(())
}

fn handle_render(
    args: &RenderArgs,
    engine: &dyn PdfEngine,
) -> Result<(), Box<dyn std::error::Error>> {
    if args.page < 1 {
        eprintln!("Error: page number must be >= 1");
        std::process::exit(1);
    }

    let session = DocumentSession::open_file(&args.file, None, engine, None)?;
    let page_idx = args.page - 1;
    let (pt_w, pt_h) = session.page_dimensions(page_idx)?;

    let scale = args.dpi / 72.0;
    let px_w = (pt_w * scale).round() as u32;
    let px_h = (pt_h * scale).round() as u32;

    println!(
        "Rendering page {} of {} at {:.1} DPI ({}x{} px)...",
        args.page,
        args.file.display(),
        args.dpi,
        px_w,
        px_h
    );

    let mut buf = vec![0u8; (px_w as usize) * (px_h as usize) * 4];
    session.render_page_into_buffer(page_idx, px_w, px_h, &mut buf)?;

    image::save_buffer(&args.out, &buf, px_w, px_h, ExtendedColorType::Rgba8)?;

    println!("Saved rendered image to: {}", args.out.display());
    Ok(())
}

fn handle_text(args: &TextArgs, engine: &dyn PdfEngine) -> Result<(), Box<dyn std::error::Error>> {
    let session = DocumentSession::open_file(&args.file, None, engine, None)?;
    let total = session.page_count()?;

    if let Some(page) = args.page {
        if page < 1 || page > total {
            eprintln!("Error: page {} out of range (1..={})", page, total);
            std::process::exit(1);
        }
        let text = session.extract_text(page - 1)?;
        println!("--- Page {} Text ---", page);
        println!("{}", text);
    } else {
        println!(
            "--- Text for {} (total pages: {}) ---",
            args.file.display(),
            total
        );
        for p in 0..total {
            let text = session.extract_text(p)?;
            println!("=== Page {} ===", p + 1);
            println!("{}", text);
        }
    }

    Ok(())
}

fn handle_merge(args: &MergeArgs) -> Result<(), Box<dyn std::error::Error>> {
    let input_refs: Vec<&Path> = args.inputs.iter().map(|p| p.as_path()).collect();
    println!(
        "Merging {} PDF files into {}...",
        input_refs.len(),
        args.out.display()
    );
    merge(&input_refs, &args.out)?;
    println!("Successfully merged into: {}", args.out.display());
    Ok(())
}

fn handle_split(args: &SplitArgs) -> Result<(), Box<dyn std::error::Error>> {
    let parts: Vec<&str> = args.range.split('-').collect();
    let (start, end) = if parts.len() == 1 {
        let p: u32 = parts[0].parse()?;
        (p, p)
    } else if parts.len() == 2 {
        let s: u32 = parts[0].parse()?;
        let e: u32 = parts[1].parse()?;
        (s, e)
    } else {
        eprintln!(
            "Invalid range format '{}'. Expected 'start-end' or 'page'",
            args.range
        );
        std::process::exit(1);
    };

    println!(
        "Splitting {} (pages {}..={}) to {}...",
        args.file.display(),
        start,
        end,
        args.out.display()
    );
    split(&args.file, start, end, &args.out)?;
    println!("Successfully split to: {}", args.out.display());
    Ok(())
}

fn handle_rotate(args: &RotateArgs) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "Rotating {} by {} degrees clockwise into {}...",
        args.file.display(),
        args.degrees,
        args.out.display()
    );
    rotate(&args.file, args.degrees, &args.out)?;
    println!("Successfully rotated to: {}", args.out.display());
    Ok(())
}

fn handle_bench(
    args: &BenchArgs,
    engine: &dyn PdfEngine,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Measure Cold Document Open Time
    let t0 = Instant::now();
    let session = DocumentSession::open_file(&args.file, None, engine, None)?;
    let cold_open_dur = t0.elapsed();
    let cold_open_ms = cold_open_dur.as_secs_f64() * 1000.0;

    let total_pages = session.page_count()?;
    if total_pages == 0 {
        eprintln!("Error: Document has 0 pages.");
        std::process::exit(1);
    }

    // 2. Measure First Page Render at 150 DPI
    let (w_pt, h_pt) = session.page_dimensions(0)?;
    let scale = 150.0 / 72.0;
    let px_w = (w_pt * scale).round() as u32;
    let px_h = (h_pt * scale).round() as u32;

    let mut first_page_buf = vec![0u8; (px_w as usize) * (px_h as usize) * 4];
    let t1 = Instant::now();
    session.render_page_into_buffer(0, px_w, px_h, &mut first_page_buf)?;
    let first_page_dur = t1.elapsed();
    let first_page_render_ms = first_page_dur.as_secs_f64() * 1000.0;

    let total_open_and_render_ms = cold_open_ms + first_page_render_ms;
    let gate_300ms_passed = total_open_and_render_ms < 300.0;

    // 3. Measure 256x256 Tile Render Latency Across Pages
    let num_tiles = args.tiles.max(10);
    let mut tile_latencies_ms = Vec::with_capacity(num_tiles);
    let mut tile_buf = vec![0u8; 256 * 256 * 4];

    // Select tiles across different pages
    for i in 0..num_tiles {
        let page = (i as u32) % total_pages;
        let tile_x = ((i * 128) % 512) as u32;
        let tile_y = ((i * 128) % 512) as u32;

        let t_start = Instant::now();
        session.render_tile_into_rgba(page, tile_x, tile_y, 256, 256, 150.0, &mut tile_buf)?;
        let dur = t_start.elapsed();
        tile_latencies_ms.push(dur.as_secs_f64() * 1000.0);
    }

    tile_latencies_ms.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let tile_min_ms = tile_latencies_ms[0];
    let tile_max_ms = *tile_latencies_ms.last().unwrap();
    let tile_sum: f64 = tile_latencies_ms.iter().sum();
    let tile_avg_ms = tile_sum / (tile_latencies_ms.len() as f64);
    let p95_idx = ((tile_latencies_ms.len() as f64) * 0.95).floor() as usize;
    let tile_p95_ms = tile_latencies_ms[p95_idx.min(tile_latencies_ms.len() - 1)];

    // 60fps budget = 16.66ms per frame
    let tile_60fps_passed = tile_avg_ms < 16.66;

    let report = BenchmarkReport {
        file: args.file.display().to_string(),
        total_pages,
        engine: engine.name().to_string(),
        cold_open_ms,
        first_page_render_ms,
        first_page_open_and_render_ms: total_open_and_render_ms,
        gate_300ms_passed,
        tile_count: num_tiles,
        tile_min_ms,
        tile_avg_ms,
        tile_max_ms,
        tile_p95_ms,
        tile_60fps_passed,
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!(
            "================================================================================"
        );
        println!(
            "                     RBD OFİS - PERFORMANCE BENCHMARK REPORT                    "
        );
        println!(
            "================================================================================"
        );
        println!("Document   : {}", report.file);
        println!("Pages      : {}", report.total_pages);
        println!("Engine     : {}", report.engine);
        println!(
            "--------------------------------------------------------------------------------"
        );
        println!(
            "+-----------------------------------------+--------------------+----------------+"
        );
        println!(
            "| Metric                                  | Value              | Target Gate    |"
        );
        println!(
            "+-----------------------------------------+--------------------+----------------+"
        );
        println!(
            "| Cold Document Open Time                 | {:>15.2} ms |                |",
            cold_open_ms
        );
        println!(
            "| First-Page Render (150 DPI)             | {:>15.2} ms |                |",
            first_page_render_ms
        );
        println!(
            "| Total (Open + First-Page Render)        | {:>15.2} ms | < 300 ms   {} |",
            total_open_and_render_ms,
            if gate_300ms_passed { "✅" } else { "❌" }
        );
        println!(
            "+-----------------------------------------+--------------------+----------------+"
        );
        println!(
            "| Tile Render Latency (256x256, N={:<3})    |                    |                |",
            num_tiles
        );
        println!(
            "|   - Minimum                             | {:>15.3} ms |                |",
            tile_min_ms
        );
        println!(
            "|   - Average                             | {:>15.3} ms | < 16.6ms   {} |",
            tile_avg_ms,
            if tile_60fps_passed { "✅" } else { "❌" }
        );
        println!(
            "|   - 95th Percentile                     | {:>15.3} ms |                |",
            tile_p95_ms
        );
        println!(
            "|   - Maximum                             | {:>15.3} ms |                |",
            tile_max_ms
        );
        println!(
            "+-----------------------------------------+--------------------+----------------+"
        );
        println!();
        if gate_300ms_passed && tile_60fps_passed {
            println!(
                "🎉 ALL PHASE 0 GATES PASSED! (<300ms open+render, 60fps tile latency budget)"
            );
        } else {
            println!("⚠️ Some Phase 0 gate criteria were not met.");
        }
    }

    Ok(())
}
