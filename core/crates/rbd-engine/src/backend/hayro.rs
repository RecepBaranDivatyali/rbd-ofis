use crate::error::EngineError;
use crate::traits::{DocumentMetadata, PdfDocument, PdfEngine, PdfPage};
use hayro::hayro_interpret::InterpreterSettings;
use hayro::hayro_interpret::util::TransformExt;
use hayro::hayro_syntax::Pdf;
use hayro::kurbo::Affine;
use hayro::vello_cpu::color::palette::css::WHITE;
use hayro::vello_cpu::{Pixmap, RasterizerSettings, RenderContext, Resources, TargetInit};
use hayro::{PixmapSettings, RenderCache, RenderSettings, render, render_into};
use std::path::Path;
use std::sync::Arc;

/// Pure-Rust reference PDF engine using `hayro` and `lopdf`.
#[derive(Default)]
pub struct HayroEngine;

impl HayroEngine {
    pub fn new() -> Self {
        Self
    }
}

impl PdfEngine for HayroEngine {
    fn name(&self) -> &'static str {
        "Hayro (Pure-Rust)"
    }

    fn open_from_file(
        &self,
        path: &Path,
        password: Option<&str>,
    ) -> Result<Box<dyn PdfDocument>, EngineError> {
        let bytes = std::fs::read(path)?;
        self.open_from_bytes(&bytes, password)
    }

    fn open_from_bytes(
        &self,
        bytes: &[u8],
        _password: Option<&str>,
    ) -> Result<Box<dyn PdfDocument>, EngineError> {
        let pdf = Pdf::new(bytes.to_vec()).map_err(|e| EngineError::Corrupt(format!("{e:?}")))?;

        let raw_bytes = Arc::new(bytes.to_vec());
        let lopdf_doc = lopdf::Document::load_mem(bytes).ok();

        Ok(Box::new(HayroDocumentWrapper {
            pdf: Arc::new(pdf),
            lopdf_doc,
            raw_bytes,
        }))
    }
}

struct HayroDocumentWrapper {
    pdf: Arc<Pdf>,
    lopdf_doc: Option<lopdf::Document>,
    raw_bytes: Arc<Vec<u8>>,
}

impl PdfDocument for HayroDocumentWrapper {
    fn page_count(&self) -> Result<u32, EngineError> {
        Ok(self.pdf.pages().len() as u32)
    }

    fn page(&self, index: u32) -> Result<Box<dyn PdfPage>, EngineError> {
        let total = self.page_count()?;
        if index >= total {
            return Err(EngineError::PageOutOfRange {
                index,
                total_pages: total,
            });
        }

        Ok(Box::new(HayroPageWrapper {
            pdf: self.pdf.clone(),
            lopdf_doc: self.lopdf_doc.clone(),
            raw_bytes: self.raw_bytes.clone(),
            page_index: index,
        }))
    }

    fn metadata(&self) -> Result<DocumentMetadata, EngineError> {
        let mut meta = DocumentMetadata::default();
        if let Some(ref doc) = self.lopdf_doc
            && let Ok(info) = doc.trailer.get(b"Info")
            && let Ok(dict) = doc.dereference(info).and_then(|(_, o)| o.as_dict())
        {
            let get_str = |key: &[u8]| -> Option<String> {
                dict.get(key)
                    .and_then(|o| o.as_str())
                    .ok()
                    .map(|b| String::from_utf8_lossy(b).into_owned())
            };
            meta.title = get_str(b"Title");
            meta.author = get_str(b"Author");
            meta.subject = get_str(b"Subject");
            meta.keywords = get_str(b"Keywords");
            meta.creator = get_str(b"Creator");
            meta.producer = get_str(b"Producer");
            meta.creation_date = get_str(b"CreationDate");
            meta.mod_date = get_str(b"ModDate");
        }
        Ok(meta)
    }
}

struct HayroPageWrapper {
    pdf: Arc<Pdf>,
    lopdf_doc: Option<lopdf::Document>,
    raw_bytes: Arc<Vec<u8>>,
    page_index: u32,
}

impl PdfPage for HayroPageWrapper {
    fn dimensions_points(&self) -> Result<(f32, f32), EngineError> {
        let page = &self.pdf.pages()[self.page_index as usize];
        let (w, h) = page.render_dimensions();
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
            return Err(EngineError::RenderError("Buffer too small".into()));
        }

        let page = &self.pdf.pages()[self.page_index as usize];
        let (dim_w, dim_h) = page.render_dimensions();

