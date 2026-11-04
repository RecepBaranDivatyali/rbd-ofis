pub use crate::error::EngineError;
use std::path::Path;

/// Pixel format for rendered tile buffers.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PixelFormat {
    /// 32-bit BGRA (Blue, Green, Red, Alpha), standard for Direct2D, GDI, and WPF backbuffers.
    #[default]
    Bgra32 = 0,
    /// 32-bit RGBA (Red, Green, Blue, Alpha).
    Rgba32 = 1,
}

impl TryFrom<i32> for PixelFormat {
    type Error = EngineError;

    fn try_from(val: i32) -> Result<Self, Self::Error> {
        match val {
            0 => Ok(PixelFormat::Bgra32),
            1 => Ok(PixelFormat::Rgba32),
            _ => Err(EngineError::InvalidParameter(format!(
                "Invalid pixel format: {val} (expected 0 for Bgra32 or 1 for Rgba32)"
            ))),
        }
    }
}

/// Standard document metadata fields.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub creation_date: Option<String>,
    pub mod_date: Option<String>,
}

/// Abstract PDF engine interface.
pub trait PdfEngine: Send + Sync {
    /// Human-readable name of this engine backend.
    fn name(&self) -> &'static str;

    /// Open a PDF document from a file path on disk.
    fn open_from_file(
        &self,
        path: &Path,
        password: Option<&str>,
    ) -> Result<Box<dyn PdfDocument>, EngineError>;

    /// Open a PDF document from an in-memory byte slice.
    fn open_from_bytes(
        &self,
        bytes: &[u8],
        password: Option<&str>,
    ) -> Result<Box<dyn PdfDocument>, EngineError>;
}

/// Abstract PDF document interface.
pub trait PdfDocument: Send + Sync {
    /// Total number of pages in the document.
    fn page_count(&self) -> Result<u32, EngineError>;

    /// Access a single page by 0-based page index.
    fn page(&self, index: u32) -> Result<Box<dyn PdfPage>, EngineError>;

    /// Extract document-level metadata (title, author, etc.).
    fn metadata(&self) -> Result<DocumentMetadata, EngineError>;
}

/// Abstract single PDF page interface.
pub trait PdfPage: Send + Sync {
    /// Dimensions of the page in points (1 point = 1/72 inch).
    fn dimensions_points(&self) -> Result<(f32, f32), EngineError>;

    /// Render the entire page to RGBA8 pixels into a caller-provided mutable slice.
    ///
    /// The slice must have length of at least `width_px * height_px * 4`.
    /// Zero-copy friendly: the pixels are written directly into `out_buf`.
    fn render_to_rgba(
        &self,
        width_px: u32,
        height_px: u32,
        out_buf: &mut [u8],
    ) -> Result<(), EngineError>;

    /// Render a sub-rectangle tile `(tile_x, tile_y, tile_width, tile_height)` at `scale_dpi`
    /// into a caller-provided mutable slice.
    ///
    /// The slice must have length of at least `tile_width * tile_height * 4`.
    fn render_tile_to_rgba(
        &self,
        tile_x: u32,
        tile_y: u32,
        tile_width: u32,
        tile_height: u32,
        scale_dpi: f32,
        out_buf: &mut [u8],
    ) -> Result<(), EngineError> {
        self.render_tile_to_buffer(
            tile_x,
            tile_y,
            tile_width,
            tile_height,
            scale_dpi,
            PixelFormat::Rgba32,
            (tile_width as usize) * 4,
            out_buf,
        )
    }

    /// Render a sub-rectangle tile `(tile_x, tile_y, tile_width, tile_height)` at `scale_dpi`
    /// into a caller-provided mutable slice respecting the specified `pixel_format` and `stride`.
    ///
    /// `stride` is the length of each row in bytes (must be >= `tile_width * 4`).
    /// The buffer must have length of at least `stride * tile_height`.
    #[allow(clippy::too_many_arguments)]
    fn render_tile_to_buffer(
        &self,
        tile_x: u32,
        tile_y: u32,
        tile_width: u32,
        tile_height: u32,
        scale_dpi: f32,
        format: PixelFormat,
        stride: usize,
        out_buf: &mut [u8],
    ) -> Result<(), EngineError>;

    /// Extract page text as a UTF-8 string, preserving Turkish characters (`İıŞşĞğÇçÖöÜü`).
    fn extract_text(&self) -> Result<String, EngineError>;
}
