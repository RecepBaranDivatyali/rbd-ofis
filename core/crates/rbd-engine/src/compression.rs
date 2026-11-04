#![allow(clippy::collapsible_if, clippy::collapsible_match)]

use crate::error::EngineError;
use lopdf::Document;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Options controlling PDF compression and size optimization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompressionOptions {
    /// Prune unreferenced objects and compact the xref table.
    pub remove_unused_objects: bool,
    /// Decompress and re-compress streams using FlateDecode (zlib).
    pub recompress_streams: bool,
    /// Strip document information dictionary (/Info) and XMP metadata stream (/Metadata).
    pub strip_metadata: bool,
}

impl Default for CompressionOptions {
    fn default() -> Self {
        Self {
            remove_unused_objects: true,
            recompress_streams: true,
            strip_metadata: false,
        }
    }
}

impl CompressionOptions {
    /// Create standard compression options with stream recompression and dead object pruning.
    pub fn standard() -> Self {
        Self::default()
    }

    /// Aggressive compression with metadata stripped.
    pub fn aggressive() -> Self {
        Self {
            remove_unused_objects: true,
            recompress_streams: true,
            strip_metadata: true,
        }
    }
}

/// Statistics detailing the outcome of a compression operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompressionResult {
    /// Size of input file in bytes.
    pub original_size: u64,
    /// Size of compressed output file in bytes.
    pub compressed_size: u64,
    /// Percentage savings: `(original - compressed) / original * 100.0`.
    pub savings_percent: f64,
}

/// Compress and optimize a PDF file.
pub fn compress_document(
    input_path: &Path,
    options: &CompressionOptions,
    output_path: &Path,
) -> Result<CompressionResult, EngineError> {
    let original_size = std::fs::metadata(input_path).map(|m| m.len()).unwrap_or(0);

    let mut doc = Document::load(input_path)?;
    compress_doc_internal(&mut doc, options);
    doc.save(output_path)?;

    let compressed_size = std::fs::metadata(output_path).map(|m| m.len()).unwrap_or(0);

    let savings_percent = if original_size > 0 && original_size > compressed_size {
        ((original_size - compressed_size) as f64 / original_size as f64) * 100.0
    } else {
        0.0
    };

    Ok(CompressionResult {
        original_size,
        compressed_size,
        savings_percent,
    })
}

/// In-memory variant of `compress_document`.
pub fn compress_document_bytes(
    input_bytes: &[u8],
    options: &CompressionOptions,
) -> Result<(Vec<u8>, CompressionResult), EngineError> {
    let original_size = input_bytes.len() as u64;

    let mut doc = Document::load_mem(input_bytes)?;
    compress_doc_internal(&mut doc, options);

    let mut compressed_bytes = Vec::new();
    doc.save_to(&mut compressed_bytes)?;
    let compressed_size = compressed_bytes.len() as u64;

    let savings_percent = if original_size > 0 && original_size > compressed_size {
        ((original_size - compressed_size) as f64 / original_size as f64) * 100.0
    } else {
        0.0
    };

    Ok((
        compressed_bytes,
        CompressionResult {
            original_size,
            compressed_size,
            savings_percent,
        },
    ))
}

