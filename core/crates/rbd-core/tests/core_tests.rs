use rbd_core::{
    Command, CommandStack, CoreError, DocumentSession, TileCache, delete_pages, merge, rotate,
    split,
};
use rbd_engine::{EngineType, create_engine};
use std::path::PathBuf;
use std::sync::Arc;

fn get_corpus_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../corpus/files")
}

#[test]
fn test_session_tile_cache_hit() {
    let engine = create_engine(EngineType::Pdfium).expect("PDFium engine");
    let pdf_path = get_corpus_dir().join("pdf-basic/pdfium-hello-world.pdf");

    let cache = Arc::new(TileCache::new(64));
    let session = DocumentSession::open_file(&pdf_path, None, engine.as_ref(), Some(cache.clone()))
        .expect("Open session");

    assert_eq!(session.page_count().unwrap(), 1);
    assert_eq!(cache.len(), 0);

    let mut buf1 = vec![0u8; 256 * 256 * 4];
    session
        .render_tile_into_rgba(0, 0, 0, 256, 256, 150.0, &mut buf1)
        .expect("First render");

    // Cache should now have 1 item
    assert_eq!(cache.len(), 1);

    // Second render into a separate buffer
    let mut buf2 = vec![0u8; 256 * 256 * 4];
    session
        .render_tile_into_rgba(0, 0, 0, 256, 256, 150.0, &mut buf2)
        .expect("Second render (from cache)");

    // Cache len remains 1
    assert_eq!(cache.len(), 1);
    // Buffers match exactly
    assert_eq!(buf1, buf2);
}

