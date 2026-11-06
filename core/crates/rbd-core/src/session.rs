use crate::command::{
    AddAnnotationCommand, Command, CommandStack, RemoveAnnotationCommand, UpdateAnnotationCommand,
};
use crate::error::CoreError;
use crate::tile_cache::{TileCache, TileKey};
use rbd_engine::{Annotation, DocumentMetadata, PdfDocument, PdfEngine, PixelFormat, SearchHit};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

static SESSION_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// A stateful document session representing an opened PDF file.
pub struct DocumentSession {
    doc_id: u64,
    doc: Box<dyn PdfDocument>,
    tile_cache: Arc<TileCache>,
    annotations: Vec<Annotation>,
    command_stack: CommandStack,
    source_path: Option<PathBuf>,
}

impl DocumentSession {
    /// Open a document from a file path using the supplied engine.
    pub fn open_file(
        path: &Path,
        password: Option<&str>,
        engine: &dyn PdfEngine,
        tile_cache: Option<Arc<TileCache>>,
    ) -> Result<Self, CoreError> {
        let doc = engine.open_from_file(path, password)?;
        let doc_id = SESSION_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let cache = tile_cache.unwrap_or_else(|| Arc::new(TileCache::default()));

        let mut annotations = Vec::new();
        if let Ok(count) = doc.page_count() {
            for p in 0..count {
                if let Ok(page_obj) = doc.page(p)
                    && let Ok(page_annots) = page_obj.list_annotations()
                {
                    annotations.extend(page_annots);
                }
            }
        }

        Ok(Self {
            doc_id,
            doc,
            tile_cache: cache,
            annotations,
            command_stack: CommandStack::default(),
            source_path: Some(path.to_path_buf()),
        })
    }

    /// Open a document from memory bytes using the supplied engine.
    pub fn open_bytes(
        bytes: &[u8],
        password: Option<&str>,
        engine: &dyn PdfEngine,
        tile_cache: Option<Arc<TileCache>>,
    ) -> Result<Self, CoreError> {
        let doc = engine.open_from_bytes(bytes, password)?;
        let doc_id = SESSION_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let cache = tile_cache.unwrap_or_else(|| Arc::new(TileCache::default()));

        let mut annotations = Vec::new();
        if let Ok(count) = doc.page_count() {
            for p in 0..count {
                if let Ok(page_obj) = doc.page(p)
                    && let Ok(page_annots) = page_obj.list_annotations()
                {
                    annotations.extend(page_annots);
                }
            }
        }

        Ok(Self {
            doc_id,
            doc,
            tile_cache: cache,
            annotations,
            command_stack: CommandStack::default(),
            source_path: None,
        })
    }

    /// Return the source file path on disk if this session was opened from a file.
    pub fn source_path(&self) -> Option<&Path> {
        self.source_path.as_deref()
    }

    /// Unique session / document identifier.
    pub fn doc_id(&self) -> u64 {
        self.doc_id
    }

    /// Access the tile cache attached to this session.
    pub fn tile_cache(&self) -> &Arc<TileCache> {
        &self.tile_cache
    }

    /// Total number of pages in the document.
    pub fn page_count(&self) -> Result<u32, CoreError> {
        Ok(self.doc.page_count()?)
    }

    /// Dimensions of the specified 0-indexed page in PDF points.
    pub fn page_dimensions(&self, page: u32) -> Result<(f32, f32), CoreError> {
        let page_obj = self.doc.page(page)?;
        Ok(page_obj.dimensions_points()?)
    }

    /// Extract metadata from the document.
    pub fn metadata(&self) -> Result<DocumentMetadata, CoreError> {
        Ok(self.doc.metadata()?)
    }

    /// Extract text from the specified 0-indexed page (preserving Turkish unicode characters).
    pub fn extract_text(&self, page: u32) -> Result<String, CoreError> {
        let page_obj = self.doc.page(page)?;
        Ok(page_obj.extract_text()?)
    }

    /// Render a full page into a caller-provided RGBA8 buffer.
    pub fn render_page_into_buffer(
        &self,
        page: u32,
        width: u32,
        height: u32,
        out_buf: &mut [u8],
    ) -> Result<(), CoreError> {
        let page_obj = self.doc.page(page)?;
        page_obj.render_to_rgba(width, height, out_buf)?;
        Ok(())
    }

