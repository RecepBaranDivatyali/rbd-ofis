use crate::annotation::Annotation;
use crate::error::EngineError;
use crate::search::{PdfRect, SearchHit};
use crate::traits::{DocumentMetadata, PdfDocument, PdfEngine, PdfPage};
use pdfium_render::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

static PDFIUM: OnceLock<Pdfium> = OnceLock::new();
static INIT_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Discover and initialize the global Pdfium library instance.
pub fn get_or_init_pdfium() -> Result<&'static Pdfium, EngineError> {
    if let Some(pdfium) = PDFIUM.get() {
        return Ok(pdfium);
    }

    let _lock = INIT_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(pdfium) = PDFIUM.get() {
        return Ok(pdfium);
    }

    // Try multiple candidate directories and platform library names (pdfium.dll / libpdfium.so / libpdfium.dylib)
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(env_path) = std::env::var("PDFIUM_PATH") {
        let p = PathBuf::from(&env_path);
        if p.is_dir() {
            candidates.push(Pdfium::pdfium_platform_library_name_at_path(&p));
        }
        candidates.push(p);
    }

    let mut search_dirs: Vec<PathBuf> = Vec::new();
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let m = PathBuf::from(&manifest_dir);
        search_dirs.push(m.join("../../vendor/pdfium/bin"));
        search_dirs.push(m.join("../../vendor/pdfium/lib"));
        search_dirs.push(m.join("vendor/pdfium/bin"));
        search_dirs.push(m.join("vendor/pdfium/lib"));
    }

    if let Ok(exe_path) = std::env::current_exe()
        && let Some(parent) = exe_path.parent()
    {
        search_dirs.push(parent.to_path_buf());
        search_dirs.push(parent.join("vendor/pdfium/bin"));
        search_dirs.push(parent.join("../../vendor/pdfium/bin"));
        search_dirs.push(parent.join("../../../vendor/pdfium/bin"));
        search_dirs.push(parent.join("../../../../vendor/pdfium/bin"));
        search_dirs.push(parent.join("../../../../../core/vendor/pdfium/bin"));
        search_dirs.push(parent.join("../../../../../../core/vendor/pdfium/bin"));
    }

    for rel in [
        "core/vendor/pdfium/bin",
        "core/vendor/pdfium/lib",
        "../core/vendor/pdfium/bin",
        "../../core/vendor/pdfium/bin",
        "../../../core/vendor/pdfium/bin",
        "../../../../core/vendor/pdfium/bin",
        "../../../../../core/vendor/pdfium/bin",
        "../vendor/pdfium/bin",
        "../../vendor/pdfium/bin",
        "vendor/pdfium/bin",
        ".",
        "runtimes/win-x64/native",
    ] {
        search_dirs.push(PathBuf::from(rel));
    }

    for dir in search_dirs {
        candidates.push(Pdfium::pdfium_platform_library_name_at_path(&dir));
    }

    let mut load_error = None;

    for candidate in candidates {
        if candidate.exists() {
            match Pdfium::bind_to_library(&candidate) {
                Ok(bindings) => {
                    let pdfium = Pdfium::new(bindings);
                    let _ = PDFIUM.set(pdfium);
                    return PDFIUM.get().ok_or_else(|| {
                        EngineError::Internal("Failed to retrieve initialized Pdfium".into())
                    });
                }
                Err(err) => {
                    if let Some(pdfium) = PDFIUM.get() {
                        return Ok(pdfium);
                    }
                    load_error = Some(err);
                }
            }
        }
    }

    // Try system library
    match Pdfium::bind_to_system_library() {
        Ok(bindings) => {
            let pdfium = Pdfium::new(bindings);
            let _ = PDFIUM.set(pdfium);
            PDFIUM.get().ok_or_else(|| {
                EngineError::Internal("Failed to retrieve initialized Pdfium".into())
            })
        }
        Err(err) => {
            if let Some(pdfium) = PDFIUM.get() {
                return Ok(pdfium);
            }
            let detail = if let Some(first_err) = load_error {
                format!("{first_err:?} / {err:?}")
            } else {
                format!("{err:?}")
            };
            Err(EngineError::Internal(format!(
                "Failed to bind to pdfium library: {detail}"
            )))
        }
    }
}