#[test]
fn test_page_operations_rotate_split_delete_merge() {
    let temp_dir = std::env::temp_dir().join("rbd_core_test_ops");
    let _ = std::fs::create_dir_all(&temp_dir);

    let src_pdf = get_corpus_dir().join("pdf-basic/pdfium-hello-world-2-pages.pdf");
    assert!(src_pdf.exists(), "src pdf exists");

    // 1. Rotate
    let rotated_pdf = temp_dir.join("rotated.pdf");
    rotate(&src_pdf, 90, &rotated_pdf).expect("Rotate 90 degrees");
    assert!(rotated_pdf.exists());

    // 2. Split
    let split_pdf = temp_dir.join("split.pdf");
    split(&src_pdf, 1, 1, &split_pdf).expect("Split page 1");
    assert!(split_pdf.exists());

    // Verify split document has 1 page
    let engine = create_engine(EngineType::Pdfium).unwrap();
    let doc = engine.open_from_file(&split_pdf, None).unwrap();
    assert_eq!(doc.page_count().unwrap(), 1);

    // 3. Delete pages
    let deleted_pdf = temp_dir.join("deleted.pdf");
    delete_pages(&src_pdf, &[2], &deleted_pdf).expect("Delete page 2");
    assert!(deleted_pdf.exists());
    let doc_del = engine.open_from_file(&deleted_pdf, None).unwrap();
    assert_eq!(doc_del.page_count().unwrap(), 1);

    // 4. Merge
    let merged_pdf = temp_dir.join("merged.pdf");
    merge(&[&split_pdf, &deleted_pdf], &merged_pdf).expect("Merge two 1-page pdfs");
    assert!(merged_pdf.exists());
    let doc_merged = engine.open_from_file(&merged_pdf, None).unwrap();
    assert_eq!(doc_merged.page_count().unwrap(), 2);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

struct DummyCommand {
    executed: bool,
}

impl Command for DummyCommand {
    fn name(&self) -> &str {
        "Dummy"
    }

    fn execute(&mut self, _session: &mut DocumentSession) -> Result<(), CoreError> {
        self.executed = true;
        Ok(())
    }

    fn undo(&mut self, _session: &mut DocumentSession) -> Result<(), CoreError> {
        self.executed = false;
        Ok(())
    }
}

#[test]
fn test_command_stack_undo_redo() {
    let engine = create_engine(EngineType::Pdfium).expect("PDFium engine");
    let pdf_path = get_corpus_dir().join("pdf-basic/pdfium-hello-world.pdf");
    let mut session = DocumentSession::open_file(&pdf_path, None, engine.as_ref(), None).unwrap();

    let mut stack = CommandStack::new(10);
    assert!(!stack.can_undo());
    assert!(!stack.can_redo());

    let cmd = Box::new(DummyCommand { executed: false });
    stack.execute(cmd, &mut session).unwrap();
    assert!(stack.can_undo());
    assert!(!stack.can_redo());

    let undone = stack.undo(&mut session).unwrap();
    assert!(undone);
    assert!(!stack.can_undo());
    assert!(stack.can_redo());

    let redone = stack.redo(&mut session).unwrap();
    assert!(redone);
    assert!(stack.can_undo());
    assert!(!stack.can_redo());
}

#[test]
fn test_annotation_session_undo_redo_and_cache_invalidation() {
    use rbd_engine::{Annotation, AnnotationKind, PdfRect};

    let engine = create_engine(EngineType::Pdfium).expect("PDFium engine");
    let pdf_path = get_corpus_dir().join("pdf-basic/pdfium-hello-world.pdf");

    let cache = Arc::new(TileCache::new(64));
    let mut session =
        DocumentSession::open_file(&pdf_path, None, engine.as_ref(), Some(cache.clone())).unwrap();

    let mut buf = vec![0u8; 256 * 256 * 4];
    session
        .render_tile_into_rgba(0, 0, 0, 256, 256, 150.0, &mut buf)
        .unwrap();
    assert_eq!(cache.len(), 1, "Cache should have 1 item");

    // Add annotation via undoable command
    let annot = Annotation::new(
        101,
        0,
        AnnotationKind::Highlight,
        PdfRect::new(20.0, 30.0, 100.0, 50.0),
        [255, 235, 59, 255],
        "Test Highlight".into(),
    );

    session.add_annotation_undoable(annot).unwrap();
    assert_eq!(session.annotations_for_page(0).len(), 1);
    assert_eq!(
        cache.len(),
        0,
        "Adding annotation must invalidate page tile cache!"
    );

    // Render again to re-populate cache
    session
        .render_tile_into_rgba(0, 0, 0, 256, 256, 150.0, &mut buf)
        .unwrap();
    assert_eq!(cache.len(), 1);

    // Undo add annotation
    assert!(session.can_undo());
    assert!(session.undo().unwrap());
    assert_eq!(
        session.annotations_for_page(0).len(),
        0,
        "Undo should remove annotation"
    );
    assert_eq!(cache.len(), 0, "Undo must invalidate page tile cache!");

    // Redo add annotation
    assert!(session.can_redo());
    assert!(session.redo().unwrap());
    assert_eq!(
        session.annotations_for_page(0).len(),
        1,
        "Redo should restore annotation"
    );
    assert_eq!(cache.len(), 0, "Redo must invalidate page tile cache!");
}

#[test]
fn test_session_turkish_search() {
    let engine = create_engine(EngineType::Pdfium).expect("PDFium engine");
    let pdf_path = get_corpus_dir().join("pdf-cjk-rtl-turkish/tr-generated-standard-fonts.pdf");
    let session = DocumentSession::open_file(&pdf_path, None, engine.as_ref(), None).unwrap();

    let hits = session.search_document("ŞAŞIRTICI", false).unwrap();
    assert!(!hits.is_empty(), "Should find matches");
    assert_eq!(hits[0].page_index, 0);
    assert!(!hits[0].rects.is_empty());
}

#[test]
fn test_core_page_ops_crop_reorder_insert_blank() {
    use rbd_core::{crop_page, insert_blank_page, reorder_pages};

    let src_pdf = get_corpus_dir().join("pdf-basic/pdfium-hello-world-2-pages.pdf");
    let temp_dir = std::env::temp_dir().join("rbd_core_page_ops_test");
    let _ = std::fs::create_dir_all(&temp_dir);

    let blank_pdf = temp_dir.join("blank.pdf");
    insert_blank_page(&src_pdf, &blank_pdf, 2, 500.0, 500.0).unwrap();

    let crop_pdf = temp_dir.join("crop.pdf");
    crop_page(&blank_pdf, &crop_pdf, 1, [0.0, 0.0, 200.0, 200.0]).unwrap();

    let reorder_pdf = temp_dir.join("reorder.pdf");
    reorder_pages(&crop_pdf, &reorder_pdf, &[3, 2, 1]).unwrap();

    let engine = create_engine(EngineType::Pdfium).unwrap();
    let doc = engine.open_from_file(&reorder_pdf, None).unwrap();
    assert_eq!(doc.page_count().unwrap(), 3);

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_regression_item5_tile_cache_and_session_max_dimensions() {
    let engine = create_engine(EngineType::Pdfium).expect("PDFium engine");
    let pdf_path = get_corpus_dir().join("pdf-basic/pdfium-hello-world.pdf");
    let session = DocumentSession::open_file(&pdf_path, None, engine.as_ref(), None).unwrap();

    let mut buf = vec![0u8; 100];

    // 1. Tile dimensions exceeding MAX_TILE_DIMENSION (2048) must be rejected with InvalidParameter
    let res_oversize = session.render_tile_into_rgba(0, 0, 0, 4000, 4000, 150.0, &mut buf);
    assert!(res_oversize.is_err());
    match res_oversize.err().unwrap() {
        rbd_core::CoreError::Engine(rbd_engine::EngineError::InvalidParameter(_)) => {}
        other => panic!("Expected EngineError::InvalidParameter, got: {:?}", other),
    }

    // 2. Zero-dimension tiles must be rejected
    let res_zero = session.render_tile_into_rgba(0, 0, 0, 0, 256, 150.0, &mut buf);
    assert!(res_zero.is_err());

    // 3. TileCache rejecting payloads larger than MAX_TILE_PAYLOAD_BYTES
    let cache = rbd_core::TileCache::default();
    let key = rbd_core::TileKey {
        doc_id: 1,
        page: 0,
        zoom_bucket: 100,
        tile_x: 0,
        tile_y: 0,
        pixel_format: 0,
    };
    // 17 MB payload (> 16 MB max)
    let huge_data = std::sync::Arc::new(vec![0u8; rbd_core::MAX_TILE_PAYLOAD_BYTES + 1024]);
    cache.insert(key, huge_data);
    assert!(
        cache.get(&key).is_none(),
        "Oversized tile payload must not be cached"
    );
}