    /// Render a sub-rectangle tile `(tile_x, tile_y, tile_w, tile_h)` at `scale_dpi`
    /// directly into a caller-provided RGBA8 buffer.
    ///
    /// Utilizes the LRU tile cache when available to eliminate redundant rasterization!
    #[allow(clippy::too_many_arguments)]
    pub fn render_tile_into_buffer(
        &self,
        page: u32,
        tile_x: u32,
        tile_y: u32,
        tile_w: u32,
        tile_h: u32,
        scale_dpi: f32,
        format: PixelFormat,
        stride: usize,
        out_buf: &mut [u8],
    ) -> Result<(), CoreError> {
        let row_bytes = (tile_w as usize) * 4;
        if stride < row_bytes {
            return Err(CoreError::Internal(format!(
                "Stride {stride} is smaller than row bytes {row_bytes}"
            )));
        }
        let total_required = stride * (tile_h as usize);
        if out_buf.len() < total_required {
            return Err(CoreError::BufferTooSmall {
                provided: out_buf.len(),
                required: total_required,
            });
        }

        // Discrete zoom bucket for cache quantization (0.1 DPI precision)
        let zoom_bucket = (scale_dpi * 10.0).round() as u32;

        let key = TileKey {
            doc_id: self.doc_id,
            page,
            zoom_bucket,
            tile_x,
            tile_y,
            pixel_format: format as u8,
        };

        let contiguous_bytes = row_bytes * (tile_h as usize);

        // Cache hit: copy cached pixels into out_buf (respecting stride)
        if let Some(cached_pixels) = self.tile_cache.get(&key)
            && cached_pixels.len() == contiguous_bytes
        {
            if stride == row_bytes {
                out_buf[..contiguous_bytes].copy_from_slice(&cached_pixels);
            } else {
                for y in 0..(tile_h as usize) {
                    let src = &cached_pixels[y * row_bytes..(y + 1) * row_bytes];
                    out_buf[y * stride..y * stride + row_bytes].copy_from_slice(src);
                }
            }
            return Ok(());
        }

        // Cache miss: render tile directly into caller's out_buf
        let page_obj = self.doc.page(page)?;
        page_obj.render_tile_to_buffer(
            tile_x, tile_y, tile_w, tile_h, scale_dpi, format, stride, out_buf,
        )?;

        // Populate the cache with an Arc of contiguous pixels
        let mut cached_vec = vec![0u8; contiguous_bytes];
        if stride == row_bytes {
            cached_vec.copy_from_slice(&out_buf[..contiguous_bytes]);
        } else {
            for y in 0..(tile_h as usize) {
                let src = &out_buf[y * stride..y * stride + row_bytes];
                cached_vec[y * row_bytes..(y + 1) * row_bytes].copy_from_slice(src);
            }
        }
        self.tile_cache.insert(key, Arc::new(cached_vec));

        Ok(())
    }

    /// Convenience wrapper for rendering into contiguous RGBA buffer.
    #[allow(clippy::too_many_arguments)]
    pub fn render_tile_into_rgba(
        &self,
        page: u32,
        tile_x: u32,
        tile_y: u32,
        tile_w: u32,
        tile_h: u32,
        scale_dpi: f32,
        out_buf: &mut [u8],
    ) -> Result<(), CoreError> {
        self.render_tile_into_buffer(
            page,
            tile_x,
            tile_y,
            tile_w,
            tile_h,
            scale_dpi,
            PixelFormat::Rgba32,
            (tile_w as usize) * 4,
            out_buf,
        )
    }

    /// Search for text across all pages of the document, returning hits with bounding boxes.
    pub fn search_document(
        &self,
        query: &str,
        case_sensitive: bool,
    ) -> Result<Vec<SearchHit>, CoreError> {
        let hits = self.doc.search_document(query, case_sensitive)?;
        Ok(hits)
    }

    /// Access all annotations currently active in this session.
    pub fn annotations(&self) -> &[Annotation] {
        &self.annotations
    }

    /// Access all annotations on a specific 0-based page.
    pub fn annotations_for_page(&self, page: u32) -> Vec<Annotation> {
        self.annotations
            .iter()
            .filter(|a| a.page_index == page)
            .cloned()
            .collect()
    }

    /// Internal forward-action: add or replace an annotation and invalidate page tiles in TileCache.
    pub fn apply_add_annotation(&mut self, annot: Annotation) {
        let page = annot.page_index;
        self.annotations.retain(|a| a.id != annot.id);
        self.annotations.push(annot);
        self.tile_cache.invalidate_page(self.doc_id, page);
    }

    /// Internal forward/undo-action: remove an annotation by ID and invalidate page tiles in TileCache.
    pub fn apply_remove_annotation(&mut self, id: u64) -> Option<Annotation> {
        if let Some(pos) = self.annotations.iter().position(|a| a.id == id) {
            let removed = self.annotations.remove(pos);
            self.tile_cache
                .invalidate_page(self.doc_id, removed.page_index);
            Some(removed)
        } else {
            None
        }
    }

    /// Internal forward/undo-action: update an existing annotation and invalidate page tiles in TileCache.
    pub fn apply_update_annotation(&mut self, annot: Annotation) -> Option<Annotation> {
        let page = annot.page_index;
        if let Some(pos) = self.annotations.iter().position(|a| a.id == annot.id) {
            let old = std::mem::replace(&mut self.annotations[pos], annot);
            self.tile_cache.invalidate_page(self.doc_id, page);
            Some(old)
        } else {
            self.apply_add_annotation(annot);
            None
        }
    }

    /// Execute a command through the undo/redo stack.
    pub fn execute_command(&mut self, command: Box<dyn Command>) -> Result<(), CoreError> {
        let mut stack = std::mem::take(&mut self.command_stack);
        let res = stack.execute(command, self);
        self.command_stack = stack;
        res
    }

