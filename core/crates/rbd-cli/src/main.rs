use clap::{Args, Parser, Subcommand};
use image::ExtendedColorType;
use rbd_core::{DocumentSession, merge, rotate, split};
use rbd_engine::{PdfEngine, create_default_engine};
use serde::Serialize;
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
    /// Merge multiple PDF documents into one
    Merge(MergeArgs),
    /// Split a PDF by page range
    Split(SplitArgs),
    /// Rotate PDF pages
    Rotate(RotateArgs),
    /// Benchmark cold open, first-page render, and 256x256 tile rendering latency
    Bench(BenchArgs),
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let engine = create_default_engine();

    match cli.command {
        Commands::Info(args) => handle_info(&args.file, engine.as_ref())?,
        Commands::Render(args) => handle_render(&args, engine.as_ref())?,
        Commands::Text(args) => handle_text(&args, engine.as_ref())?,
        Commands::Merge(args) => handle_merge(&args)?,
        Commands::Split(args) => handle_split(&args)?,
        Commands::Rotate(args) => handle_rotate(&args)?,
        Commands::Bench(args) => handle_bench(&args, engine.as_ref())?,
    }

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
    if !args.json {
        println!(
            "================================================================================"
        );
        println!(
            "                 RBD OFİS — PDF Engine & Rendering Benchmark                    "
        );
        println!(
            "================================================================================"
        );
        println!("Target File: {}", args.file.display());
        println!("Engine:      {}", engine.name());
    }

    // 1. Measure Cold Open Time
    let t0 = Instant::now();
    let session = DocumentSession::open_file(&args.file, None, engine, None)?;
    let cold_open_dur = t0.elapsed();
    let cold_open_ms = cold_open_dur.as_secs_f64() * 1000.0;

    let total_pages = session.page_count()?;

    // 2. Measure First-Page Render at 150 DPI
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
        println!("Total Pages: {}", total_pages);
        println!();
        println!(
            "+-----------------------------------------+--------------------+----------------+"
        );
        println!(
            "| Metric                                  | Result             | Target Gate    |"
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