/// PDFium-backed PDF Engine implementation.
pub struct PdfiumEngine {
    pdfium: &'static Pdfium,
}

impl PdfiumEngine {
    /// Create a new `PdfiumEngine`, initializing the library if necessary.
    pub fn new() -> Result<Self, EngineError> {
        let pdfium = get_or_init_pdfium()?;
        Ok(Self { pdfium })
    }
}

impl PdfEngine for PdfiumEngine {
    fn name(&self) -> &'static str {
        "PDFium"
    }

    fn open_from_file(
        &self,
        path: &Path,
        password: Option<&str>,
    ) -> Result<Box<dyn PdfDocument>, EngineError> {
        if !path.exists() {
            return Err(EngineError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", path.display()),
            )));
        }

        let doc = self
            .pdfium
            .load_pdf_from_file(path, password)
            .map_err(map_pdfium_error)?;

        let lopdf_doc = std::fs::read(path)
            .ok()
            .and_then(|bytes| lopdf::Document::load_mem(&bytes).ok())
            .map(Arc::new);

        Ok(Box::new(PdfiumDocumentWrapper {
            doc: Arc::new(doc),
            lopdf_doc,
        }))
    }

    fn open_from_bytes(
        &self,
        bytes: &[u8],
        password: Option<&str>,
    ) -> Result<Box<dyn PdfDocument>, EngineError> {
        let doc = self
            .pdfium
            .load_pdf_from_byte_vec(bytes.to_vec(), password)
            .map_err(map_pdfium_error)?;

        let lopdf_doc = lopdf::Document::load_mem(bytes).ok().map(Arc::new);

        Ok(Box::new(PdfiumDocumentWrapper {
            doc: Arc::new(doc),
            lopdf_doc,
        }))
    }
}

struct PdfiumDocumentWrapper {
    doc: Arc<pdfium_render::prelude::PdfDocument<'static>>,
    lopdf_doc: Option<Arc<lopdf::Document>>,
}

impl PdfDocument for PdfiumDocumentWrapper {
    fn page_count(&self) -> Result<u32, EngineError> {
        Ok(self.doc.pages().len() as u32)
    }

    fn page(&self, index: u32) -> Result<Box<dyn PdfPage>, EngineError> {
        let total = self.page_count()?;
        if index >= total {
            return Err(EngineError::PageOutOfRange {
                index,
                total_pages: total,
            });
        }

        let page = self
            .doc
            .pages()
            .get(index as i32)
            .map_err(map_pdfium_error)?;

        Ok(Box::new(PdfiumPageWrapper {
            _doc: self.doc.clone(),
            page,
            page_index: index,
            lopdf_doc: self.lopdf_doc.clone(),
        }))
    }

    fn metadata(&self) -> Result<DocumentMetadata, EngineError> {
        let meta = self.doc.metadata();
        let get_val = |key: PdfDocumentMetadataTagType| -> Option<String> {
            meta.get(key)
                .map(|val| val.value().to_string())
                .filter(|s| !s.trim().is_empty())
        };

        Ok(DocumentMetadata {
            title: get_val(PdfDocumentMetadataTagType::Title),
            author: get_val(PdfDocumentMetadataTagType::Author),
            subject: get_val(PdfDocumentMetadataTagType::Subject),
            keywords: get_val(PdfDocumentMetadataTagType::Keywords),
            creator: get_val(PdfDocumentMetadataTagType::Creator),
            producer: get_val(PdfDocumentMetadataTagType::Producer),
            creation_date: get_val(PdfDocumentMetadataTagType::CreationDate),
            mod_date: get_val(PdfDocumentMetadataTagType::ModificationDate),
        })
    }
}

