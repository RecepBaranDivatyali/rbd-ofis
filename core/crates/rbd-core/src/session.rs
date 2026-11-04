use crate::error::CoreError;
use crate::tile_cache::{TileCache, TileKey};
use rbd_engine::{DocumentMetadata, PdfDocument, PdfEngine, PixelFormat};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

static SESSION_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// A stateful document session representing an opened PDF file.
pub struct DocumentSession {
    doc_id: u64,
    doc: Box<dyn PdfDocument>,
    tile_cache: Arc<TileCache>,
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

        Ok(Self {
            doc_id,
            doc,
            tile_cache: cache,
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

        Ok(Self {
            doc_id,
            doc,
            tile_cache: cache,
        })
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
}