    /// Undo the top command on the history stack.
    pub fn undo(&mut self) -> Result<bool, CoreError> {
        let mut stack = std::mem::take(&mut self.command_stack);
        let res = stack.undo(self);
        self.command_stack = stack;
        res
    }

    /// Redo the top command on the redo stack.
    pub fn redo(&mut self) -> Result<bool, CoreError> {
        let mut stack = std::mem::take(&mut self.command_stack);
        let res = stack.redo(self);
        self.command_stack = stack;
        res
    }

    /// Return whether an undo action is available.
    pub fn can_undo(&self) -> bool {
        self.command_stack.can_undo()
    }

    /// Return whether a redo action is available.
    pub fn can_redo(&self) -> bool {
        self.command_stack.can_redo()
    }

    /// Add an annotation with undo/redo support.
    pub fn add_annotation_undoable(&mut self, annot: Annotation) -> Result<(), CoreError> {
        let cmd = Box::new(AddAnnotationCommand::new(annot));
        self.execute_command(cmd)
    }

    /// Remove an annotation by ID with undo/redo support.
    pub fn remove_annotation_undoable(&mut self, id: u64) -> Result<(), CoreError> {
        let annot = self
            .annotations
            .iter()
            .find(|a| a.id == id)
            .cloned()
            .ok_or_else(|| CoreError::Internal(format!("Annotation with ID {id} not found")))?;
        let cmd = Box::new(RemoveAnnotationCommand::new(annot));
        self.execute_command(cmd)
    }

    /// Update an annotation with undo/redo support.
    pub fn update_annotation_undoable(&mut self, annot: Annotation) -> Result<(), CoreError> {
        let old = self
            .annotations
            .iter()
            .find(|a| a.id == annot.id)
            .cloned()
            .ok_or_else(|| {
                CoreError::Internal(format!("Annotation with ID {} not found", annot.id))
            })?;
        let cmd = Box::new(UpdateAnnotationCommand::new(old, annot));
        self.execute_command(cmd)
    }

    /// Save all current session annotations to a PDF file at `output_path`.
    pub fn save_annotations_to_pdf(
        &self,
        input_path: &Path,
        output_path: &Path,
    ) -> Result<(), CoreError> {
        rbd_engine::save_annotations_to_pdf(input_path, output_path, &self.annotations)?;
        Ok(())
    }

    /// Extract text across all pages and summarize using offline natural language processing.
    pub fn summarize_document(
        &self,
        max_sentences: usize,
    ) -> Result<rbd_engine::DocumentSummary, CoreError> {
        let count = self.page_count()?;
        let mut page_texts: Vec<(u32, String)> = Vec::with_capacity(count as usize);

        for p in 0..count {
            let text = self.extract_text(p).unwrap_or_default();
            page_texts.push((p + 1, text));
        }

        let slice_refs: Vec<(u32, &str)> =
            page_texts.iter().map(|(p, t)| (*p, t.as_str())).collect();
        let summary = rbd_engine::summarize_document(&slice_refs, max_sentences)?;
        Ok(summary)
    }

    /// Query the document using natural language and return the most relevant passage with page citation.
    pub fn ask_document(&self, query: &str) -> Result<rbd_engine::DocumentAnswer, CoreError> {
        let count = self.page_count()?;
        let mut page_texts: Vec<(u32, String)> = Vec::with_capacity(count as usize);

        for p in 0..count {
            let text = self.extract_text(p).unwrap_or_default();
            page_texts.push((p + 1, text));
        }

        let slice_refs: Vec<(u32, &str)> =
            page_texts.iter().map(|(p, t)| (*p, t.as_str())).collect();
        let answer = rbd_engine::ask_document(&slice_refs, query)?;
        Ok(answer)
    }

    /// Extract fluid, structured paragraphs from a page for reflow editing.
    pub fn extract_page_paragraphs(
        &self,
        page_index: u32,
    ) -> Result<Vec<rbd_engine::ReflowParagraph>, CoreError> {
        let text = self.extract_text(page_index)?;
        let (width, height) = self.page_dimensions(page_index)?;

        let mut segments = Vec::new();
        let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
        let line_count = lines.len().max(1) as f32;
        let step_y = (height - 100.0) / line_count;

        for (i, line) in lines.iter().enumerate() {
            let y_top = height - 50.0 - (i as f32) * step_y;
            let y_bottom = y_top - 12.0;
            segments.push(rbd_engine::TextLineSegment {
                text: line.to_string(),
                bbox: [50.0, y_bottom, width - 50.0, y_top],
                font_size: 11.0,
            });
        }

        let paragraphs = rbd_engine::cluster_lines_into_paragraphs(page_index, &segments);
        Ok(paragraphs)
    }

    /// Replace text in a paragraph bounding box with automatic multi-line reflow.
    pub fn replace_paragraph_text(
        &self,
        input_path: &Path,
        page_index: u32,
        bbox: rbd_engine::BoundingBox,
        new_text: &str,
        font_size: f32,
        output_path: &Path,
    ) -> Result<(), CoreError> {
        rbd_engine::replace_paragraph_text(
            input_path,
            page_index,
            bbox,
            new_text,
            font_size,
            output_path,
        )?;
        Ok(())
    }
}