struct PdfiumPageWrapper {
    _doc: Arc<pdfium_render::prelude::PdfDocument<'static>>,
    page: pdfium_render::prelude::PdfPage<'static>,
    page_index: u32,
    lopdf_doc: Option<Arc<lopdf::Document>>,
}

impl PdfPage for PdfiumPageWrapper {
    fn dimensions_points(&self) -> Result<(f32, f32), EngineError> {
        let w = self.page.width().value;
        let h = self.page.height().value;
        Ok((w, h))
    }

    fn render_to_rgba(
        &self,
        width_px: u32,
        height_px: u32,
        out_buf: &mut [u8],
    ) -> Result<(), EngineError> {
        let required_len = (width_px as usize) * (height_px as usize) * 4;
        if out_buf.len() < required_len {
            return Err(EngineError::RenderError(format!(
                "Buffer too small: provided {} bytes, required {} bytes",
                out_buf.len(),
                required_len
            )));
        }

        let config = PdfRenderConfig::new()
            .set_target_width(width_px as i32)
            .set_target_height(height_px as i32);

        let mut bitmap = PdfBitmap::from_bytes(
            width_px as i32,
            height_px as i32,
            PdfBitmapFormat::BGRA,
            &mut out_buf[..required_len],
        )
        .map_err(map_pdfium_error)?;

        self.page
            .render_into_bitmap_with_config(&mut bitmap, &config)
            .map_err(map_pdfium_error)?;

        Ok(())
    }

    fn render_tile_to_buffer(
        &self,
        tile_x: u32,
        tile_y: u32,
        tile_width: u32,
        tile_height: u32,
        scale_dpi: f32,
        format: crate::traits::PixelFormat,
        stride: usize,
        out_buf: &mut [u8],
    ) -> Result<(), EngineError> {
        let row_bytes = (tile_width as usize) * 4;
        if stride < row_bytes {
            return Err(EngineError::InvalidParameter(format!(
                "Stride {stride} is smaller than row bytes {row_bytes}"
            )));
        }
        let total_required = stride * (tile_height as usize);
        if out_buf.len() < total_required {
            return Err(EngineError::BufferTooSmall {
                provided: out_buf.len(),
                required: total_required,
            });
        }

        let scale = scale_dpi / 72.0;
        let config = PdfRenderConfig::new()
            .scale_page_by_factor(scale)
            .set_origin(-(tile_x as i32), -(tile_y as i32));

        if stride == row_bytes {
            // Contiguous rows: render directly into out_buf (zero-copy)
            let mut bitmap = PdfBitmap::from_bytes(
                tile_width as i32,
                tile_height as i32,
                PdfBitmapFormat::BGRA,
                &mut out_buf[..total_required],
            )
            .map_err(map_pdfium_error)?;

            self.page
                .render_into_bitmap_with_config(&mut bitmap, &config)
                .map_err(map_pdfium_error)?;
            drop(bitmap);

            if format == crate::traits::PixelFormat::Rgba32 {
                for pixel in out_buf[..total_required].as_chunks_mut::<4>().0 {
                    pixel.swap(0, 2); // BGRA -> RGBA
                }
            }
        } else {
            // Non-contiguous rows: render into contiguous buffer, then copy row-by-row
            let contiguous_len = row_bytes * (tile_height as usize);
            let mut temp_buf = vec![0u8; contiguous_len];

            let mut bitmap = PdfBitmap::from_bytes(
                tile_width as i32,
                tile_height as i32,
                PdfBitmapFormat::BGRA,
                &mut temp_buf,
            )
            .map_err(map_pdfium_error)?;

            self.page
                .render_into_bitmap_with_config(&mut bitmap, &config)
                .map_err(map_pdfium_error)?;
            drop(bitmap);

            for y in 0..(tile_height as usize) {
                let src_row = &temp_buf[y * row_bytes..(y + 1) * row_bytes];
                let dst_row = &mut out_buf[y * stride..y * stride + row_bytes];
                if format == crate::traits::PixelFormat::Rgba32 {
                    for (src, dst) in src_row
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .zip(dst_row.as_chunks_mut::<4>().0)
                    {
                        dst[0] = src[2]; // R
                        dst[1] = src[1]; // G
                        dst[2] = src[0]; // B
                        dst[3] = src[3]; // A
                    }
                } else {
                    dst_row.copy_from_slice(src_row);
                }
            }
        }

        Ok(())
    }

