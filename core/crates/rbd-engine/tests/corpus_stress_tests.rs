use rbd_engine::{EngineType, create_engine};
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;

fn get_corpus_dir() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("../../../corpus/files")
}

const PDF_CATEGORIES: &[&str] = &[
    "pdf-basic",
    "pdf-large",
    "pdf-encrypted",
    "pdf-corrupt",
    "pdf-forms",
    "pdf-annotations",
    "pdf-scanned",
    "pdf-pdfa",
    "pdf-signed",
    "pdf-cjk-rtl-turkish",
];

struct CategoryStats {
    total: usize,
    passed: usize,
    graceful_err: usize,
    panicked: usize,
}

#[test]
fn test_entire_pdf_corpus_zero_panics() {
    let corpus_dir = get_corpus_dir();
    assert!(
        corpus_dir.exists(),
        "Corpus files directory must exist at: {}",
        corpus_dir.display()
    );

    let engine = create_engine(EngineType::Pdfium).expect("Failed to initialize PDFium engine");

    let mut all_files: Vec<(String, PathBuf)> = Vec::new();

    for &category in PDF_CATEGORIES {
        let cat_dir = corpus_dir.join(category);
        if !cat_dir.exists() {
            continue;
        }

        let entries = fs::read_dir(&cat_dir).unwrap_or_else(|e| {
            panic!("Failed to read category dir {}: {:?}", cat_dir.display(), e)
        });

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|s| s.eq_ignore_ascii_case("pdf"))
                    .unwrap_or(false)
            {
                all_files.push((category.to_string(), path));
            }
        }
    }

    all_files.sort_by(|a, b| a.1.cmp(&b.1));

    let total_count = all_files.len();
    println!("\nFound {} PDF files across 10 categories.", total_count);
    assert!(
        total_count >= 115,
        "Expected at least 115 PDF files, but found {}",
        total_count
    );

    let mut category_results: std::collections::BTreeMap<String, CategoryStats> =
        std::collections::BTreeMap::new();

    for &category in PDF_CATEGORIES {
        category_results.insert(
            category.to_string(),
            CategoryStats {
                total: 0,
                passed: 0,
                graceful_err: 0,
                panicked: 0,
            },
        );
    }

    let mut total_panicked = 0usize;

    for (category, file_path) in &all_files {
        let stats = category_results.get_mut(category).unwrap();
        stats.total += 1;

        let is_corrupt_or_encrypted = category == "pdf-corrupt" || category == "pdf-encrypted";
        let path_clone = file_path.clone();

        let panic_result = catch_unwind(AssertUnwindSafe(|| {
            match engine.open_from_file(&path_clone, None) {
                Ok(doc) => {
                    let page_count = match doc.page_count() {
                        Ok(cnt) => cnt,
                        Err(e) => {
                            if is_corrupt_or_encrypted {
                                return false;
                            } else {
                                panic!(
                                    "page_count() failed on valid PDF {:?}: {:?}",
                                    path_clone, e
                                );
                            }
                        }
                    };

                    if !is_corrupt_or_encrypted {
                        assert!(
                            page_count >= 1,
                            "Page count must be >= 1 for valid PDF {:?}",
                            path_clone
                        );
                    }

                    if page_count >= 1 {
                        let page = match doc.page(0) {
                            Ok(p) => p,
                            Err(e) => {
                                if is_corrupt_or_encrypted {
                                    return false;
                                } else {
                                    panic!("page(0) failed on valid PDF {:?}: {:?}", path_clone, e);
                                }
                            }
                        };

                        let (w, h) = match page.dimensions_points() {
                            Ok(dims) => dims,
                            Err(e) => {
                                if is_corrupt_or_encrypted {
                                    return false;
                                } else {
                                    panic!(
                                        "dimensions_points() failed on valid PDF {:?}: {:?}",
                                        path_clone, e
                                    );
                                }
                            }
                        };

                        if !is_corrupt_or_encrypted {
                            assert!(
                                w > 0.0 && h > 0.0,
                                "Dimensions must be positive for valid PDF {:?}: ({}, {})",
                                path_clone,
                                w,
                                h
                            );
                        }

                        // Text extraction must not panic
                        let _ = page.extract_text();

                        // Tile rendering 128x128 at 72.0 DPI
                        let mut tile_buf = vec![0u8; 128 * 128 * 4];
                        let render_res =
                            page.render_tile_to_rgba(0, 0, 128, 128, 72.0, &mut tile_buf);

                        if !is_corrupt_or_encrypted {
                            assert!(
                                render_res.is_ok(),
                                "render_tile_to_rgba failed on valid PDF {:?}: {:?}",
                                path_clone,
                                render_res.err()
                            );
                        }
                    }

                    true
                }
                Err(e) => {
                    if is_corrupt_or_encrypted {
                        false
                    } else {
                        panic!(
                            "open_from_file failed unexpectedly on valid PDF {:?}: {:?}",
                            path_clone, e
                        );
                    }
                }
            }
        }));

        match panic_result {
            Ok(true) => {
                stats.passed += 1;
            }
            Ok(false) => {
                stats.graceful_err += 1;
            }
            Err(_) => {
                stats.panicked += 1;
                total_panicked += 1;
                eprintln!("CRITICAL PANIC on file: {}", file_path.display());
            }
        }
    }

    println!("\n==========================================================================");
    println!("CORPUS STRESS SUITE: 119 PDF FILES ZERO-PANIC VERIFICATION TABLE");
    println!("==========================================================================");
    println!(
        "{:<25} | {:>6} | {:>8} | {:>14} | {:>8}",
        "Category", "Total", "Passed", "Graceful Err", "Panicked"
    );
    println!("--------------------------------------------------------------------------");

    let mut sum_total = 0;
    let mut sum_passed = 0;
    let mut sum_err = 0;
    let mut sum_panic = 0;

    for (cat, stats) in &category_results {
        println!(
            "{:<25} | {:>6} | {:>8} | {:>14} | {:>8}",
            cat, stats.total, stats.passed, stats.graceful_err, stats.panicked
        );
        sum_total += stats.total;
        sum_passed += stats.passed;
        sum_err += stats.graceful_err;
        sum_panic += stats.panicked;
    }

    println!("==========================================================================");
    println!(
        "{:<25} | {:>6} | {:>8} | {:>14} | {:>8}",
        "TOTAL", sum_total, sum_passed, sum_err, sum_panic
    );
    println!("==========================================================================\n");

    assert_eq!(
        total_panicked, 0,
        "Expected ZERO panics across all {} PDF files, but found {}",
        sum_total, total_panicked
    );
}