fn compress_doc_internal(doc: &mut Document, options: &CompressionOptions) {
    // 1. Strip metadata if requested
    if options.strip_metadata {
        // Remove /Info from trailer
        let _ = doc.trailer.remove(b"Info");

        // Remove /Metadata and /PieceInfo from document catalog
        if let Ok(catalog_ref) = doc.trailer.get(b"Root").and_then(|o| o.as_reference()) {
            if let Ok(catalog_dict) = doc.get_dictionary_mut(catalog_ref) {
                let _ = catalog_dict.remove(b"Metadata");
                let _ = catalog_dict.remove(b"PieceInfo");
            }
        }
    }

    // 2. Stream re-compression
    if options.recompress_streams {
        doc.decompress();
        doc.compress();
        doc.delete_zero_length_streams();
    }

    // 3. Object pruning and compaction
    if options.remove_unused_objects {
        doc.prune_objects();
        doc.renumber_objects();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::content::{Content, Operation};
    use lopdf::{Dictionary, Document, Object, Stream};

    fn create_uncompressed_sample_pdf() -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();

        // Repetitive text to achieve great compression ratios
        let mut content = Content {
            operations: Vec::new(),
        };
        for i in 0..50 {
            content.operations.push(Operation::new("BT", vec![]));
            content.operations.push(Operation::new(
                "Tm",
                vec![
                    Object::Real(1.0),
                    Object::Real(0.0),
                    Object::Real(0.0),
                    Object::Real(1.0),
                    Object::Real(50.0),
                    Object::Real(100.0 + i as f32 * 10.0),
                ],
            ));
            content.operations.push(Operation::new(
                "Tj",
                vec![Object::string_literal(format!(
                    "Line number {i}: This is repeating text to test FlateDecode compression!"
                ))],
            ));
            content.operations.push(Operation::new("ET", vec![]));
        }

        let content_bytes = content.encode().unwrap();
        // Uncompressed stream
        let stream = Stream::new(Dictionary::new(), content_bytes);
        let content_id = doc.add_object(Object::Stream(stream));

        let mut page_dict = Dictionary::new();
        page_dict.set("Type", Object::Name(b"Page".to_vec()));
        page_dict.set("Parent", Object::Reference(pages_id));
        page_dict.set(
            "MediaBox",
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(600.0),
                Object::Real(800.0),
            ]),
        );
        page_dict.set("Contents", Object::Reference(content_id));
        let page_id = doc.add_object(Object::Dictionary(page_dict));

        let mut pages_dict = Dictionary::new();
        pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
        pages_dict.set("Kids", Object::Array(vec![Object::Reference(page_id)]));
        pages_dict.set("Count", Object::Integer(1));
        doc.objects.insert(pages_id, Object::Dictionary(pages_dict));

        let mut catalog = Dictionary::new();
        catalog.set("Type", Object::Name(b"Catalog".to_vec()));
        catalog.set("Pages", Object::Reference(pages_id));
        let cat_id = doc.add_object(Object::Dictionary(catalog));
        doc.trailer.set("Root", Object::Reference(cat_id));

        let mut out = Vec::new();
        doc.save_to(&mut out).unwrap();
        out
    }

    #[test]
    fn test_compression_reduces_size() {
        let uncompressed = create_uncompressed_sample_pdf();
        let options = CompressionOptions::standard();

        let (compressed, result) = compress_document_bytes(&uncompressed, &options).unwrap();

        assert!(
            result.compressed_size < result.original_size,
            "Compressed size ({}) must be smaller than uncompressed original ({})",
            result.compressed_size,
            result.original_size
        );
        assert!(
            result.savings_percent > 10.0,
            "Compression savings percent should be substantial: {}%",
            result.savings_percent
        );

        // Verify resulting document loads and has valid structure
        let roundtrip_doc = Document::load_mem(&compressed).unwrap();
        assert_eq!(roundtrip_doc.get_pages().len(), 1);
    }

    #[test]
    fn test_compression_strip_metadata() {
        let base = create_uncompressed_sample_pdf();
        let mut doc = Document::load_mem(&base).unwrap();

        // Add /Info in trailer
        let mut info_dict = Dictionary::new();
        info_dict.set("Title", Object::string_literal("Confidential Document"));
        info_dict.set("Author", Object::string_literal("Author Name"));
        let info_id = doc.add_object(Object::Dictionary(info_dict));
        doc.trailer.set("Info", Object::Reference(info_id));

        let mut with_info = Vec::new();
        doc.save_to(&mut with_info).unwrap();
        assert!(String::from_utf8_lossy(&with_info).contains("Confidential Document"));

        let options = CompressionOptions::aggressive();
        let (compressed, _) = compress_document_bytes(&with_info, &options).unwrap();
        let roundtrip_doc = Document::load_mem(&compressed).unwrap();

        assert!(
            roundtrip_doc.trailer.get(b"Info").is_err(),
            "Trailer /Info must be completely removed"
        );
        assert!(
            !String::from_utf8_lossy(&compressed).contains("Confidential Document"),
            "Title string must be stripped from document bytes"
        );
    }

    #[test]
    fn test_compression_prune_unused_objects() {
        let base = create_uncompressed_sample_pdf();
        let mut doc = Document::load_mem(&base).unwrap();

        // Inject 5 orphan/unreferenced objects
        for i in 0..5 {
            let mut orphan = Dictionary::new();
            orphan.set("OrphanKey", Object::string_literal(format!("Orphan {i}")));
            doc.add_object(Object::Dictionary(orphan));
        }

        let obj_count_before = doc.objects.len();

        let mut with_orphans = Vec::new();
        doc.save_to(&mut with_orphans).unwrap();

        let options = CompressionOptions {
            remove_unused_objects: true,
            recompress_streams: false,
            strip_metadata: false,
        };

        let (compressed, _) = compress_document_bytes(&with_orphans, &options).unwrap();
        let roundtrip_doc = Document::load_mem(&compressed).unwrap();

        assert!(
            roundtrip_doc.objects.len() < obj_count_before,
            "Unused objects must be pruned (before: {}, after: {})",
            obj_count_before,
            roundtrip_doc.objects.len()
        );
    }
}