    fn extract_text(&self) -> Result<String, EngineError> {
        let text_page = self.page.text().map_err(map_pdfium_error)?;
        Ok(text_page.all())
    }

    fn search_text(
        &self,
        page_index: u32,
        query: &str,
        case_sensitive: bool,
    ) -> Result<Vec<SearchHit>, EngineError> {
        let text_page = self.page.text().map_err(map_pdfium_error)?;
        let chars = text_page.chars();
        let total_chars = chars.len();

        let mut page_chars = Vec::with_capacity(total_chars);
        let mut char_boxes: Vec<Option<PdfRect>> = Vec::with_capacity(total_chars);

        for i in 0..total_chars {
            if let Ok(ch_obj) = chars.get(i) {
                let c = ch_obj.unicode_char().unwrap_or(' ');
                let rect = ch_obj.tight_bounds().ok().map(|r| {
                    PdfRect::new(
                        r.left().value,
                        r.bottom().value,
                        r.right().value,
                        r.top().value,
                    )
                });
                page_chars.push(c);
                char_boxes.push(rect);
            }
        }

        let page_text: String = page_chars.iter().collect();
        let match_spans = crate::search::find_matches(&page_text, query, case_sensitive);

        let mut hits = Vec::with_capacity(match_spans.len());
        for span in match_spans {
            let match_text: String = page_chars[span.start..span.end].iter().collect();
            let snippet = crate::search::build_snippet(&page_chars, span.start, span.end, 30);

            let span_boxes: Vec<PdfRect> = char_boxes[span.start..span.end]
                .iter()
                .flatten()
                .copied()
                .collect();

            let rects = if span_boxes.is_empty() {
                let (w, h) = self.dimensions_points()?;
                vec![PdfRect::new(0.0, 0.0, w, h)]
            } else {
                crate::search::group_char_rects_into_lines(&span_boxes)
            };

            hits.push(SearchHit {
                page_index,
                match_text,
                snippet,
                rects,
            });
        }

        Ok(hits)
    }

    fn list_annotations(&self) -> Result<Vec<Annotation>, EngineError> {
        if let Some(ref doc) = self.lopdf_doc {
            crate::annotation::list_annotations_from_doc(doc, self.page_index)
        } else {
            Ok(Vec::new())
        }
    }
}

fn map_pdfium_error(err: PdfiumError) -> EngineError {
    match err {
        PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::PasswordError) => {
            EngineError::Encrypted("Invalid or missing password".into())
        }
        PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::FormatError)
        | PdfiumError::PdfiumLibraryInternalError(PdfiumInternalError::FileError) => {
            EngineError::Corrupt(format!("{err:?}"))
        }
        PdfiumError::PageIndexOutOfBounds => EngineError::PageOutOfRange {
            index: 0,
            total_pages: 0,
        },
        PdfiumError::IoError(io) => EngineError::Io(io),
        PdfiumError::ImageBufferTooSmall | PdfiumError::ImageSizeOutOfBounds => {
            EngineError::RenderError(err.to_string())
        }
        _ => {
            let msg = err.to_string();
            if msg.contains("password") || msg.contains("Password") {
                EngineError::Encrypted(msg)
            } else if msg.contains("format") || msg.contains("corrupt") {
                EngineError::Corrupt(msg)
            } else {
                EngineError::Internal(msg)
            }
        }
    }
}