        let scale_x = width_px as f32 / dim_w.max(1.0);
        let scale_y = height_px as f32 / dim_h.max(1.0);

        let cache = RenderCache::new();
        let interpreter_settings = InterpreterSettings::default();
        let render_settings = RenderSettings::default();
        let pixmap_settings = PixmapSettings {
            x_scale: scale_x,
            y_scale: scale_y,
            bg_color: WHITE,
        };

        let rendered = render(
            page,
            &cache,
            &interpreter_settings,
            &render_settings,
            &pixmap_settings,
        );

        let data = rendered.data();
        let bytes: &[u8] = bytemuck::cast_slice(data);
        let copy_len = bytes.len().min(required_len);
        out_buf[..copy_len].copy_from_slice(&bytes[..copy_len]);

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

        let w_u16 = u16::try_from(tile_width).map_err(|_| {
            EngineError::InvalidParameter(format!("Tile width exceeds u16: {tile_width}"))
        })?;
        let h_u16 = u16::try_from(tile_height).map_err(|_| {
            EngineError::InvalidParameter(format!("Tile height exceeds u16: {tile_height}"))
        })?;

        let page = &self.pdf.pages()[self.page_index as usize];
        let cache = RenderCache::new();
        let interpreter_settings = InterpreterSettings::default();
        let render_settings = RenderSettings::default();

        let scale = (scale_dpi / 72.0) as f64;
        let mut ctx = RenderContext::new(w_u16, h_u16);
        let mut pixmap = Pixmap::new(w_u16, h_u16);
        let mut resources = Resources::default();

        let transform = Affine::translate((-(tile_x as f64), -(tile_y as f64)))
            * Affine::scale(scale)
            * page.initial_transform(true).to_kurbo();

        render_into(
            page,
            &cache,
            &interpreter_settings,
            &render_settings,
            &mut ctx,
            transform,
        );

        ctx.flush();

        ctx.render_with(
            &mut pixmap,
            &mut resources,
            RasterizerSettings {
                target_init: TargetInit::Clear(WHITE),
                ..Default::default()
            },
        );

        let data = pixmap.data();
        let src_bytes: &[u8] = bytemuck::cast_slice(data);

        for y in 0..(tile_height as usize) {
            let src_row = &src_bytes[y * row_bytes..(y + 1) * row_bytes];
            let dst_row = &mut out_buf[y * stride..y * stride + row_bytes];
            if format == crate::traits::PixelFormat::Bgra32 {
                for (src, dst) in src_row
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .zip(dst_row.as_chunks_mut::<4>().0)
                {
                    dst[0] = src[2]; // B
                    dst[1] = src[1]; // G
                    dst[2] = src[0]; // R
                    dst[3] = src[3]; // A
                }
            } else {
                dst_row.copy_from_slice(src_row);
            }
        }

        Ok(())
    }

    fn extract_text(&self) -> Result<String, EngineError> {
        if let Some(ref doc) = self.lopdf_doc {
            let page_num = self.page_index + 1;
            if let Ok(text) = doc.extract_text(&[page_num])
                && !text.trim().is_empty()
            {
                return Ok(text);
            }

            // Fallback: extract string literals from page content stream directly
            let pages = doc.get_pages();
            if let Some(&page_id) = pages.get(&page_num) {
                let content = doc.get_page_content(page_id);
                let fallback_text = extract_raw_text(&content);
                if !fallback_text.trim().is_empty() {
                    return Ok(fallback_text);
                }
            }
        }

        // Final fallback: scan raw bytes
        let raw_text = extract_raw_text(&self.raw_bytes);
        if !raw_text.trim().is_empty() {
            return Ok(raw_text);
        }

        Ok(String::new())
    }
}

fn extract_raw_text(content: &[u8]) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < content.len() {
        if content[i] == b'(' {
            i += 1;
            let mut s = Vec::new();
            let mut depth = 1;
            while i < content.len() && depth > 0 {
                match content[i] {
                    b'\\' => {
                        if i + 1 < content.len() {
                            i += 1;
                            s.push(content[i]);
                        }
                    }
                    b'(' => {
                        depth += 1;
                        s.push(b'(');
                    }
                    b')' => {
                        depth -= 1;
                        if depth > 0 {
                            s.push(b')');
                        }
                    }
                    b => s.push(b),
                }
                i += 1;
            }
            if let Ok(str_slice) = std::str::from_utf8(&s) {
                out.push_str(str_slice);
                out.push(' ');
            } else {
                out.push_str(&String::from_utf8_lossy(&s));
                out.push(' ');
            }
        } else {
            i += 1;
        }
    }
    out
}
