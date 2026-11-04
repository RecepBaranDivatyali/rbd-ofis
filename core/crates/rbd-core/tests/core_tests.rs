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