/// Deterministic Pseudo-Random Number Generator (Xorshift64).
struct Xorshift64 {
    state: u64,
}

impl Xorshift64 {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0x524244_4F464953 } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn next_usize(&mut self, max: usize) -> usize {
        if max == 0 {
            0
        } else {
            (self.next_u64() as usize) % max
        }
    }

    fn next_range(&mut self, min: usize, max: usize) -> usize {
        if min >= max {
            min
        } else {
            min + self.next_usize(max - min + 1)
        }
    }
}

fn mutate_strategy_bit_flips(buf: &mut [u8], prng: &mut Xorshift64) {
    if buf.is_empty() {
        return;
    }
    let num_flips = prng.next_range(1, 4);
    for _ in 0..num_flips {
        let idx = prng.next_usize(buf.len());
        let bit = prng.next_usize(8);
        buf[idx] ^= 1 << bit;
    }
}

fn mutate_strategy_truncation(buf: &mut Vec<u8>, prng: &mut Xorshift64) {
    if buf.len() < 20 {
        return;
    }
    let pct = prng.next_range(10, 95);
    let keep_len = ((buf.len() * pct) / 100).max(10);
    buf.truncate(keep_len);
}

fn mutate_strategy_zero_slice(buf: &mut [u8], prng: &mut Xorshift64) {
    if buf.len() < 16 {
        return;
    }
    let slice_len = prng.next_range(4, 64).min(buf.len() / 2);
    let start = prng.next_usize(buf.len() - slice_len);
    buf[start..start + slice_len].fill(0x00);
}

fn mutate_strategy_corrupt_literals(buf: &mut Vec<u8>, prng: &mut Xorshift64) {
    let targets: &[&[u8]] = &[b"/Size", b"startxref", b"/Length", b"/Count"];
    let target = targets[prng.next_usize(targets.len())];

    let mut found_pos = None;
    if buf.len() >= target.len() {
        let max_search = buf.len() - target.len();
        let search_start = prng.next_usize(max_search + 1);

        // Search starting from a random offset, wrap around
        for i in 0..max_search {
            let pos = (search_start + i) % max_search;
            if &buf[pos..pos + target.len()] == target {
                found_pos = Some(pos + target.len());
                break;
            }
        }
    }

    if let Some(mut pos) = found_pos {
        while pos < buf.len() && (buf[pos] == b' ' || buf[pos] == b'\r' || buf[pos] == b'\n') {
            pos += 1;
        }

        let corrupt_payloads: &[&[u8]] = &[
            b"999999999",
            b"-999999",
            b"0",
            b"-1",
            b"//CORRUPT//",
            b"NaN",
            b"99999999999999999999999999999999999999999999999",
        ];
        let payload = corrupt_payloads[prng.next_usize(corrupt_payloads.len())];

        let mut digit_end = pos;
        while digit_end < buf.len() && buf[digit_end].is_ascii_digit() {
            digit_end += 1;
        }

        if digit_end > pos {
            buf.splice(pos..digit_end, payload.iter().copied());
        } else if pos < buf.len() {
            buf.splice(pos..pos, payload.iter().copied());
        }
    } else {
        mutate_strategy_bit_flips(buf, prng);
    }
}

#[test]
fn test_deterministic_fuzzer_1000_mutations_zero_panics() {
    let corpus_dir = get_corpus_dir();

    let seed_paths = [
        corpus_dir.join("pdf-basic/pdfium-hello-world.pdf"),
        corpus_dir.join("pdf-cjk-rtl-turkish/tr-generated-standard-fonts.pdf"),
        corpus_dir.join("pdf-forms/pdfium-combobox-form.pdf"),
        corpus_dir.join("pdf-annotations/pdfium-annot-highlight-long.pdf"),
        corpus_dir.join("pdf-basic/pdfjs-basicapi.pdf"),
    ];

    let mut seeds: Vec<(String, Vec<u8>)> = Vec::new();
    for path in &seed_paths {
        assert!(path.exists(), "Seed PDF must exist: {}", path.display());
        let bytes = fs::read(path).unwrap_or_else(|e| {
            panic!("Failed to read seed PDF {}: {:?}", path.display(), e);
        });
        seeds.push((
            path.file_name().unwrap().to_string_lossy().to_string(),
            bytes,
        ));
    }

    let pdfium = create_engine(EngineType::Pdfium).expect("PDFium engine initialization");
    let hayro = create_engine(EngineType::PureRust).expect("Hayro engine initialization");

    let mut prng = Xorshift64::new(0x524244_4F464953);

    let total_iterations = 1000usize;
    let mut pdfium_opened = 0usize;
    let mut hayro_opened = 0usize;
    let mut pdfium_panics = 0usize;
    let mut hayro_panics = 0usize;

    println!(
        "\nStarting deterministic fuzz test: {} mutations across 5 seeds...",
        total_iterations
    );

    for i in 0..total_iterations {
        let seed_idx = prng.next_usize(seeds.len());
        let strategy = i % 4; // 0..=3 mutation strategies
        let mut mutated = seeds[seed_idx].1.clone();

        match strategy {
            0 => mutate_strategy_bit_flips(&mut mutated, &mut prng),
            1 => mutate_strategy_truncation(&mut mutated, &mut prng),
            2 => mutate_strategy_zero_slice(&mut mutated, &mut prng),
            3 => mutate_strategy_corrupt_literals(&mut mutated, &mut prng),
            _ => unreachable!(),
        }

        // Test with PDFium
        let pdfium_result = catch_unwind(AssertUnwindSafe(|| {
            if let Ok(doc) = pdfium.open_from_bytes(&mutated, None) {
                let _ = doc.page_count();
                if let Ok(page) = doc.page(0) {
                    let _ = page.dimensions_points();
                    let _ = page.extract_text();
                    let mut tile = vec![0u8; 64 * 64 * 4];
                    let _ = page.render_tile_to_rgba(0, 0, 64, 64, 72.0, &mut tile);
                }
                true
            } else {
                false
            }
        }));

        match pdfium_result {
            Ok(opened) => {
                if opened {
                    pdfium_opened += 1;
                }
            }
            Err(_) => {
                pdfium_panics += 1;
                eprintln!(
                    "FUZZ PANIC (PDFium): iteration {}, seed {}",
                    i, seeds[seed_idx].0
                );
            }
        }

        // Test with Hayro (Pure-Rust)
        let hayro_result = catch_unwind(AssertUnwindSafe(|| {
            if let Ok(doc) = hayro.open_from_bytes(&mutated, None) {
                let _ = doc.page_count();
                if let Ok(page) = doc.page(0) {
                    let _ = page.dimensions_points();
                    let _ = page.extract_text();
                    let mut tile = vec![0u8; 64 * 64 * 4];
                    let _ = page.render_tile_to_rgba(0, 0, 64, 64, 72.0, &mut tile);
                }
                true
            } else {
                false
            }
        }));

        match hayro_result {
            Ok(opened) => {
                if opened {
                    hayro_opened += 1;
                }
            }
            Err(_) => {
                hayro_panics += 1;
                eprintln!(
                    "FUZZ PANIC (Hayro): iteration {}, seed {}",
                    i, seeds[seed_idx].0
                );
            }
        }
    }

    println!("\n==========================================================================");
    println!("DETERMINISTIC FUZZ HARNESS (1,000 MUTATIONS, PRNG 0x524244_4F464953)");
    println!("==========================================================================");
    println!("Engine               | Mutations | Opened | Graceful Rejection | Panics");
    println!("--------------------------------------------------------------------------");
    println!(
        "{:<20} | {:>9} | {:>6} | {:>18} | {:>6}",
        "PDFium",
        total_iterations,
        pdfium_opened,
        total_iterations - pdfium_opened,
        pdfium_panics
    );
    println!(
        "{:<20} | {:>9} | {:>6} | {:>18} | {:>6}",
        "Hayro (Pure-Rust)",
        total_iterations,
        hayro_opened,
        total_iterations - hayro_opened,
        hayro_panics
    );
    println!("==========================================================================\n");

    assert_eq!(
        pdfium_panics, 0,
        "PDFium panicked on {} mutated inputs!",
        pdfium_panics
    );
    assert_eq!(
        hayro_panics, 0,
        "Hayro panicked on {} mutated inputs!",
        hayro_panics
    );
}
