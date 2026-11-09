use rbd_core::{CoreError, DocumentSession};
use rbd_engine::{EngineError, PdfEngine, PixelFormat, create_default_engine};
use std::cell::RefCell;
use std::ffi::{CStr, CString, c_void};
use std::os::raw::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::slice;
use std::sync::{Arc, OnceLock};

pub mod jni;

// Exact status codes matching apps/windows/RbdOfis.Interop/Native/NativeTypes.cs
pub const RBD_OK: i32 = 0;
pub const RBD_ERR_NULL_PTR: i32 = -1;
pub const RBD_ERR_INVALID_PARAM: i32 = -2;
pub const RBD_ERR_FILE_IO: i32 = -3;
pub const RBD_ERR_CORRUPT_PDF: i32 = -4;
pub const RBD_ERR_PASSWORD_REQUIRED: i32 = -5;
pub const RBD_ERR_PAGE_OUT_OF_RANGE: i32 = -6;
pub const RBD_ERR_BUFFER_TOO_SMALL: i32 = -7;
pub const RBD_ERR_ENGINE_FAILURE: i32 = -8;
pub const RBD_ERR_PANIC: i32 = -99;

static GLOBAL_ENGINE: OnceLock<Box<dyn PdfEngine>> = OnceLock::new();

fn get_engine() -> &'static dyn PdfEngine {
    GLOBAL_ENGINE.get_or_init(create_default_engine).as_ref()
}

thread_local! {
    static LAST_ERROR: RefCell<CString> = RefCell::new(CString::new("No error").unwrap());
}

fn set_last_error(msg: &str) {
    LAST_ERROR.with(|cell| {
        let clean = msg.replace('\0', " ");
        if let Ok(cs) = CString::new(clean) {
            *cell.borrow_mut() = cs;
        }
    });
}

fn map_engine_error(err: &EngineError) -> i32 {
    set_last_error(&err.to_string());
    match err {
        EngineError::Io(_) => RBD_ERR_FILE_IO,
        EngineError::Encrypted(_) => RBD_ERR_PASSWORD_REQUIRED,
        EngineError::Corrupt(_) => RBD_ERR_CORRUPT_PDF,
        EngineError::PageOutOfRange { .. } => RBD_ERR_PAGE_OUT_OF_RANGE,
        EngineError::InvalidParameter(_) => RBD_ERR_INVALID_PARAM,
        EngineError::BufferTooSmall { .. } => RBD_ERR_BUFFER_TOO_SMALL,
        EngineError::RenderError(_) => RBD_ERR_ENGINE_FAILURE,
        EngineError::Unsupported(_) | EngineError::Internal(_) => RBD_ERR_ENGINE_FAILURE,
    }
}

fn map_core_error(err: &CoreError) -> i32 {
    set_last_error(&err.to_string());
    match err {
        CoreError::Engine(e) => map_engine_error(e),
        CoreError::Io(_) => RBD_ERR_FILE_IO,
        CoreError::InvalidPage(_) => RBD_ERR_PAGE_OUT_OF_RANGE,
        CoreError::InvalidPageRange { .. } | CoreError::InvalidRotation(_) => RBD_ERR_INVALID_PARAM,
        CoreError::BufferTooSmall { .. } => RBD_ERR_BUFFER_TOO_SMALL,
        CoreError::CommandError(_) | CoreError::Internal(_) => RBD_ERR_ENGINE_FAILURE,
    }
}

/// Helper executing code inside catch_unwind and converting Result to status code.
fn ffi_boundary<F>(f: F) -> i32
where
    F: FnOnce() -> Result<(), i32> + std::panic::UnwindSafe,
{
    match catch_unwind(f) {
        Ok(Ok(())) => RBD_OK,
        Ok(Err(err_code)) => err_code,
        Err(_) => {
            eprintln!("[RBD-FFI] Caught panic at FFI boundary! Preventing process crash.");
            set_last_error("Internal panic at native FFI boundary");
            RBD_ERR_PANIC
        }
    }
}

// ---------------------------------------------------------------------------
// Unified Native Engine and Document Handles (used by C# NativeMethods.cs)
// ---------------------------------------------------------------------------

pub struct RbdEngineHandle {
    pub engine: Arc<dyn PdfEngine>,
}

pub struct RbdDocumentHandle {
    pub session: parking_lot::RwLock<DocumentSession>,
}

/// Initialize the native RBD engine instance.
///
/// # Safety
/// `out_engine` must be a valid, writable pointer to an `IntPtr` / pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_engine_init(out_engine: *mut *mut c_void) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if out_engine.is_null() {
            set_last_error("out_engine pointer is null");
            return Err(RBD_ERR_NULL_PTR);
        }

        let engine = create_default_engine();
        let handle = Box::new(RbdEngineHandle {
            engine: Arc::from(engine),
        });

        unsafe {
            *out_engine = Box::into_raw(handle) as *mut c_void;
        }

        Ok(())
    }))
}

/// Release and free a native engine instance.
///
/// # Safety
/// `engine` must be a valid pointer returned by `rbd_engine_init` or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_engine_free(engine: *mut c_void) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !engine.is_null() {
            let _ = unsafe { Box::from_raw(engine as *mut RbdEngineHandle) };
        }
    }));
}

/// Open a document from a UTF-8 file path.
///
/// # Safety
/// `engine` and `file_path_utf8` must be valid non-null pointers. `out_doc` must be a writable pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_document_open_file(
    engine: *mut c_void,
    file_path_utf8: *const c_char,
    out_doc: *mut *mut c_void,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if engine.is_null() || file_path_utf8.is_null() || out_doc.is_null() {
            set_last_error("Null pointer passed to rbd_document_open_file");
            return Err(RBD_ERR_NULL_PTR);
        }

        let path_str = unsafe { CStr::from_ptr(file_path_utf8) }
            .to_str()
            .map_err(|_| {
                set_last_error("Invalid UTF-8 in file path");
                RBD_ERR_INVALID_PARAM
            })?;

        let engine_handle = unsafe { &*(engine as *const RbdEngineHandle) };
        let session = DocumentSession::open_file(
            Path::new(path_str),
            None,
            engine_handle.engine.as_ref(),
            None,
        )
        .map_err(|e| map_core_error(&e))?;

        let doc_handle = Box::new(RbdDocumentHandle {
            session: parking_lot::RwLock::new(session),
        });
        unsafe {
            *out_doc = Box::into_raw(doc_handle) as *mut c_void;
        }

        Ok(())
    }))
}

/// Open a document directly from an in-memory byte buffer.
///
/// # Safety
/// `engine` and `data` must be non-null valid pointers. `length` must be > 0. `out_doc` must be writable.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_document_open_memory(
    engine: *mut c_void,
    data: *const u8,
    length: usize,
    out_doc: *mut *mut c_void,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if engine.is_null() || data.is_null() || out_doc.is_null() {
            set_last_error("Null pointer passed to rbd_document_open_memory");
            return Err(RBD_ERR_NULL_PTR);
        }
        if length == 0 {
            set_last_error("Zero-length data buffer passed to rbd_document_open_memory");
            return Err(RBD_ERR_INVALID_PARAM);
        }

        let slice = unsafe { slice::from_raw_parts(data, length) };
        let engine_handle = unsafe { &*(engine as *const RbdEngineHandle) };
        let session = DocumentSession::open_bytes(slice, None, engine_handle.engine.as_ref(), None)
            .map_err(|e| map_core_error(&e))?;

        let doc_handle = Box::new(RbdDocumentHandle {
            session: parking_lot::RwLock::new(session),
        });
        unsafe {
            *out_doc = Box::into_raw(doc_handle) as *mut c_void;
        }

        Ok(())
    }))
}

/// Release and free an opened document.
///
/// # Safety
/// `doc` must be a valid pointer returned by `rbd_document_open_*` or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_document_free(doc: *mut c_void) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !doc.is_null() {
            let _ = unsafe { Box::from_raw(doc as *mut RbdDocumentHandle) };
        }
    }));
}

/// Query the total page count of the document.
///
/// # Safety
/// `doc` and `out_page_count` must be non-null valid pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_document_get_page_count(
    doc: *mut c_void,
    out_page_count: *mut i32,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if doc.is_null() || out_page_count.is_null() {
            set_last_error("Null pointer passed to rbd_document_get_page_count");
            return Err(RBD_ERR_NULL_PTR);
        }

        let doc_handle = unsafe { &*(doc as *const RbdDocumentHandle) };
        let count = doc_handle
            .session
            .read()
            .page_count()
            .map_err(|e| map_core_error(&e))?;

        unsafe {
            *out_page_count = count as i32;
        }

        Ok(())
    }))
}

/// Query the dimensions (width and height in points) for a specific 0-based page index.
///
/// # Safety
/// `doc`, `out_width_pt`, and `out_height_pt` must be non-null valid pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_document_get_page_size(
    doc: *mut c_void,
    page_index: i32,
    out_width_pt: *mut f64,
    out_height_pt: *mut f64,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if doc.is_null() || out_width_pt.is_null() || out_height_pt.is_null() {
            set_last_error("Null pointer passed to rbd_document_get_page_size");
            return Err(RBD_ERR_NULL_PTR);
        }
        if page_index < 0 {
            set_last_error("Negative page index passed to rbd_document_get_page_size");
            return Err(RBD_ERR_PAGE_OUT_OF_RANGE);
        }

        let doc_handle = unsafe { &*(doc as *const RbdDocumentHandle) };
        let (w, h) = doc_handle
            .session
            .read()
            .page_dimensions(page_index as u32)
            .map_err(|e| map_core_error(&e))?;

        unsafe {
            *out_width_pt = w as f64;
            *out_height_pt = h as f64;
        }

        Ok(())
    }))
}

/// Render a single tile directly into a caller-provided native buffer (zero-copy friendly).
///
/// # Safety
/// `doc` and `destination_buffer` must be valid non-null pointers.
/// `destination_buffer` must point to at least `buffer_stride * tile_height` writable bytes.
#[allow(clippy::too_many_arguments)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_render_tile_into_buffer(
    doc: *mut c_void,
    page_index: i32,
    tile_x: i32,
    tile_y: i32,
    tile_width: i32,
    tile_height: i32,
    zoom: f64,
    destination_buffer: *mut u8,
    buffer_stride: i32,
    pixel_format: i32,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if doc.is_null() || destination_buffer.is_null() {
            set_last_error("Null pointer passed to rbd_render_tile_into_buffer");
            return Err(RBD_ERR_NULL_PTR);
        }
        if page_index < 0
            || tile_x < 0
            || tile_y < 0
            || tile_width <= 0
            || tile_height <= 0
            || zoom <= 0.0
        {
            set_last_error("Invalid dimension or coordinate parameter");
            return Err(RBD_ERR_INVALID_PARAM);
        }

        const MAX_BUFFER_STRIDE: i32 = 1024 * 1024;
        if buffer_stride <= 0 || buffer_stride > MAX_BUFFER_STRIDE {
            set_last_error("buffer_stride must be positive and within valid range");
            return Err(RBD_ERR_INVALID_PARAM);
        }

        if tile_width > (rbd_core::MAX_TILE_DIMENSION as i32)
            || tile_height > (rbd_core::MAX_TILE_DIMENSION as i32)
        {
            set_last_error("Tile dimension exceeds maximum allowed dimension");
            return Err(RBD_ERR_INVALID_PARAM);
        }

        let row_bytes = match (tile_width as usize).checked_mul(4) {
            Some(rb) => rb,
            None => {
                set_last_error("Buffer size overflow in tile_width * 4");
                return Err(RBD_ERR_INVALID_PARAM);
            }
        };
        if (buffer_stride as usize) < row_bytes {
            set_last_error("buffer_stride is smaller than tile_width * 4");
            return Err(RBD_ERR_INVALID_PARAM);
        }

        let format = PixelFormat::try_from(pixel_format).map_err(|e| map_engine_error(&e))?;
        let scale_dpi = (zoom * 72.0) as f32;
        let total_bytes = match (buffer_stride as usize).checked_mul(tile_height as usize) {
            Some(tb) => tb,
            None => {
                set_last_error("Buffer size overflow in buffer_stride * tile_height");
                return Err(RBD_ERR_INVALID_PARAM);
            }
        };

        let slice = unsafe { slice::from_raw_parts_mut(destination_buffer, total_bytes) };
        let doc_handle = unsafe { &*(doc as *const RbdDocumentHandle) };

        doc_handle
            .session
            .read()
            .render_tile_into_buffer(
                page_index as u32,
                tile_x as u32,
                tile_y as u32,
                tile_width as u32,
                tile_height as u32,
                scale_dpi,
                format,
                buffer_stride as usize,
                slice,
            )
            .map_err(|e| map_core_error(&e))?;

        Ok(())
    }))
}

/// Retrieve the last error message recorded on the calling thread.
///
/// # Safety
/// The returned pointer points to a thread-local null-terminated C string
/// valid until the next FFI call on the same thread.
#[unsafe(no_mangle)]
pub extern "C" fn rbd_get_last_error() -> *const c_char {
    LAST_ERROR.with(|cell| cell.borrow().as_ptr())
}

static VERSION_C_STR: &[u8] = b"0.1.0-phase0\0";

/// Retrieve the static null-terminated version string of the RBD engine.
#[unsafe(no_mangle)]
pub extern "C" fn rbd_version_string() -> *const c_char {
    VERSION_C_STR.as_ptr() as *const c_char
}

/// Extract text from a page in the document as a UTF-8 null-terminated string.
/// The returned string must be freed using `rbd_free_string`.
///
/// # Safety
/// `doc` must be a valid document handle and `out_utf8_ptr` must be a valid, writable pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_document_extract_page_text(
    doc: *mut c_void,
    page_index: u32,
    out_utf8_ptr: *mut *mut c_char,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if doc.is_null() || out_utf8_ptr.is_null() {
            set_last_error("Null pointer passed to rbd_document_extract_page_text");
            return Err(RBD_ERR_NULL_PTR);
        }

        let doc_handle = unsafe { &*(doc as *const RbdDocumentHandle) };
        let text = doc_handle
            .session
            .read()
            .extract_text(page_index)
            .map_err(|e| map_core_error(&e))?;

        let clean = text.replace('\0', " ");
        let cs = CString::new(clean).map_err(|_| {
            set_last_error("Failed to convert page text to CString");
            RBD_ERR_INVALID_PARAM
        })?;

        unsafe {
            *out_utf8_ptr = cs.into_raw();
        }
        Ok(())
    }))
}

/// Search for query text across the document and return hits serialized as a JSON string.
/// The returned JSON string must be freed using `rbd_free_string`.
///
/// # Safety
/// Pointers must be valid, and `out_json_utf8_ptr` must be a valid, writable pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_document_search_json(
    doc: *mut c_void,
    query_utf8: *const c_char,
    case_sensitive: i32,
    out_json_utf8_ptr: *mut *mut c_char,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if doc.is_null() || query_utf8.is_null() || out_json_utf8_ptr.is_null() {
            set_last_error("Null pointer passed to rbd_document_search_json");
            return Err(RBD_ERR_NULL_PTR);
        }

        let query = unsafe { CStr::from_ptr(query_utf8) }
            .to_str()
            .map_err(|_| {
                set_last_error("Invalid UTF-8 in query string");
                RBD_ERR_INVALID_PARAM
            })?;

        let doc_handle = unsafe { &*(doc as *const RbdDocumentHandle) };
        let hits = doc_handle
            .session
            .read()
            .search_document(query, case_sensitive != 0)
            .map_err(|e| map_core_error(&e))?;

        let json = serde_json::to_string(&hits).map_err(|e| {
            set_last_error(&format!("JSON serialization failed: {e}"));
            RBD_ERR_ENGINE_FAILURE
        })?;

        let cs = CString::new(json).map_err(|_| {
            set_last_error("Failed to convert search JSON to CString");
            RBD_ERR_INVALID_PARAM
        })?;

        unsafe {
            *out_json_utf8_ptr = cs.into_raw();
        }
        Ok(())
    }))
}

// ---------------------------------------------------------------------------
// Document Annotation Management & Undo / Redo Entrypoints
// ---------------------------------------------------------------------------

/// Add an annotation to the document session from a JSON string with undo support.
///
/// # Safety
/// `doc` must be a valid document handle and `annotation_json` must be a valid null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_document_add_annotation_json(
    doc: *mut c_void,
    annotation_json: *const c_char,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if doc.is_null() || annotation_json.is_null() {
            set_last_error("Null pointer passed to rbd_document_add_annotation_json");
            return Err(RBD_ERR_NULL_PTR);
        }

        let json_str = unsafe { CStr::from_ptr(annotation_json) }
            .to_str()
            .map_err(|_| {
                set_last_error("Invalid UTF-8 in annotation JSON");
                RBD_ERR_INVALID_PARAM
            })?;

        let annotation: rbd_engine::Annotation = serde_json::from_str(json_str).map_err(|e| {
            set_last_error(&format!("Failed to deserialize annotation JSON: {e}"));
            RBD_ERR_INVALID_PARAM
        })?;

        let doc_handle = unsafe { &*(doc as *const RbdDocumentHandle) };
        doc_handle
            .session
            .write()
            .add_annotation_undoable(annotation)
            .map_err(|e| map_core_error(&e))?;

        Ok(())
    }))
}

/// Remove an annotation by ID with undo support.
///
/// # Safety
/// `doc` must be a valid document handle.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_document_remove_annotation(
    doc: *mut c_void,
    annotation_id: u64,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if doc.is_null() {
            set_last_error("Null pointer passed to rbd_document_remove_annotation");
            return Err(RBD_ERR_NULL_PTR);
        }

        let doc_handle = unsafe { &*(doc as *const RbdDocumentHandle) };
        doc_handle
            .session
            .write()
            .remove_annotation_undoable(annotation_id)
            .map_err(|e| map_core_error(&e))?;

        Ok(())
    }))
}

/// Retrieve all annotations for a specific page as a JSON string.
/// The returned string must be freed using `rbd_free_string`.
///
/// # Safety
/// `doc` must be a valid document handle and `out_json_utf8_ptr` must be a writable pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_document_get_annotations_json(
    doc: *mut c_void,
    page_index: u32,
    out_json_utf8_ptr: *mut *mut c_char,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if doc.is_null() || out_json_utf8_ptr.is_null() {
            set_last_error("Null pointer passed to rbd_document_get_annotations_json");
            return Err(RBD_ERR_NULL_PTR);
        }

        let doc_handle = unsafe { &*(doc as *const RbdDocumentHandle) };
        let annotations = doc_handle.session.read().annotations_for_page(page_index);

        let json = serde_json::to_string(&annotations).map_err(|e| {
            set_last_error(&format!("Failed to serialize annotations: {e}"));
            RBD_ERR_ENGINE_FAILURE
        })?;

        let cs = CString::new(json).map_err(|_| {
            set_last_error("Failed to convert annotations JSON to CString");
            RBD_ERR_INVALID_PARAM
        })?;

        unsafe {
            *out_json_utf8_ptr = cs.into_raw();
        }
        Ok(())
    }))
}

/// Update an existing annotation with undo support.
///
/// # Safety
/// `doc` and `annotation_json` must be non-null valid pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_document_update_annotation_json(
    doc: *mut c_void,
    annotation_json: *const c_char,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if doc.is_null() || annotation_json.is_null() {
            set_last_error("Null pointer passed to rbd_document_update_annotation_json");
            return Err(RBD_ERR_NULL_PTR);
        }

        let json_str = unsafe { CStr::from_ptr(annotation_json) }
            .to_str()
            .map_err(|_| {
                set_last_error("Invalid UTF-8 in annotation JSON");
                RBD_ERR_INVALID_PARAM
            })?;

        let annotation: rbd_engine::Annotation = serde_json::from_str(json_str).map_err(|e| {
            set_last_error(&format!("Failed to deserialize annotation JSON: {e}"));
            RBD_ERR_INVALID_PARAM
        })?;

        let doc_handle = unsafe { &*(doc as *const RbdDocumentHandle) };
        doc_handle
            .session
            .write()
            .update_annotation_undoable(annotation)
            .map_err(|e| map_core_error(&e))?;

        Ok(())
    }))
}

/// Undo the last annotation action.
/// Sets `*out_undone = 1` if an action was undone, or `0` if nothing was available to undo.
///
/// # Safety
/// `doc` and `out_undone` must be non-null valid pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_document_undo(doc: *mut c_void, out_undone: *mut i32) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if doc.is_null() || out_undone.is_null() {
            set_last_error("Null pointer passed to rbd_document_undo");
            return Err(RBD_ERR_NULL_PTR);
        }

        let doc_handle = unsafe { &*(doc as *const RbdDocumentHandle) };
        let undone = doc_handle
            .session
            .write()
            .undo()
            .map_err(|e| map_core_error(&e))?;

        unsafe {
            *out_undone = if undone { 1 } else { 0 };
        }
        Ok(())
    }))
}

/// Redo the last undone annotation action.
/// Sets `*out_redone = 1` if an action was redone, or `0` if nothing was available to redo.
///
/// # Safety
/// `doc` and `out_redone` must be non-null valid pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_document_redo(doc: *mut c_void, out_redone: *mut i32) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if doc.is_null() || out_redone.is_null() {
            set_last_error("Null pointer passed to rbd_document_redo");
            return Err(RBD_ERR_NULL_PTR);
        }

        let doc_handle = unsafe { &*(doc as *const RbdDocumentHandle) };
        let redone = doc_handle
            .session
            .write()
            .redo()
            .map_err(|e| map_core_error(&e))?;

        unsafe {
            *out_redone = if redone { 1 } else { 0 };
        }
        Ok(())
    }))
}

/// Query whether an undo action is currently available.
///
/// # Safety
/// `doc` and `out_can_undo` must be non-null valid pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_document_can_undo(doc: *mut c_void, out_can_undo: *mut i32) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if doc.is_null() || out_can_undo.is_null() {
            set_last_error("Null pointer passed to rbd_document_can_undo");
            return Err(RBD_ERR_NULL_PTR);
        }

        let doc_handle = unsafe { &*(doc as *const RbdDocumentHandle) };
        unsafe {
            *out_can_undo = if doc_handle.session.read().can_undo() {
                1
            } else {
                0
            };
        }
        Ok(())
    }))
}

/// Query whether a redo action is currently available.
///
/// # Safety
/// `doc` and `out_can_redo` must be non-null valid pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_document_can_redo(doc: *mut c_void, out_can_redo: *mut i32) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if doc.is_null() || out_can_redo.is_null() {
            set_last_error("Null pointer passed to rbd_document_can_redo");
            return Err(RBD_ERR_NULL_PTR);
        }

        let doc_handle = unsafe { &*(doc as *const RbdDocumentHandle) };
        unsafe {
            *out_can_redo = if doc_handle.session.read().can_redo() {
                1
            } else {
                0
            };
        }
        Ok(())
    }))
}

/// Save annotations to the specified PDF file path.
/// If opened from a file, the original source file is used as input.
///
/// # Safety
/// `doc` and `output_path_utf8` must be non-null valid pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_document_save_annotations(
    doc: *mut c_void,
    output_path_utf8: *const c_char,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if doc.is_null() || output_path_utf8.is_null() {
            set_last_error("Null pointer passed to rbd_document_save_annotations");
            return Err(RBD_ERR_NULL_PTR);
        }

        let out_str = unsafe { CStr::from_ptr(output_path_utf8) }
            .to_str()
            .map_err(|_| {
                set_last_error("Invalid UTF-8 in output path");
                RBD_ERR_INVALID_PARAM
            })?;

        let out_path = Path::new(out_str);
        let doc_handle = unsafe { &*(doc as *const RbdDocumentHandle) };
        let session = doc_handle.session.read();

        let in_path_buf = match session.source_path() {
            Some(p) => p.to_path_buf(),
            None => {
                if out_path.exists() {
                    out_path.to_path_buf()
                } else {
                    set_last_error("Source file path unknown and output file does not exist");
                    return Err(RBD_ERR_FILE_IO);
                }
            }
        };

        session
            .save_annotations_to_pdf(&in_path_buf, out_path)
            .map_err(|e| map_core_error(&e))?;

        Ok(())
    }))
}

/// Save annotations specifying both input template path and output path.
///
/// # Safety
/// Pointers must be non-null valid pointers to null-terminated UTF-8 strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_document_save_annotations_to_path(
    doc: *mut c_void,
    input_path_utf8: *const c_char,
    output_path_utf8: *const c_char,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if doc.is_null() || input_path_utf8.is_null() || output_path_utf8.is_null() {
            set_last_error("Null pointer passed to rbd_document_save_annotations_to_path");
            return Err(RBD_ERR_NULL_PTR);
        }

        let in_str = unsafe { CStr::from_ptr(input_path_utf8) }
            .to_str()
            .map_err(|_| {
                set_last_error("Invalid UTF-8 in input path");
                RBD_ERR_INVALID_PARAM
            })?;
        let out_str = unsafe { CStr::from_ptr(output_path_utf8) }
            .to_str()
            .map_err(|_| {
                set_last_error("Invalid UTF-8 in output path");
                RBD_ERR_INVALID_PARAM
            })?;

        let doc_handle = unsafe { &*(doc as *const RbdDocumentHandle) };
        doc_handle
            .session
            .read()
            .save_annotations_to_pdf(Path::new(in_str), Path::new(out_str))
            .map_err(|e| map_core_error(&e))?;

        Ok(())
    }))
}

// ---------------------------------------------------------------------------
// Standalone PDF Page Operations Entrypoints
// ---------------------------------------------------------------------------

/// Merge multiple PDF documents into a single output file.
/// `in_paths_json` must be a valid JSON array of UTF-8 file paths.
///
/// # Safety
/// Pointers must be valid null-terminated UTF-8 strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_page_op_merge(
    out_path: *const c_char,
    in_paths_json: *const c_char,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if out_path.is_null() || in_paths_json.is_null() {
            set_last_error("Null pointer passed to rbd_page_op_merge");
            return Err(RBD_ERR_NULL_PTR);
        }

        let out_str = unsafe { CStr::from_ptr(out_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in output path");
            RBD_ERR_INVALID_PARAM
        })?;

        let json_str = unsafe { CStr::from_ptr(in_paths_json) }
            .to_str()
            .map_err(|_| {
                set_last_error("Invalid UTF-8 in in_paths_json");
                RBD_ERR_INVALID_PARAM
            })?;

        let paths: Vec<String> = serde_json::from_str(json_str).map_err(|e| {
            set_last_error(&format!("Failed to parse in_paths_json: {e}"));
            RBD_ERR_INVALID_PARAM
        })?;

        if paths.is_empty() {
            set_last_error("Empty paths array passed to rbd_page_op_merge");
            return Err(RBD_ERR_INVALID_PARAM);
        }

        let path_refs: Vec<&Path> = paths.iter().map(Path::new).collect();
        rbd_core::page_ops::merge(&path_refs, Path::new(out_str))
            .map_err(|e| map_core_error(&e))?;

        Ok(())
    }))
}

/// Split a page range [start_page, end_page] (1-based inclusive) into an output PDF.
///
/// # Safety
/// Pointers must be valid null-terminated UTF-8 strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_page_op_split(
    in_path: *const c_char,
    out_path: *const c_char,
    start_page: u32,
    end_page: u32,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if in_path.is_null() || out_path.is_null() {
            set_last_error("Null pointer passed to rbd_page_op_split");
            return Err(RBD_ERR_NULL_PTR);
        }

        let in_str = unsafe { CStr::from_ptr(in_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in in_path");
            RBD_ERR_INVALID_PARAM
        })?;
        let out_str = unsafe { CStr::from_ptr(out_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in out_path");
            RBD_ERR_INVALID_PARAM
        })?;

        rbd_core::page_ops::split(Path::new(in_str), start_page, end_page, Path::new(out_str))
            .map_err(|e| map_core_error(&e))?;

        Ok(())
    }))
}

/// Rotate all pages in a PDF document by `degrees` (must be a multiple of 90).
///
/// # Safety
/// Pointers must be valid null-terminated UTF-8 strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_page_op_rotate(
    in_path: *const c_char,
    out_path: *const c_char,
    degrees: i32,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if in_path.is_null() || out_path.is_null() {
            set_last_error("Null pointer passed to rbd_page_op_rotate");
            return Err(RBD_ERR_NULL_PTR);
        }

        let in_str = unsafe { CStr::from_ptr(in_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in in_path");
            RBD_ERR_INVALID_PARAM
        })?;
        let out_str = unsafe { CStr::from_ptr(out_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in out_path");
            RBD_ERR_INVALID_PARAM
        })?;

        rbd_core::page_ops::rotate(Path::new(in_str), degrees, Path::new(out_str))
            .map_err(|e| map_core_error(&e))?;

        Ok(())
    }))
}

/// Delete pages specified in `pages_json` (1-based page numbers JSON array).
///
/// # Safety
/// Pointers must be valid null-terminated UTF-8 strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_page_op_delete_pages(
    in_path: *const c_char,
    out_path: *const c_char,
    pages_json: *const c_char,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if in_path.is_null() || out_path.is_null() || pages_json.is_null() {
            set_last_error("Null pointer passed to rbd_page_op_delete_pages");
            return Err(RBD_ERR_NULL_PTR);
        }

        let in_str = unsafe { CStr::from_ptr(in_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in in_path");
            RBD_ERR_INVALID_PARAM
        })?;
        let out_str = unsafe { CStr::from_ptr(out_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in out_path");
            RBD_ERR_INVALID_PARAM
        })?;
        let json_str = unsafe { CStr::from_ptr(pages_json) }
            .to_str()
            .map_err(|_| {
                set_last_error("Invalid UTF-8 in pages_json");
                RBD_ERR_INVALID_PARAM
            })?;

        let pages: Vec<u32> = serde_json::from_str(json_str).map_err(|e| {
            set_last_error(&format!("Failed to parse pages_json: {e}"));
            RBD_ERR_INVALID_PARAM
        })?;

        rbd_core::page_ops::delete_pages(Path::new(in_str), &pages, Path::new(out_str))
            .map_err(|e| map_core_error(&e))?;

        Ok(())
    }))
}

/// Reorder pages according to `order_json` (1-based page numbers JSON array).
///
/// # Safety
/// Pointers must be valid null-terminated UTF-8 strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_page_op_reorder(
    in_path: *const c_char,
    out_path: *const c_char,
    order_json: *const c_char,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if in_path.is_null() || out_path.is_null() || order_json.is_null() {
            set_last_error("Null pointer passed to rbd_page_op_reorder");
            return Err(RBD_ERR_NULL_PTR);
        }

        let in_str = unsafe { CStr::from_ptr(in_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in in_path");
            RBD_ERR_INVALID_PARAM
        })?;
        let out_str = unsafe { CStr::from_ptr(out_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in out_path");
            RBD_ERR_INVALID_PARAM
        })?;
        let json_str = unsafe { CStr::from_ptr(order_json) }
            .to_str()
            .map_err(|_| {
                set_last_error("Invalid UTF-8 in order_json");
                RBD_ERR_INVALID_PARAM
            })?;

        let order: Vec<u32> = serde_json::from_str(json_str).map_err(|e| {
            set_last_error(&format!("Failed to parse order_json: {e}"));
            RBD_ERR_INVALID_PARAM
        })?;

        rbd_core::page_ops::reorder_pages(Path::new(in_str), Path::new(out_str), &order)
            .map_err(|e| map_core_error(&e))?;

        Ok(())
    }))
}

/// Crop the 1-based `page_num` to bounding box `[x0, y0, x1, y1]` in points.
///
/// # Safety
/// Pointers must be valid null-terminated UTF-8 strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_page_op_crop(
    in_path: *const c_char,
    out_path: *const c_char,
    page_num: u32,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if in_path.is_null() || out_path.is_null() {
            set_last_error("Null pointer passed to rbd_page_op_crop");
            return Err(RBD_ERR_NULL_PTR);
        }

        let in_str = unsafe { CStr::from_ptr(in_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in in_path");
            RBD_ERR_INVALID_PARAM
        })?;
        let out_str = unsafe { CStr::from_ptr(out_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in out_path");
            RBD_ERR_INVALID_PARAM
        })?;

        rbd_core::page_ops::crop_page(
            Path::new(in_str),
            Path::new(out_str),
            page_num,
            [x0, y0, x1, y1],
        )
        .map_err(|e| map_core_error(&e))?;

        Ok(())
    }))
}

/// Insert a blank page after 1-based `after_page` with dimensions `width_pt` x `height_pt`.
///
/// # Safety
/// Pointers must be valid null-terminated UTF-8 strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_page_op_insert_blank(
    in_path: *const c_char,
    out_path: *const c_char,
    after_page: u32,
    width_pt: f32,
    height_pt: f32,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if in_path.is_null() || out_path.is_null() {
            set_last_error("Null pointer passed to rbd_page_op_insert_blank");
            return Err(RBD_ERR_NULL_PTR);
        }

        if width_pt <= 0.0 || height_pt <= 0.0 {
            set_last_error("Invalid page dimensions for blank page");
            return Err(RBD_ERR_INVALID_PARAM);
        }

        let in_str = unsafe { CStr::from_ptr(in_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in in_path");
            RBD_ERR_INVALID_PARAM
        })?;
        let out_str = unsafe { CStr::from_ptr(out_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in out_path");
            RBD_ERR_INVALID_PARAM
        })?;

        rbd_core::page_ops::insert_blank_page(
            Path::new(in_str),
            Path::new(out_str),
            after_page,
            width_pt,
            height_pt,
        )
        .map_err(|e| map_core_error(&e))?;

        Ok(())
    }))
}

/// Decrypt a password-protected PDF document.
///
/// # Safety
/// `in_path` and `out_path` must be valid null-terminated UTF-8 strings.
/// `password_utf8` may be null (empty password).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_page_op_decrypt(
    in_path: *const c_char,
    out_path: *const c_char,
    password_utf8: *const c_char,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if in_path.is_null() || out_path.is_null() {
            set_last_error("Null pointer passed to rbd_page_op_decrypt");
            return Err(RBD_ERR_NULL_PTR);
        }

        let in_str = unsafe { CStr::from_ptr(in_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in in_path");
            RBD_ERR_INVALID_PARAM
        })?;
        let out_str = unsafe { CStr::from_ptr(out_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in out_path");
            RBD_ERR_INVALID_PARAM
        })?;

        let password = if password_utf8.is_null() {
            ""
        } else {
            unsafe { CStr::from_ptr(password_utf8) }
                .to_str()
                .map_err(|_| {
                    set_last_error("Invalid UTF-8 in password");
                    RBD_ERR_INVALID_PARAM
                })?
        };

        rbd_core::page_ops::decrypt_pdf(Path::new(in_str), Path::new(out_str), password)
            .map_err(|e| map_core_error(&e))?;

        Ok(())
    }))
}

// ---------------------------------------------------------------------------
// Professional PDF Tools C-ABI Entrypoints (Phase 4 Track 4B)
// ---------------------------------------------------------------------------

fn copy_str_to_caller_buffer(s: &str, out_buf: *mut c_char, max_len: usize) -> Result<(), i32> {
    if out_buf.is_null() || max_len == 0 {
        set_last_error("Null or zero-sized destination buffer");
        return Err(RBD_ERR_BUFFER_TOO_SMALL);
    }
    let bytes = s.as_bytes();
    if bytes.len() + 1 > max_len {
        set_last_error("Caller destination buffer too small for string");
        return Err(RBD_ERR_BUFFER_TOO_SMALL);
    }
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_buf as *mut u8, bytes.len());
        *out_buf.add(bytes.len()) = 0;
    }
    Ok(())
}

fn map_stamp_position(pos: i32, default_pos: rbd_core::StampPosition) -> rbd_core::StampPosition {
    match pos {
        0 => rbd_core::StampPosition::TopLeft,
        1 => rbd_core::StampPosition::TopCenter,
        2 => rbd_core::StampPosition::TopRight,
        3 => rbd_core::StampPosition::BottomLeft,
        4 => rbd_core::StampPosition::BottomCenter,
        5 => rbd_core::StampPosition::BottomRight,
        _ => default_pos,
    }
}

/// Apply permanent redactions to a PDF document, purging text and drawing blackout boxes.
///
/// # Safety
/// Pointers must be valid null-terminated UTF-8 strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_pro_redact_document(
    in_path: *const c_char,
    out_path: *const c_char,
    redactions_json: *const c_char,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if in_path.is_null() || out_path.is_null() || redactions_json.is_null() {
            set_last_error("Null pointer passed to rbd_pro_redact_document");
            return Err(RBD_ERR_NULL_PTR);
        }

        let in_str = unsafe { CStr::from_ptr(in_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in in_path");
            RBD_ERR_INVALID_PARAM
        })?;
        let out_str = unsafe { CStr::from_ptr(out_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in out_path");
            RBD_ERR_INVALID_PARAM
        })?;
        let json_str = unsafe { CStr::from_ptr(redactions_json) }
            .to_str()
            .map_err(|_| {
                set_last_error("Invalid UTF-8 in redactions_json");
                RBD_ERR_INVALID_PARAM
            })?;

        let redactions: Vec<rbd_core::PageRedaction> =
            serde_json::from_str(json_str).map_err(|e| {
                set_last_error(&format!("Failed to parse redactions JSON: {e}"));
                RBD_ERR_INVALID_PARAM
            })?;

        rbd_core::redact_document(Path::new(in_str), &redactions, Path::new(out_str))
            .map_err(|e| map_engine_error(&e))?;

        Ok(())
    }))
}

/// Compress a PDF document and optionally output compression statistics as JSON.
///
/// # Safety
/// `in_path` and `out_path` must be valid null-terminated UTF-8 strings.
/// `out_savings_json` may be null if stats are not requested; if non-null, `max_savings_len` must reflect buffer size.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_pro_compress_document(
    in_path: *const c_char,
    out_path: *const c_char,
    remove_unused: i32,
    recompress: i32,
    strip_meta: i32,
    out_savings_json: *mut c_char,
    max_savings_len: usize,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if in_path.is_null() || out_path.is_null() {
            set_last_error("Null pointer passed to rbd_pro_compress_document");
            return Err(RBD_ERR_NULL_PTR);
        }

        let in_str = unsafe { CStr::from_ptr(in_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in in_path");
            RBD_ERR_INVALID_PARAM
        })?;
        let out_str = unsafe { CStr::from_ptr(out_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in out_path");
            RBD_ERR_INVALID_PARAM
        })?;

        let options = rbd_core::CompressionOptions {
            remove_unused_objects: remove_unused != 0,
            recompress_streams: recompress != 0,
            strip_metadata: strip_meta != 0,
        };

        let result = rbd_core::compress_document(Path::new(in_str), &options, Path::new(out_str))
            .map_err(|e| map_engine_error(&e))?;

        if !out_savings_json.is_null() && max_savings_len > 0 {
            let json = serde_json::to_string(&result).map_err(|e| {
                set_last_error(&format!("Failed to serialize compression result: {e}"));
                RBD_ERR_ENGINE_FAILURE
            })?;
            copy_str_to_caller_buffer(&json, out_savings_json, max_savings_len)?;
        }

        Ok(())
    }))
}

/// Apply a watermark to all pages in a PDF document.
///
/// # Safety
/// Pointers must be valid null-terminated UTF-8 strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_pro_apply_watermark(
    in_path: *const c_char,
    out_path: *const c_char,
    text: *const c_char,
    font_size: f32,
    rotation_deg: f32,
    opacity: f32,
    r: u8,
    g: u8,
    b: u8,
    on_top: i32,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if in_path.is_null() || out_path.is_null() || text.is_null() {
            set_last_error("Null pointer passed to rbd_pro_apply_watermark");
            return Err(RBD_ERR_NULL_PTR);
        }

        let in_str = unsafe { CStr::from_ptr(in_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in in_path");
            RBD_ERR_INVALID_PARAM
        })?;
        let out_str = unsafe { CStr::from_ptr(out_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in out_path");
            RBD_ERR_INVALID_PARAM
        })?;
        let text_str = unsafe { CStr::from_ptr(text) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in text");
            RBD_ERR_INVALID_PARAM
        })?;

        let options = rbd_core::WatermarkOptions {
            text: text_str.to_string(),
            font_size: if font_size > 0.0 { font_size } else { 48.0 },
            rotation_degrees: rotation_deg,
            opacity: opacity.clamp(0.0, 1.0),
            color_rgb: (r, g, b),
            on_top: on_top != 0,
        };

        rbd_core::apply_watermark(Path::new(in_str), &options, Path::new(out_str))
            .map_err(|e| map_engine_error(&e))?;

        Ok(())
    }))
}

/// Apply dynamic page numbers to pages using template pattern (e.g. "Page {page} of {total}").
///
/// # Safety
/// Pointers must be valid null-terminated UTF-8 strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_pro_apply_page_numbers(
    in_path: *const c_char,
    out_path: *const c_char,
    pattern: *const c_char,
    position: i32,
    font_size: f32,
    margin: f32,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if in_path.is_null() || out_path.is_null() || pattern.is_null() {
            set_last_error("Null pointer passed to rbd_pro_apply_page_numbers");
            return Err(RBD_ERR_NULL_PTR);
        }

        let in_str = unsafe { CStr::from_ptr(in_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in in_path");
            RBD_ERR_INVALID_PARAM
        })?;
        let out_str = unsafe { CStr::from_ptr(out_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in out_path");
            RBD_ERR_INVALID_PARAM
        })?;
        let pattern_str = unsafe { CStr::from_ptr(pattern) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in pattern");
            RBD_ERR_INVALID_PARAM
        })?;

        let pos = map_stamp_position(position, rbd_core::StampPosition::BottomCenter);
        let options = rbd_core::PageNumberOptions {
            template: pattern_str.to_string(),
            position: pos,
            font_size: if font_size > 0.0 { font_size } else { 10.0 },
            margin: if margin > 0.0 { margin } else { 36.0 },
            color_rgb: (80, 80, 80),
            start_page: 1,
        };

        rbd_core::apply_page_numbers(Path::new(in_str), &options, Path::new(out_str))
            .map_err(|e| map_engine_error(&e))?;

        Ok(())
    }))
}

/// Apply sequential legal Bates stamp numbers across pages.
///
/// # Safety
/// Pointers must be valid null-terminated UTF-8 strings (prefix and suffix can be null).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_pro_apply_bates_stamp(
    in_path: *const c_char,
    out_path: *const c_char,
    prefix: *const c_char,
    start_number: u32,
    padding_digits: u32,
    suffix: *const c_char,
    position: i32,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if in_path.is_null() || out_path.is_null() {
            set_last_error("Null pointer passed to rbd_pro_apply_bates_stamp");
            return Err(RBD_ERR_NULL_PTR);
        }

        let in_str = unsafe { CStr::from_ptr(in_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in in_path");
            RBD_ERR_INVALID_PARAM
        })?;
        let out_str = unsafe { CStr::from_ptr(out_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in out_path");
            RBD_ERR_INVALID_PARAM
        })?;

        let prefix_str = if !prefix.is_null() {
            unsafe { CStr::from_ptr(prefix) }.to_str().map_err(|_| {
                set_last_error("Invalid UTF-8 in prefix");
                RBD_ERR_INVALID_PARAM
            })?
        } else {
            ""
        };

        let suffix_str = if !suffix.is_null() {
            unsafe { CStr::from_ptr(suffix) }.to_str().map_err(|_| {
                set_last_error("Invalid UTF-8 in suffix");
                RBD_ERR_INVALID_PARAM
            })?
        } else {
            ""
        };

        let pos = map_stamp_position(position, rbd_core::StampPosition::BottomRight);
        let options = rbd_core::BatesOptions {
            prefix: prefix_str.to_string(),
            start_number,
            padding: padding_digits as usize,
            suffix: suffix_str.to_string(),
            position: pos,
            margin: 36.0,
            color_rgb: (0, 0, 0),
        };

        rbd_core::apply_bates_stamp(Path::new(in_str), &options, Path::new(out_str))
            .map_err(|e| map_engine_error(&e))?;

        Ok(())
    }))
}

/// Read all interactive form fields from a PDF file as a JSON array.
///
/// # Safety
/// `in_path` must be a valid null-terminated UTF-8 string.
/// `out_json` must be a writable buffer of at least `max_json_len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_pro_read_form_fields(
    in_path: *const c_char,
    out_json: *mut c_char,
    max_json_len: usize,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if in_path.is_null() || out_json.is_null() || max_json_len == 0 {
            set_last_error("Null pointer or zero buffer length passed to rbd_pro_read_form_fields");
            return Err(RBD_ERR_NULL_PTR);
        }

        let in_str = unsafe { CStr::from_ptr(in_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in in_path");
            RBD_ERR_INVALID_PARAM
        })?;

        let fields =
            rbd_core::read_form_fields(Path::new(in_str)).map_err(|e| map_engine_error(&e))?;

        let json = serde_json::to_string(&fields).map_err(|e| {
            set_last_error(&format!("Failed to serialize form fields: {e}"));
            RBD_ERR_ENGINE_FAILURE
        })?;

        copy_str_to_caller_buffer(&json, out_json, max_json_len)?;

        Ok(())
    }))
}

/// Fill interactive form fields in a PDF document and optionally flatten them.
///
/// # Safety
/// Pointers must be valid null-terminated UTF-8 strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_pro_fill_form_fields(
    in_path: *const c_char,
    out_path: *const c_char,
    field_values_json: *const c_char,
    flatten: i32,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if in_path.is_null() || out_path.is_null() || field_values_json.is_null() {
            set_last_error("Null pointer passed to rbd_pro_fill_form_fields");
            return Err(RBD_ERR_NULL_PTR);
        }

        let in_str = unsafe { CStr::from_ptr(in_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in in_path");
            RBD_ERR_INVALID_PARAM
        })?;
        let out_str = unsafe { CStr::from_ptr(out_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in out_path");
            RBD_ERR_INVALID_PARAM
        })?;
        let json_str = unsafe { CStr::from_ptr(field_values_json) }
            .to_str()
            .map_err(|_| {
                set_last_error("Invalid UTF-8 in field_values_json");
                RBD_ERR_INVALID_PARAM
            })?;

        let field_values: std::collections::HashMap<String, String> =
            serde_json::from_str(json_str).map_err(|e| {
                set_last_error(&format!("Failed to parse field_values_json: {e}"));
                RBD_ERR_INVALID_PARAM
            })?;

        rbd_core::fill_form_fields(
            Path::new(in_str),
            &field_values,
            flatten != 0,
            Path::new(out_str),
        )
        .map_err(|e| map_engine_error(&e))?;

        Ok(())
    }))
}

// ---------------------------------------------------------------------------
// Document Converter C-ABI Entrypoints (Phase 5 Track 5B)
// ---------------------------------------------------------------------------

/// Convert PDF pages to image files (PNG or JPEG) with configurable DPI.
///
/// # Safety
/// Pointers must be valid null-terminated UTF-8 strings.
/// `page_range_json` may be null (converts all pages).
/// `out_paths_json` buffer of length `max_paths_len` receives the JSON array of generated image paths.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_convert_pdf_to_images(
    in_path: *const c_char,
    out_dir: *const c_char,
    dpi: u32,
    format: i32,
    page_range_json: *const c_char,
    out_paths_json: *mut c_char,
    max_paths_len: usize,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if in_path.is_null() || out_dir.is_null() {
            set_last_error("Null pointer passed to rbd_convert_pdf_to_images");
            return Err(RBD_ERR_NULL_PTR);
        }

        if dpi == 0 {
            set_last_error("DPI must be greater than 0");
            return Err(RBD_ERR_INVALID_PARAM);
        }

        let in_str = unsafe { CStr::from_ptr(in_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in in_path");
            RBD_ERR_INVALID_PARAM
        })?;
        let out_str = unsafe { CStr::from_ptr(out_dir) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in out_dir");
            RBD_ERR_INVALID_PARAM
        })?;

        let img_format = match format {
            0 => rbd_core::ImageFormatType::Png,
            1 => rbd_core::ImageFormatType::Jpeg,
            _ => {
                set_last_error("Invalid format: expected 0 (Png) or 1 (Jpeg)");
                return Err(RBD_ERR_INVALID_PARAM);
            }
        };

        let page_range = if page_range_json.is_null() {
            None
        } else {
            let json_str = unsafe { CStr::from_ptr(page_range_json) }
                .to_str()
                .map_err(|_| {
                    set_last_error("Invalid UTF-8 in page_range_json");
                    RBD_ERR_INVALID_PARAM
                })?;
            let trimmed = json_str.trim();
            if trimmed.is_empty() || trimmed == "null" {
                None
            } else {
                let range: Vec<u32> = serde_json::from_str(trimmed).map_err(|e| {
                    set_last_error(&format!("Failed to parse page_range_json: {e}"));
                    RBD_ERR_INVALID_PARAM
                })?;
                Some(range)
            }
        };

        let options = rbd_core::PdfToImageOptions {
            dpi,
            format: img_format,
            page_range,
        };

        let generated_paths =
            rbd_core::pdf_to_images(Path::new(in_str), &options, Path::new(out_str))
                .map_err(|e| map_engine_error(&e))?;

        if !out_paths_json.is_null() && max_paths_len > 0 {
            let path_strings: Vec<String> = generated_paths
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            let json = serde_json::to_string(&path_strings).map_err(|e| {
                set_last_error(&format!("Failed to serialize paths to JSON: {e}"));
                RBD_ERR_ENGINE_FAILURE
            })?;
            copy_str_to_caller_buffer(&json, out_paths_json, max_paths_len)?;
        }

        Ok(())
    }))
}

/// Aggregate one or more image files into a single PDF document.
///
/// # Safety
/// Pointers must be valid null-terminated UTF-8 strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_convert_images_to_pdf(
    images_json: *const c_char,
    page_size: i32,
    margin_pt: f32,
    out_path: *const c_char,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if images_json.is_null() || out_path.is_null() {
            set_last_error("Null pointer passed to rbd_convert_images_to_pdf");
            return Err(RBD_ERR_NULL_PTR);
        }

        if margin_pt < 0.0 {
            set_last_error("margin_pt cannot be negative");
            return Err(RBD_ERR_INVALID_PARAM);
        }

        let out_str = unsafe { CStr::from_ptr(out_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in out_path");
            RBD_ERR_INVALID_PARAM
        })?;
        let json_str = unsafe { CStr::from_ptr(images_json) }
            .to_str()
            .map_err(|_| {
                set_last_error("Invalid UTF-8 in images_json");
                RBD_ERR_INVALID_PARAM
            })?;

        let image_path_strings: Vec<String> = serde_json::from_str(json_str).map_err(|e| {
            set_last_error(&format!("Failed to parse images_json: {e}"));
            RBD_ERR_INVALID_PARAM
        })?;

        if image_path_strings.is_empty() {
            set_last_error("No image paths provided in images_json");
            return Err(RBD_ERR_INVALID_PARAM);
        }

        let image_paths: Vec<std::path::PathBuf> = image_path_strings
            .into_iter()
            .map(std::path::PathBuf::from)
            .collect();

        let target_page_size = match page_size {
            0 => rbd_core::PageSize::A4,
            1 => rbd_core::PageSize::Letter,
            2 => rbd_core::PageSize::FitImage,
            _ => {
                set_last_error("Invalid page_size: expected 0 (A4), 1 (Letter), or 2 (FitImage)");
                return Err(RBD_ERR_INVALID_PARAM);
            }
        };

        let options = rbd_core::ImageToPdfOptions {
            page_size: target_page_size,
            margin_pt,
        };

        rbd_core::images_to_pdf(&image_paths, &options, Path::new(out_str))
            .map_err(|e| map_engine_error(&e))?;

        Ok(())
    }))
}

/// Convert markdown text into a paginated PDF document.
///
/// # Safety
/// Pointers must be valid null-terminated UTF-8 strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_convert_markdown_to_pdf(
    markdown_text: *const c_char,
    out_path: *const c_char,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if markdown_text.is_null() || out_path.is_null() {
            set_last_error("Null pointer passed to rbd_convert_markdown_to_pdf");
            return Err(RBD_ERR_NULL_PTR);
        }

        let md_str = unsafe { CStr::from_ptr(markdown_text) }
            .to_str()
            .map_err(|_| {
                set_last_error("Invalid UTF-8 in markdown_text");
                RBD_ERR_INVALID_PARAM
            })?;
        let out_str = unsafe { CStr::from_ptr(out_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in out_path");
            RBD_ERR_INVALID_PARAM
        })?;

        rbd_core::markdown_to_pdf(md_str, Path::new(out_str)).map_err(|e| map_engine_error(&e))?;

        Ok(())
    }))
}

// ---------------------------------------------------------------------------
// Document Scanner & Image Processing Functions
// ---------------------------------------------------------------------------

/// Compute standard ITU-R BT.601 luminance for RGB channels.
///
/// # Safety
/// `out_luminance` must point to valid writable memory.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_scan_compute_luminance(
    r: u8,
    g: u8,
    b: u8,
    out_luminance: *mut u8,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if out_luminance.is_null() {
            set_last_error("Null pointer passed to rbd_scan_compute_luminance");
            return Err(RBD_ERR_NULL_PTR);
        }
        unsafe {
            *out_luminance = rbd_core::compute_luminance(r, g, b);
        }
        Ok(())
    }))
}

/// Calculate optimal Otsu threshold from a 256-bin histogram.
///
/// # Safety
/// `histogram` must point to an array of 256 `uint32_t` elements.
/// `out_threshold` must point to valid writable memory.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_scan_calculate_otsu_threshold(
    histogram: *const u32,
    total_pixels: usize,
    out_threshold: *mut u8,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if histogram.is_null() || out_threshold.is_null() {
            set_last_error("Null pointer passed to rbd_scan_calculate_otsu_threshold");
            return Err(RBD_ERR_NULL_PTR);
        }
        let hist_slice = unsafe { slice::from_raw_parts(histogram, 256) };
        let mut hist_arr = [0u32; 256];
        hist_arr.copy_from_slice(hist_slice);
        unsafe {
            *out_threshold = rbd_core::calculate_otsu_threshold(&hist_arr, total_pixels);
        }
        Ok(())
    }))
}

/// Convert an RGBA pixel buffer to 8-bit grayscale in-place.
///
/// # Safety
/// `buffer` must point to a writable buffer of at least `stride * (height - 1) + width * 4` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_scan_to_grayscale(
    buffer: *mut u8,
    width: i32,
    height: i32,
    stride: i32,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if buffer.is_null() {
            set_last_error("Null pointer passed to rbd_scan_to_grayscale");
            return Err(RBD_ERR_NULL_PTR);
        }
        if width <= 0 || height <= 0 || stride <= 0 {
            set_last_error("Invalid dimensions or stride in rbd_scan_to_grayscale");
            return Err(RBD_ERR_INVALID_PARAM);
        }
        let row_bytes = (width as usize).checked_mul(4).ok_or_else(|| {
            set_last_error("Width overflow in rbd_scan_to_grayscale");
            RBD_ERR_INVALID_PARAM
        })?;
        if (stride as usize) < row_bytes {
            set_last_error("Stride less than width * 4 in rbd_scan_to_grayscale");
            return Err(RBD_ERR_INVALID_PARAM);
        }
        let required_len = (stride as usize)
            .checked_mul(height as usize - 1)
            .and_then(|s| s.checked_add(row_bytes))
            .ok_or_else(|| {
                set_last_error("Buffer length overflow in rbd_scan_to_grayscale");
                RBD_ERR_INVALID_PARAM
            })?;
        let buf_slice = unsafe { slice::from_raw_parts_mut(buffer, required_len) };
        rbd_core::scan_grayscale(buf_slice, width as u32, height as u32, stride as usize)
            .map_err(|e| map_engine_error(&e))?;
        Ok(())
    }))
}

/// Convert an RGBA pixel buffer to high-contrast binarized (black text, white paper) in-place.
///
/// # Safety
/// `buffer` must point to a writable buffer of at least `stride * (height - 1) + width * 4` bytes.
/// `out_threshold` may be null or point to a writable `uint8_t` for the calculated threshold.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_scan_to_binarized(
    buffer: *mut u8,
    width: i32,
    height: i32,
    stride: i32,
    out_threshold: *mut u8,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if buffer.is_null() {
            set_last_error("Null pointer passed to rbd_scan_to_binarized");
            return Err(RBD_ERR_NULL_PTR);
        }
        if width <= 0 || height <= 0 || stride <= 0 {
            set_last_error("Invalid dimensions or stride in rbd_scan_to_binarized");
            return Err(RBD_ERR_INVALID_PARAM);
        }
        let row_bytes = (width as usize).checked_mul(4).ok_or_else(|| {
            set_last_error("Width overflow in rbd_scan_to_binarized");
            RBD_ERR_INVALID_PARAM
        })?;
        if (stride as usize) < row_bytes {
            set_last_error("Stride less than width * 4 in rbd_scan_to_binarized");
            return Err(RBD_ERR_INVALID_PARAM);
        }
        let required_len = (stride as usize)
            .checked_mul(height as usize - 1)
            .and_then(|s| s.checked_add(row_bytes))
            .ok_or_else(|| {
                set_last_error("Buffer length overflow in rbd_scan_to_binarized");
                RBD_ERR_INVALID_PARAM
            })?;
        let buf_slice = unsafe { slice::from_raw_parts_mut(buffer, required_len) };
        let threshold =
            rbd_core::scan_binarize(buf_slice, width as u32, height as u32, stride as usize)
                .map_err(|e| map_engine_error(&e))?;
        if !out_threshold.is_null() {
            unsafe {
                *out_threshold = threshold;
            }
        }
        Ok(())
    }))
}

/// Compile scanned images into a standard PDF document.
///
/// # Safety
/// Pointers must be valid null-terminated UTF-8 strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_scan_generate_pdf(
    images_json: *const c_char,
    out_path: *const c_char,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if images_json.is_null() || out_path.is_null() {
            set_last_error("Null pointer passed to rbd_scan_generate_pdf");
            return Err(RBD_ERR_NULL_PTR);
        }
        let out_str = unsafe { CStr::from_ptr(out_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in out_path");
            RBD_ERR_INVALID_PARAM
        })?;
        let json_str = unsafe { CStr::from_ptr(images_json) }
            .to_str()
            .map_err(|_| {
                set_last_error("Invalid UTF-8 in images_json");
                RBD_ERR_INVALID_PARAM
            })?;
        let image_path_strings: Vec<String> = serde_json::from_str(json_str).map_err(|e| {
            set_last_error(&format!("Failed to parse images_json: {e}"));
            RBD_ERR_INVALID_PARAM
        })?;
        if image_path_strings.is_empty() {
            set_last_error("No image paths provided in images_json");
            return Err(RBD_ERR_INVALID_PARAM);
        }
        let image_paths: Vec<std::path::PathBuf> = image_path_strings
            .into_iter()
            .map(std::path::PathBuf::from)
            .collect();
        rbd_core::scan_generate_pdf(&image_paths, Path::new(out_str))
            .map_err(|e| map_engine_error(&e))?;
        Ok(())
    }))
}

// ---------------------------------------------------------------------------
// Office Document Functions (DOCX, XLSX, PPTX inspection, extraction & creation)
// ---------------------------------------------------------------------------

/// Extract text from Office document (DOCX, XLSX, PPTX).
///
/// # Safety
/// Pointers must be valid null-terminated strings. `out_text_buf` must be writable with at least `max_len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_office_extract_text(
    in_path: *const c_char,
    out_text_buf: *mut c_char,
    max_len: usize,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if in_path.is_null() || out_text_buf.is_null() {
            set_last_error("Null pointer passed to rbd_office_extract_text");
            return Err(RBD_ERR_NULL_PTR);
        }
        if max_len == 0 {
            set_last_error("max_len must be greater than zero");
            return Err(RBD_ERR_INVALID_PARAM);
        }

        let path_str = unsafe { CStr::from_ptr(in_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in in_path");
            RBD_ERR_INVALID_PARAM
        })?;

        let text =
            rbd_core::extract_office_text(Path::new(path_str)).map_err(|e| map_engine_error(&e))?;

        let text_bytes = text.as_bytes();
        if text_bytes.len() + 1 > max_len {
            set_last_error(&format!(
                "Output buffer too small: required {}, provided {}",
                text_bytes.len() + 1,
                max_len
            ));
            return Err(RBD_ERR_BUFFER_TOO_SMALL);
        }

        unsafe {
            let slice = slice::from_raw_parts_mut(out_text_buf as *mut u8, max_len);
            slice[..text_bytes.len()].copy_from_slice(text_bytes);
            slice[text_bytes.len()] = 0;
        }

        Ok(())
    }))
}

/// Inspect Office document and return JSON metadata (doc_type, title, word_count, item_count, summary).
///
/// # Safety
/// Pointers must be valid null-terminated strings. `out_json_buf` must be writable with at least `max_len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_office_inspect(
    in_path: *const c_char,
    out_json_buf: *mut c_char,
    max_len: usize,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if in_path.is_null() || out_json_buf.is_null() {
            set_last_error("Null pointer passed to rbd_office_inspect");
            return Err(RBD_ERR_NULL_PTR);
        }
        if max_len == 0 {
            set_last_error("max_len must be greater than zero");
            return Err(RBD_ERR_INVALID_PARAM);
        }

        let path_str = unsafe { CStr::from_ptr(in_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in in_path");
            RBD_ERR_INVALID_PARAM
        })?;

        let info = rbd_core::inspect_office_document(Path::new(path_str))
            .map_err(|e| map_engine_error(&e))?;

        let json_str = serde_json::to_string(&info).map_err(|e| {
            set_last_error(&format!("JSON serialization failed: {}", e));
            RBD_ERR_ENGINE_FAILURE
        })?;

        let json_bytes = json_str.as_bytes();
        if json_bytes.len() + 1 > max_len {
            set_last_error(&format!(
                "Output buffer too small: required {}, provided {}",
                json_bytes.len() + 1,
                max_len
            ));
            return Err(RBD_ERR_BUFFER_TOO_SMALL);
        }

        unsafe {
            let slice = slice::from_raw_parts_mut(out_json_buf as *mut u8, max_len);
            slice[..json_bytes.len()].copy_from_slice(json_bytes);
            slice[json_bytes.len()] = 0;
        }

        Ok(())
    }))
}

/// Create a minimal standard-compliant DOCX file from raw text.
///
/// # Safety
/// Pointers must be valid null-terminated UTF-8 strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_office_create_docx(
    text: *const c_char,
    out_path: *const c_char,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if text.is_null() || out_path.is_null() {
            set_last_error("Null pointer passed to rbd_office_create_docx");
            return Err(RBD_ERR_NULL_PTR);
        }

        let text_str = unsafe { CStr::from_ptr(text) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in text");
            RBD_ERR_INVALID_PARAM
        })?;

        let out_str = unsafe { CStr::from_ptr(out_path) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in out_path");
            RBD_ERR_INVALID_PARAM
        })?;

        rbd_core::create_simple_docx(text_str, Path::new(out_str))
            .map_err(|e| map_engine_error(&e))?;

        Ok(())
    }))
}

// ---------------------------------------------------------------------------
// Phase 8: Offline AI, Document Q&A & Reflow Editing
// ---------------------------------------------------------------------------

/// Summarize document text using offline natural language processing (returns JSON).
///
/// # Safety
/// `doc` must be a valid document handle. `out_json_buf` must be writable with at least `max_len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_ai_summarize_document(
    doc: *mut c_void,
    max_sentences: u32,
    out_json_buf: *mut c_char,
    max_len: usize,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if doc.is_null() || out_json_buf.is_null() {
            set_last_error("Null pointer passed to rbd_ai_summarize_document");
            return Err(RBD_ERR_NULL_PTR);
        }
        if max_len == 0 {
            set_last_error("max_len must be greater than zero");
            return Err(RBD_ERR_INVALID_PARAM);
        }

        let doc_handle = unsafe { &*(doc as *const RbdDocumentHandle) };
        let summary = doc_handle
            .session
            .write()
            .summarize_document(max_sentences as usize)
            .map_err(|e| map_core_error(&e))?;

        let json_str = serde_json::to_string(&summary).map_err(|e| {
            set_last_error(&format!("JSON serialization failed: {e}"));
            RBD_ERR_ENGINE_FAILURE
        })?;

        copy_str_to_caller_buffer(&json_str, out_json_buf, max_len)?;
        Ok(())
    }))
}

/// Query the document using natural language and return the best passage with citation as JSON.
///
/// # Safety
/// `doc` must be a valid document handle. `query` and `out_json_buf` must be non-null and valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_ai_ask_document(
    doc: *mut c_void,
    query: *const c_char,
    out_json_buf: *mut c_char,
    max_len: usize,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if doc.is_null() || query.is_null() || out_json_buf.is_null() {
            set_last_error("Null pointer passed to rbd_ai_ask_document");
            return Err(RBD_ERR_NULL_PTR);
        }
        if max_len == 0 {
            set_last_error("max_len must be greater than zero");
            return Err(RBD_ERR_INVALID_PARAM);
        }

        let query_str = unsafe { CStr::from_ptr(query) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in query");
            RBD_ERR_INVALID_PARAM
        })?;

        let doc_handle = unsafe { &*(doc as *const RbdDocumentHandle) };
        let answer = doc_handle
            .session
            .write()
            .ask_document(query_str)
            .map_err(|e| map_core_error(&e))?;

        let json_str = serde_json::to_string(&answer).map_err(|e| {
            set_last_error(&format!("JSON serialization failed: {e}"));
            RBD_ERR_ENGINE_FAILURE
        })?;

        copy_str_to_caller_buffer(&json_str, out_json_buf, max_len)?;
        Ok(())
    }))
}

/// Extract fluid, structured paragraphs from a page for reflow editing as JSON.
///
/// # Safety
/// `doc` must be a valid document handle. `out_json_buf` must be writable with at least `max_len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_document_reflow_page_text(
    doc: *mut c_void,
    page_index: u32,
    out_json_buf: *mut c_char,
    max_len: usize,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if doc.is_null() || out_json_buf.is_null() {
            set_last_error("Null pointer passed to rbd_document_reflow_page_text");
            return Err(RBD_ERR_NULL_PTR);
        }
        if max_len == 0 {
            set_last_error("max_len must be greater than zero");
            return Err(RBD_ERR_INVALID_PARAM);
        }

        let doc_handle = unsafe { &*(doc as *const RbdDocumentHandle) };
        let paragraphs = doc_handle
            .session
            .write()
            .extract_page_paragraphs(page_index)
            .map_err(|e| map_core_error(&e))?;

        let json_str = serde_json::to_string(&paragraphs).map_err(|e| {
            set_last_error(&format!("JSON serialization failed: {e}"));
            RBD_ERR_ENGINE_FAILURE
        })?;

        copy_str_to_caller_buffer(&json_str, out_json_buf, max_len)?;
        Ok(())
    }))
}

/// Replace text in a paragraph bounding box with automatic multi-line reflow.
///
/// # Safety
/// All pointers must be valid and null-terminated UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_document_replace_paragraph_text(
    doc: *mut c_void,
    input_path: *const c_char,
    page_index: u32,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    new_text: *const c_char,
    font_size: f32,
    output_path: *const c_char,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if doc.is_null() || input_path.is_null() || new_text.is_null() || output_path.is_null() {
            set_last_error("Null pointer passed to rbd_document_replace_paragraph_text");
            return Err(RBD_ERR_NULL_PTR);
        }

        let in_str = unsafe { CStr::from_ptr(input_path) }
            .to_str()
            .map_err(|_| {
                set_last_error("Invalid UTF-8 in input_path");
                RBD_ERR_INVALID_PARAM
            })?;
        let text_str = unsafe { CStr::from_ptr(new_text) }.to_str().map_err(|_| {
            set_last_error("Invalid UTF-8 in new_text");
            RBD_ERR_INVALID_PARAM
        })?;
        let out_str = unsafe { CStr::from_ptr(output_path) }
            .to_str()
            .map_err(|_| {
                set_last_error("Invalid UTF-8 in output_path");
                RBD_ERR_INVALID_PARAM
            })?;

        let doc_handle = unsafe { &*(doc as *const RbdDocumentHandle) };
        doc_handle
            .session
            .write()
            .replace_paragraph_text(
                Path::new(in_str),
                page_index,
                [x0, y0, x1, y1],
                text_str,
                if font_size > 0.0 { font_size } else { 11.0 },
                Path::new(out_str),
            )
            .map_err(|e| map_core_error(&e))?;

        Ok(())
    }))
}

// ---------------------------------------------------------------------------
// Existing Legacy RbdSession Entrypoints (retained for backward compatibility)
// ---------------------------------------------------------------------------

pub struct RbdSession {
    inner: DocumentSession,
}

/// Open a PDF document session from a UTF-8 file path.
///
/// # Safety
/// Pointers must point to valid null-terminated UTF-8 strings or null (for password),
/// and `out_session` must be a valid, writable pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_session_open_file(
    path_utf8: *const c_char,
    password_utf8: *const c_char,
    out_session: *mut *mut RbdSession,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if path_utf8.is_null() || out_session.is_null() {
            return Err(RBD_ERR_NULL_PTR);
        }

        let path_str = unsafe { CStr::from_ptr(path_utf8) }
            .to_str()
            .map_err(|_| RBD_ERR_INVALID_PARAM)?;

        let password = if password_utf8.is_null() {
            None
        } else {
            Some(
                unsafe { CStr::from_ptr(password_utf8) }
                    .to_str()
                    .map_err(|_| RBD_ERR_INVALID_PARAM)?,
            )
        };

        let path = Path::new(path_str);
        let session = DocumentSession::open_file(path, password, get_engine(), None)
            .map_err(|e| map_core_error(&e))?;

        let boxed = Box::new(RbdSession { inner: session });
        unsafe {
            *out_session = Box::into_raw(boxed);
        }

        Ok(())
    }))
}

/// Open a PDF document session from an in-memory byte buffer.
///
/// # Safety
/// `bytes` must point to at least `len` valid readable bytes.
/// `password_utf8` must be null or point to a valid null-terminated UTF-8 string.
/// `out_session` must be a valid, writable pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_session_open_bytes(
    bytes: *const u8,
    len: usize,
    password_utf8: *const c_char,
    out_session: *mut *mut RbdSession,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if bytes.is_null() || out_session.is_null() || len == 0 {
            return Err(RBD_ERR_NULL_PTR);
        }

        let slice = unsafe { slice::from_raw_parts(bytes, len) };

        let password = if password_utf8.is_null() {
            None
        } else {
            Some(
                unsafe { CStr::from_ptr(password_utf8) }
                    .to_str()
                    .map_err(|_| RBD_ERR_INVALID_PARAM)?,
            )
        };

        let session = DocumentSession::open_bytes(slice, password, get_engine(), None)
            .map_err(|e| map_core_error(&e))?;

        let boxed = Box::new(RbdSession { inner: session });
        unsafe {
            *out_session = Box::into_raw(boxed);
        }

        Ok(())
    }))
}

/// Close and free a document session.
///
/// # Safety
/// `session` must be a valid pointer returned by `rbd_session_open_*` or null.
/// After calling this, `session` must not be dereferenced again.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_session_close(session: *mut RbdSession) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if session.is_null() {
            return Err(RBD_ERR_NULL_PTR);
        }
        let _ = unsafe { Box::from_raw(session) };
        Ok(())
    }))
}

/// Query the total page count of the document.
///
/// # Safety
/// `session` must be an active, valid pointer to `RbdSession`.
/// `out_count` must be a valid, writable pointer to `u32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_session_page_count(
    session: *mut RbdSession,
    out_count: *mut u32,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if session.is_null() || out_count.is_null() {
            return Err(RBD_ERR_NULL_PTR);
        }
        let rbd = unsafe { &*session };
        let count = rbd.inner.page_count().map_err(|e| map_core_error(&e))?;
        unsafe {
            *out_count = count;
        }
        Ok(())
    }))
}

/// Query page dimensions (width and height in points).
///
/// # Safety
/// `session` must be an active, valid pointer to `RbdSession`.
/// `out_width` and `out_height` must be valid, writable pointers to `f32`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_session_page_dimensions(
    session: *mut RbdSession,
    page: u32,
    out_width: *mut f32,
    out_height: *mut f32,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if session.is_null() || out_width.is_null() || out_height.is_null() {
            return Err(RBD_ERR_NULL_PTR);
        }
        let rbd = unsafe { &*session };
        let (w, h) = rbd
            .inner
            .page_dimensions(page)
            .map_err(|e| map_core_error(&e))?;
        unsafe {
            *out_width = w;
            *out_height = h;
        }
        Ok(())
    }))
}

/// Render a full page directly into a caller-provided native RGBA buffer (zero-copy friendly).
///
/// # Safety
/// `session` must be an active, valid pointer to `RbdSession`.
/// `out_rgba` must point to a writable memory region of at least `out_len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_session_render_page(
    session: *mut RbdSession,
    page: u32,
    width: u32,
    height: u32,
    out_rgba: *mut u8,
    out_len: usize,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if session.is_null() || out_rgba.is_null() {
            return Err(RBD_ERR_NULL_PTR);
        }
        let required = match (width as usize)
            .checked_mul(height as usize)
            .and_then(|wh| wh.checked_mul(4))
        {
            Some(req) => req,
            None => {
                set_last_error("Buffer size overflow in width * height * 4");
                return Err(RBD_ERR_INVALID_PARAM);
            }
        };
        if out_len < required {
            return Err(RBD_ERR_BUFFER_TOO_SMALL);
        }
        let rbd = unsafe { &*session };
        let slice = unsafe { slice::from_raw_parts_mut(out_rgba, out_len) };
        rbd.inner
            .render_page_into_buffer(page, width, height, slice)
            .map_err(|e| map_core_error(&e))?;
        Ok(())
    }))
}

/// Render a sub-rectangle tile directly into a caller-provided native RGBA buffer (zero-copy friendly).
///
/// # Safety
/// `session` must be an active, valid pointer to `RbdSession`.
/// `out_rgba` must point to a writable memory region of at least `out_len` bytes.
#[allow(clippy::too_many_arguments)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_session_render_tile_into_buffer(
    session: *mut RbdSession,
    page: u32,
    tile_x: u32,
    tile_y: u32,
    tile_w: u32,
    tile_h: u32,
    scale_dpi: f32,
    out_rgba: *mut u8,
    out_len: usize,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if session.is_null() || out_rgba.is_null() {
            return Err(RBD_ERR_NULL_PTR);
        }
        let required = match (tile_w as usize)
            .checked_mul(tile_h as usize)
            .and_then(|wh| wh.checked_mul(4))
        {
            Some(req) => req,
            None => {
                set_last_error("Buffer size overflow in tile_w * tile_h * 4");
                return Err(RBD_ERR_INVALID_PARAM);
            }
        };
        if out_len < required {
            return Err(RBD_ERR_BUFFER_TOO_SMALL);
        }
        let rbd = unsafe { &*session };
        let slice = unsafe { slice::from_raw_parts_mut(out_rgba, out_len) };
        rbd.inner
            .render_tile_into_rgba(page, tile_x, tile_y, tile_w, tile_h, scale_dpi, slice)
            .map_err(|e| map_core_error(&e))?;
        Ok(())
    }))
}

/// Extract UTF-8 text from page (preserves Turkish characters).
/// Caller must free returned string with `rbd_free_string`.
///
/// # Safety
/// `session` must be an active, valid pointer to `RbdSession`.
/// `out_text` must be a valid, writable pointer to a `*mut c_char`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_session_extract_text(
    session: *mut RbdSession,
    page: u32,
    out_text: *mut *mut c_char,
) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if session.is_null() || out_text.is_null() {
            return Err(RBD_ERR_NULL_PTR);
        }
        let rbd = unsafe { &*session };
        let text = rbd
            .inner
            .extract_text(page)
            .map_err(|e| map_core_error(&e))?;
        let c_str = CString::new(text).map_err(|_| RBD_ERR_INVALID_PARAM)?;
        unsafe {
            *out_text = c_str.into_raw();
        }
        Ok(())
    }))
}

/// Free a C string allocated by RBD FFI functions.
///
/// # Safety
/// `text` must be a pointer returned by an RBD FFI function (e.g. `rbd_session_extract_text`) or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rbd_free_string(text: *mut c_char) -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        if !text.is_null() {
            let _ = unsafe { CString::from_raw(text) };
        }
        Ok(())
    }))
}

/// Test function that simulates a panic inside FFI.
/// Demonstrates that the panic is caught and returns RBD_ERR_PANIC instead of aborting.
#[unsafe(no_mangle)]
pub extern "C" fn rbd_simulate_panic() -> i32 {
    ffi_boundary(AssertUnwindSafe(|| {
        panic!("Simulated critical error inside Rust core!");
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn test_null_pointer_checks() {
        unsafe {
            assert_eq!(
                rbd_session_open_file(ptr::null(), ptr::null(), ptr::null_mut()),
                RBD_ERR_NULL_PTR
            );
            assert_eq!(rbd_session_close(ptr::null_mut()), RBD_ERR_NULL_PTR);
            assert_eq!(rbd_engine_init(ptr::null_mut()), RBD_ERR_NULL_PTR);
            assert_eq!(
                rbd_document_open_file(ptr::null_mut(), ptr::null(), ptr::null_mut()),
                RBD_ERR_NULL_PTR
            );
        }
    }

    #[test]
    fn test_panic_containment() {
        let code = rbd_simulate_panic();
        assert_eq!(
            code, RBD_ERR_PANIC,
            "FFI boundary must contain panics and return RBD_ERR_PANIC"
        );
    }

    #[test]
    fn test_unified_c_abi_lifecycle_and_tile_render() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let pdf_path = manifest_dir.join("../../../corpus/files/pdf-basic/pdfium-hello-world.pdf");
        assert!(pdf_path.exists(), "Test PDF must exist");

        let c_path = CString::new(pdf_path.to_str().unwrap()).unwrap();

        unsafe {
            let mut engine: *mut c_void = ptr::null_mut();
            let res = rbd_engine_init(&mut engine);
            assert_eq!(res, RBD_OK);
            assert!(!engine.is_null());

            let mut doc: *mut c_void = ptr::null_mut();
            let res = rbd_document_open_file(engine, c_path.as_ptr(), &mut doc);
            assert_eq!(res, RBD_OK);
            assert!(!doc.is_null());

            let mut page_count: i32 = 0;
            let res = rbd_document_get_page_count(doc, &mut page_count);
            assert_eq!(res, RBD_OK);
            assert_eq!(page_count, 1);

            let mut width_pt = 0.0;
            let mut height_pt = 0.0;
            let res = rbd_document_get_page_size(doc, 0, &mut width_pt, &mut height_pt);
            assert_eq!(res, RBD_OK);
            assert!(width_pt > 0.0 && height_pt > 0.0);

            // Render 256x256 tile with Bgra32
            let stride = 256 * 4;
            let mut buffer = vec![0u8; stride * 256];
            let res = rbd_render_tile_into_buffer(
                doc,
                0,
                0,
                0,
                256,
                256,
                1.0,
                buffer.as_mut_ptr(),
                stride as i32,
                0, // Bgra32
            );
            assert_eq!(res, RBD_OK);

            let non_zero = buffer.iter().filter(|&&b| b != 0).count();
            assert!(
                non_zero > 0,
                "Rendered Bgra32 buffer must have non-zero pixels"
            );

            // Render 256x256 tile with Rgba32
            let mut rgba_buffer = vec![0u8; stride * 256];
            let res = rbd_render_tile_into_buffer(
                doc,
                0,
                0,
                0,
                256,
                256,
                1.0,
                rgba_buffer.as_mut_ptr(),
                stride as i32,
                1, // Rgba32
            );
            assert_eq!(res, RBD_OK);

            let rgba_non_zero = rgba_buffer.iter().filter(|&&b| b != 0).count();
            assert!(
                rgba_non_zero > 0,
                "Rendered Rgba32 buffer must have non-zero pixels"
            );

            // Verify version and last error pointers
            let version_ptr = rbd_version_string();
            assert!(!version_ptr.is_null());
            let version_str = CStr::from_ptr(version_ptr).to_str().unwrap();
            assert!(version_str.contains("0.1.0"));

            let err_ptr = rbd_get_last_error();
            assert!(!err_ptr.is_null());

            rbd_document_free(doc);
            rbd_engine_free(engine);
        }
    }

    #[test]
    fn test_ffi_extract_text_and_search_json() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let sample_pdf =
            manifest_dir.join("../../../corpus/files/pdf-basic/pdfium-hello-world.pdf");
        assert!(sample_pdf.exists());

        unsafe {
            let mut engine = std::ptr::null_mut();
            assert_eq!(rbd_engine_init(&mut engine), RBD_OK);

            let path_c = CString::new(sample_pdf.to_str().unwrap()).unwrap();
            let mut doc = std::ptr::null_mut();
            assert_eq!(
                rbd_document_open_file(engine, path_c.as_ptr(), &mut doc),
                RBD_OK
            );

            // 1. Extract text
            let mut text_ptr: *mut c_char = std::ptr::null_mut();
            let res = rbd_document_extract_page_text(doc, 0, &mut text_ptr);
            assert_eq!(res, RBD_OK);
            assert!(!text_ptr.is_null());
            let text = CStr::from_ptr(text_ptr).to_str().unwrap();
            assert!(text.contains("Hello, world!"));
            rbd_free_string(text_ptr);

            // 2. Search JSON
            let query_c = CString::new("Hello").unwrap();
            let mut json_ptr: *mut c_char = std::ptr::null_mut();
            let res = rbd_document_search_json(doc, query_c.as_ptr(), 0, &mut json_ptr);
            assert_eq!(res, RBD_OK);
            assert!(!json_ptr.is_null());
            let json = CStr::from_ptr(json_ptr).to_str().unwrap();
            assert!(json.contains("Hello"));
            assert!(json.contains("rects"));
            rbd_free_string(json_ptr);

            rbd_document_free(doc);
            rbd_engine_free(engine);
        }
    }

    #[test]
    fn test_ffi_annotations_and_undo_redo() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let sample_pdf =
            manifest_dir.join("../../../corpus/files/pdf-basic/pdfium-hello-world.pdf");
        assert!(sample_pdf.exists());

        unsafe {
            let mut engine = std::ptr::null_mut();
            assert_eq!(rbd_engine_init(&mut engine), RBD_OK);

            let path_c = CString::new(sample_pdf.to_str().unwrap()).unwrap();
            let mut doc = std::ptr::null_mut();
            assert_eq!(
                rbd_document_open_file(engine, path_c.as_ptr(), &mut doc),
                RBD_OK
            );

            // Verify initial annotations are empty
            let mut json_ptr: *mut c_char = std::ptr::null_mut();
            assert_eq!(
                rbd_document_get_annotations_json(doc, 0, &mut json_ptr),
                RBD_OK
            );
            assert!(!json_ptr.is_null());
            let json = CStr::from_ptr(json_ptr).to_str().unwrap();
            assert_eq!(json, "[]");
            rbd_free_string(json_ptr);

            // Add annotation
            let annot_json = r#"{
                "id": 42,
                "page_index": 0,
                "kind": "Highlight",
                "rect": { "x0": 10.0, "y0": 20.0, "x1": 100.0, "y1": 50.0 },
                "color_rgba": [255, 255, 0, 128],
                "contents": "Test annotation content"
            }"#;
            let annot_c = CString::new(annot_json).unwrap();
            assert_eq!(
                rbd_document_add_annotation_json(doc, annot_c.as_ptr()),
                RBD_OK
            );

            // Get annotations - verify presence
            let mut json_ptr: *mut c_char = std::ptr::null_mut();
            assert_eq!(
                rbd_document_get_annotations_json(doc, 0, &mut json_ptr),
                RBD_OK
            );
            let json = CStr::from_ptr(json_ptr).to_str().unwrap();
            assert!(json.contains("Test annotation content"));
            assert!(json.contains("Highlight"));
            rbd_free_string(json_ptr);

            // Test undo
            let mut undone: i32 = 0;
            assert_eq!(rbd_document_undo(doc, &mut undone), RBD_OK);
            assert_eq!(undone, 1);

            let mut json_ptr: *mut c_char = std::ptr::null_mut();
            assert_eq!(
                rbd_document_get_annotations_json(doc, 0, &mut json_ptr),
                RBD_OK
            );
            let json = CStr::from_ptr(json_ptr).to_str().unwrap();
            assert_eq!(json, "[]");
            rbd_free_string(json_ptr);

            // Test redo
            let mut redone: i32 = 0;
            assert_eq!(rbd_document_redo(doc, &mut redone), RBD_OK);
            assert_eq!(redone, 1);

            let mut json_ptr: *mut c_char = std::ptr::null_mut();
            assert_eq!(
                rbd_document_get_annotations_json(doc, 0, &mut json_ptr),
                RBD_OK
            );
            let json = CStr::from_ptr(json_ptr).to_str().unwrap();
            assert!(json.contains("Test annotation content"));
            rbd_free_string(json_ptr);

            // Save annotations to temporary output PDF
            let out_pdf =
                std::env::temp_dir().join(format!("rbd_ffi_annot_{}.pdf", std::process::id()));
            let out_c = CString::new(out_pdf.to_str().unwrap()).unwrap();
            assert_eq!(rbd_document_save_annotations(doc, out_c.as_ptr()), RBD_OK);
            assert!(out_pdf.exists());
            let _ = std::fs::remove_file(out_pdf);

            // Remove annotation
            assert_eq!(rbd_document_remove_annotation(doc, 42), RBD_OK);
            let mut json_ptr: *mut c_char = std::ptr::null_mut();
            assert_eq!(
                rbd_document_get_annotations_json(doc, 0, &mut json_ptr),
                RBD_OK
            );
            let json = CStr::from_ptr(json_ptr).to_str().unwrap();
            assert_eq!(json, "[]");
            rbd_free_string(json_ptr);

            rbd_document_free(doc);
            rbd_engine_free(engine);
        }
    }

    #[test]
    fn test_ffi_page_operations() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let sample_pdf =
            manifest_dir.join("../../../corpus/files/pdf-basic/pdfium-hello-world.pdf");
        assert!(sample_pdf.exists());

        let pid = std::process::id();
        let in_c = CString::new(sample_pdf.to_str().unwrap()).unwrap();

        unsafe {
            // 1. Insert blank page
            let out_blank = std::env::temp_dir().join(format!("rbd_ffi_test_blank_{pid}.pdf"));
            let out_blank_c = CString::new(out_blank.to_str().unwrap()).unwrap();
            assert_eq!(
                rbd_page_op_insert_blank(in_c.as_ptr(), out_blank_c.as_ptr(), 1, 595.0, 842.0),
                RBD_OK
            );
            assert!(out_blank.exists());

            // 2. Rotate
            let out_rot = std::env::temp_dir().join(format!("rbd_ffi_test_rot_{pid}.pdf"));
            let out_rot_c = CString::new(out_rot.to_str().unwrap()).unwrap();
            assert_eq!(
                rbd_page_op_rotate(in_c.as_ptr(), out_rot_c.as_ptr(), 90),
                RBD_OK
            );
            assert!(out_rot.exists());

            // 3. Crop
            let out_crop = std::env::temp_dir().join(format!("rbd_ffi_test_crop_{pid}.pdf"));
            let out_crop_c = CString::new(out_crop.to_str().unwrap()).unwrap();
            assert_eq!(
                rbd_page_op_crop(
                    in_c.as_ptr(),
                    out_crop_c.as_ptr(),
                    1,
                    50.0,
                    50.0,
                    300.0,
                    400.0
                ),
                RBD_OK
            );
            assert!(out_crop.exists());

            // 4. Merge
            let out_merge = std::env::temp_dir().join(format!("rbd_ffi_test_merge_{pid}.pdf"));
            let out_merge_c = CString::new(out_merge.to_str().unwrap()).unwrap();
            let json_merge =
                serde_json::to_string(&[sample_pdf.to_str().unwrap(), out_blank.to_str().unwrap()])
                    .unwrap();
            let json_merge_c = CString::new(json_merge).unwrap();
            assert_eq!(
                rbd_page_op_merge(out_merge_c.as_ptr(), json_merge_c.as_ptr()),
                RBD_OK
            );
            assert!(out_merge.exists());

            // 5. Delete pages
            let out_del = std::env::temp_dir().join(format!("rbd_ffi_test_del_{pid}.pdf"));
            let out_del_c = CString::new(out_del.to_str().unwrap()).unwrap();
            let del_json = CString::new("[1]").unwrap();
            assert_eq!(
                rbd_page_op_delete_pages(
                    out_merge_c.as_ptr(),
                    out_del_c.as_ptr(),
                    del_json.as_ptr()
                ),
                RBD_OK
            );
            assert!(out_del.exists());

            // 6. Reorder
            let out_reord = std::env::temp_dir().join(format!("rbd_ffi_test_reord_{pid}.pdf"));
            let out_reord_c = CString::new(out_reord.to_str().unwrap()).unwrap();
            let order_json = CString::new("[2, 1]").unwrap();
            assert_eq!(
                rbd_page_op_reorder(
                    out_merge_c.as_ptr(),
                    out_reord_c.as_ptr(),
                    order_json.as_ptr()
                ),
                RBD_OK
            );
            assert!(out_reord.exists());

            // 7. Split
            let out_split = std::env::temp_dir().join(format!("rbd_ffi_test_split_{pid}.pdf"));
            let out_split_c = CString::new(out_split.to_str().unwrap()).unwrap();
            assert_eq!(
                rbd_page_op_split(out_merge_c.as_ptr(), out_split_c.as_ptr(), 1, 1),
                RBD_OK
            );
            assert!(out_split.exists());

            // Clean up temporary files
            let _ = std::fs::remove_file(out_blank);
            let _ = std::fs::remove_file(out_rot);
            let _ = std::fs::remove_file(out_crop);
            let _ = std::fs::remove_file(out_merge);
            let _ = std::fs::remove_file(out_del);
            let _ = std::fs::remove_file(out_reord);
            let _ = std::fs::remove_file(out_split);
        }
    }

    #[test]
    fn test_ffi_pro_tools() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let sample_pdf =
            manifest_dir.join("../../../corpus/files/pdf-basic/pdfium-hello-world.pdf");
        let form_pdf = manifest_dir.join("../../../corpus/files/pdf-forms/pdfium-text-form.pdf");
        assert!(sample_pdf.exists());
        assert!(form_pdf.exists());

        let pid = std::process::id();
        let in_c = CString::new(sample_pdf.to_str().unwrap()).unwrap();
        let form_c = CString::new(form_pdf.to_str().unwrap()).unwrap();

        unsafe {
            // Null pointer checks
            assert_eq!(
                rbd_pro_redact_document(std::ptr::null(), std::ptr::null(), std::ptr::null()),
                RBD_ERR_NULL_PTR
            );
            assert_eq!(
                rbd_pro_compress_document(
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    0,
                    0,
                    std::ptr::null_mut(),
                    0,
                ),
                RBD_ERR_NULL_PTR
            );
            assert_eq!(
                rbd_pro_apply_watermark(
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    0.0,
                    0.0,
                    0.0,
                    0,
                    0,
                    0,
                    0,
                ),
                RBD_ERR_NULL_PTR
            );
            assert_eq!(
                rbd_pro_apply_page_numbers(
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    0.0,
                    0.0,
                ),
                RBD_ERR_NULL_PTR
            );
            assert_eq!(
                rbd_pro_apply_bates_stamp(
                    std::ptr::null(),
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    0,
                    std::ptr::null(),
                    0,
                ),
                RBD_ERR_NULL_PTR
            );
            assert_eq!(
                rbd_pro_read_form_fields(std::ptr::null(), std::ptr::null_mut(), 0),
                RBD_ERR_NULL_PTR
            );
            assert_eq!(
                rbd_pro_fill_form_fields(std::ptr::null(), std::ptr::null(), std::ptr::null(), 0),
                RBD_ERR_NULL_PTR
            );

            // 1. Redaction
            let out_red = std::env::temp_dir().join(format!("rbd_ffi_test_red_{pid}.pdf"));
            let out_red_c = CString::new(out_red.to_str().unwrap()).unwrap();
            let red_json = CString::new(
                r#"[{"page_index":0,"rects":[{"x0":50.0,"y0":650.0,"x1":300.0,"y1":750.0}],"fill_color":[0,0,0]}]"#,
            )
            .unwrap();
            assert_eq!(
                rbd_pro_redact_document(in_c.as_ptr(), out_red_c.as_ptr(), red_json.as_ptr()),
                RBD_OK
            );
            assert!(out_red.exists());

            // 2. Compression
            let out_comp = std::env::temp_dir().join(format!("rbd_ffi_test_comp_{pid}.pdf"));
            let out_comp_c = CString::new(out_comp.to_str().unwrap()).unwrap();
            let mut savings_buf = vec![0u8; 512];
            assert_eq!(
                rbd_pro_compress_document(
                    in_c.as_ptr(),
                    out_comp_c.as_ptr(),
                    1,
                    1,
                    0,
                    savings_buf.as_mut_ptr() as *mut c_char,
                    savings_buf.len(),
                ),
                RBD_OK
            );
            assert!(out_comp.exists());
            let stats_str = CStr::from_ptr(savings_buf.as_ptr() as *const c_char)
                .to_str()
                .unwrap();
            assert!(stats_str.contains("original_size"));

            // 3. Watermark
            let out_wm = std::env::temp_dir().join(format!("rbd_ffi_test_wm_{pid}.pdf"));
            let out_wm_c = CString::new(out_wm.to_str().unwrap()).unwrap();
            let wm_text = CString::new("FFI WATERMARK").unwrap();
            assert_eq!(
                rbd_pro_apply_watermark(
                    in_c.as_ptr(),
                    out_wm_c.as_ptr(),
                    wm_text.as_ptr(),
                    36.0,
                    45.0,
                    0.3,
                    200,
                    200,
                    200,
                    1,
                ),
                RBD_OK
            );
            assert!(out_wm.exists());

            // 4. Page numbers
            let out_num = std::env::temp_dir().join(format!("rbd_ffi_test_num_{pid}.pdf"));
            let out_num_c = CString::new(out_num.to_str().unwrap()).unwrap();
            let pat = CString::new("Sayfa {page} / {total}").unwrap();
            assert_eq!(
                rbd_pro_apply_page_numbers(
                    in_c.as_ptr(),
                    out_num_c.as_ptr(),
                    pat.as_ptr(),
                    4, // BottomCenter
                    10.0,
                    36.0,
                ),
                RBD_OK
            );
            assert!(out_num.exists());

            // 5. Bates stamp
            let out_bates = std::env::temp_dir().join(format!("rbd_ffi_test_bates_{pid}.pdf"));
            let out_bates_c = CString::new(out_bates.to_str().unwrap()).unwrap();
            let pfx = CString::new("FFI-").unwrap();
            let sfx = CString::new("").unwrap();
            assert_eq!(
                rbd_pro_apply_bates_stamp(
                    in_c.as_ptr(),
                    out_bates_c.as_ptr(),
                    pfx.as_ptr(),
                    10,
                    6,
                    sfx.as_ptr(),
                    5, // BottomRight
                ),
                RBD_OK
            );
            assert!(out_bates.exists());

            // 6. Read form fields
            let mut fields_buf = vec![0u8; 16384];
            assert_eq!(
                rbd_pro_read_form_fields(
                    form_c.as_ptr(),
                    fields_buf.as_mut_ptr() as *mut c_char,
                    fields_buf.len(),
                ),
                RBD_OK
            );
            let fields_json = CStr::from_ptr(fields_buf.as_ptr() as *const c_char)
                .to_str()
                .unwrap();
            assert!(fields_json.contains("name"));

            // 7. Fill form fields
            let out_form = std::env::temp_dir().join(format!("rbd_ffi_test_form_{pid}.pdf"));
            let out_form_c = CString::new(out_form.to_str().unwrap()).unwrap();
            let val_json = CString::new(r#"{"text_field": "FFI Form Test"}"#).unwrap();
            assert_eq!(
                rbd_pro_fill_form_fields(
                    form_c.as_ptr(),
                    out_form_c.as_ptr(),
                    val_json.as_ptr(),
                    1,
                ),
                RBD_OK
            );
            assert!(out_form.exists());

            let _ = std::fs::remove_file(out_red);
            let _ = std::fs::remove_file(out_comp);
            let _ = std::fs::remove_file(out_wm);
            let _ = std::fs::remove_file(out_num);
            let _ = std::fs::remove_file(out_bates);
            let _ = std::fs::remove_file(out_form);
        }
    }

    #[test]
    fn test_ffi_converter() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let sample_pdf =
            manifest_dir.join("../../../corpus/files/pdf-basic/pdfium-hello-world-2-pages.pdf");
        assert!(sample_pdf.exists());

        let pid = std::process::id();
        let temp_dir = std::env::temp_dir().join(format!("rbd_ffi_conv_{pid}"));
        let _ = std::fs::create_dir_all(&temp_dir);

        let in_c = CString::new(sample_pdf.to_str().unwrap()).unwrap();
        let out_dir_c = CString::new(temp_dir.to_str().unwrap()).unwrap();

        unsafe {
            // 1. Null pointer checks
            assert_eq!(
                rbd_convert_pdf_to_images(
                    std::ptr::null(),
                    std::ptr::null(),
                    150,
                    0,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    0,
                ),
                RBD_ERR_NULL_PTR
            );
            assert_eq!(
                rbd_convert_images_to_pdf(std::ptr::null(), 0, 10.0, std::ptr::null()),
                RBD_ERR_NULL_PTR
            );
            assert_eq!(
                rbd_convert_markdown_to_pdf(std::ptr::null(), std::ptr::null()),
                RBD_ERR_NULL_PTR
            );

            // 2. PDF to images (page 2 only, PNG)
            let mut paths_buf = vec![0u8; 4096];
            let pr_json = CString::new("[2]").unwrap();
            assert_eq!(
                rbd_convert_pdf_to_images(
                    in_c.as_ptr(),
                    out_dir_c.as_ptr(),
                    100,
                    0, // PNG
                    pr_json.as_ptr(),
                    paths_buf.as_mut_ptr() as *mut c_char,
                    paths_buf.len(),
                ),
                RBD_OK
            );
            let paths_str = CStr::from_ptr(paths_buf.as_ptr() as *const c_char)
                .to_str()
                .unwrap();
            assert!(paths_str.contains("page_2.png"));

            // 3. Images to PDF (round-trip the image generated in step 2)
            let out_img_pdf = temp_dir.join("from_images.pdf");
            let out_img_pdf_c = CString::new(out_img_pdf.to_str().unwrap()).unwrap();

            assert_eq!(
                rbd_convert_images_to_pdf(
                    paths_buf.as_ptr() as *const c_char,
                    0, // A4
                    20.0,
                    out_img_pdf_c.as_ptr(),
                ),
                RBD_OK
            );
            assert!(out_img_pdf.exists());

            // 4. Markdown to PDF
            let md_text =
                CString::new("# C-ABI Header\n\n- Bullet 1\n- Bullet 2\n\nParagraph text.")
                    .unwrap();
            let out_md_pdf = temp_dir.join("from_md.pdf");
            let out_md_pdf_c = CString::new(out_md_pdf.to_str().unwrap()).unwrap();

            assert_eq!(
                rbd_convert_markdown_to_pdf(md_text.as_ptr(), out_md_pdf_c.as_ptr()),
                RBD_OK
            );
            assert!(out_md_pdf.exists());

            let _ = std::fs::remove_dir_all(temp_dir);
        }
    }

    #[test]
    fn test_ffi_office() {
        let pid = std::process::id();
        let temp_dir = std::env::temp_dir().join(format!("rbd_ffi_office_{pid}"));
        let _ = std::fs::create_dir_all(&temp_dir);

        let out_docx = temp_dir.join("test_output.docx");
        let out_docx_c = CString::new(out_docx.to_str().unwrap()).unwrap();
        let text_content = CString::new("RBD OFİS Ofis C-ABI\nİkinci satır.").unwrap();

        unsafe {
            // Null pointer checks
            assert_eq!(
                rbd_office_extract_text(std::ptr::null(), std::ptr::null_mut(), 0),
                RBD_ERR_NULL_PTR
            );
            assert_eq!(
                rbd_office_inspect(std::ptr::null(), std::ptr::null_mut(), 0),
                RBD_ERR_NULL_PTR
            );
            assert_eq!(
                rbd_office_create_docx(std::ptr::null(), std::ptr::null()),
                RBD_ERR_NULL_PTR
            );

            // 1. Create DOCX
            assert_eq!(
                rbd_office_create_docx(text_content.as_ptr(), out_docx_c.as_ptr()),
                RBD_OK
            );
            assert!(out_docx.exists());

            // 2. Extract Text
            let mut text_buf = vec![0u8; 4096];
            assert_eq!(
                rbd_office_extract_text(
                    out_docx_c.as_ptr(),
                    text_buf.as_mut_ptr() as *mut c_char,
                    text_buf.len(),
                ),
                RBD_OK
            );
            let extracted = CStr::from_ptr(text_buf.as_ptr() as *const c_char)
                .to_str()
                .unwrap();
            assert!(extracted.contains("RBD OFİS Ofis C-ABI"));
            assert!(extracted.contains("İkinci satır."));

            // 3. Inspect
            let mut json_buf = vec![0u8; 4096];
            assert_eq!(
                rbd_office_inspect(
                    out_docx_c.as_ptr(),
                    json_buf.as_mut_ptr() as *mut c_char,
                    json_buf.len(),
                ),
                RBD_OK
            );
            let json_str = CStr::from_ptr(json_buf.as_ptr() as *const c_char)
                .to_str()
                .unwrap();
            assert!(json_str.contains("\"doc_type\":\"DOCX\""));
            assert!(json_str.contains("\"word_count\":6"));

            let _ = std::fs::remove_dir_all(temp_dir);
        }
    }

    #[test]
    fn test_ffi_ai_summarize_and_ask() {
        let sample_pdf = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../corpus/files/pdf-basic/sample.pdf");
        if !sample_pdf.exists() {
            return;
        }

        let pid = std::process::id();
        let path_c = CString::new(sample_pdf.to_str().unwrap()).unwrap();

        unsafe {
            let mut engine = std::ptr::null_mut();
            assert_eq!(rbd_engine_init(&mut engine), RBD_OK);
            assert!(!engine.is_null());

            let mut doc = std::ptr::null_mut();
            assert_eq!(
                rbd_document_open_file(engine, path_c.as_ptr(), &mut doc),
                RBD_OK
            );
            assert!(!doc.is_null());

            // 1. AI Summarize
            let mut sum_buf = vec![0u8; 16384];
            assert_eq!(
                rbd_ai_summarize_document(
                    doc,
                    3,
                    sum_buf.as_mut_ptr() as *mut c_char,
                    sum_buf.len()
                ),
                RBD_OK
            );
            let sum_str = CStr::from_ptr(sum_buf.as_ptr() as *const c_char)
                .to_str()
                .unwrap();
            assert!(sum_str.contains("summary_text"));

            // 2. AI Ask
            let query_c = CString::new("sample document query").unwrap();
            let mut ask_buf = vec![0u8; 16384];
            assert_eq!(
                rbd_ai_ask_document(
                    doc,
                    query_c.as_ptr(),
                    ask_buf.as_mut_ptr() as *mut c_char,
                    ask_buf.len()
                ),
                RBD_OK
            );
            let ask_str = CStr::from_ptr(ask_buf.as_ptr() as *const c_char)
                .to_str()
                .unwrap();
            assert!(ask_str.contains("query"));

            // 3. Reflow Paragraphs
            let mut reflow_buf = vec![0u8; 16384];
            assert_eq!(
                rbd_document_reflow_page_text(
                    doc,
                    0,
                    reflow_buf.as_mut_ptr() as *mut c_char,
                    reflow_buf.len()
                ),
                RBD_OK
            );
            let reflow_str = CStr::from_ptr(reflow_buf.as_ptr() as *const c_char)
                .to_str()
                .unwrap();
            assert!(reflow_str.starts_with('['));

            // 4. Replace paragraph text
            let out_pdf = std::env::temp_dir().join(format!("rbd_ffi_reflow_{pid}.pdf"));
            let out_c = CString::new(out_pdf.to_str().unwrap()).unwrap();
            let new_text_c = CString::new("Reflowed text for testing").unwrap();

            assert_eq!(
                rbd_document_replace_paragraph_text(
                    doc,
                    path_c.as_ptr(),
                    0,
                    72.0,
                    600.0,
                    400.0,
                    650.0,
                    new_text_c.as_ptr(),
                    11.0,
                    out_c.as_ptr(),
                ),
                RBD_OK
            );
            assert!(out_pdf.exists());
            let _ = std::fs::remove_file(out_pdf);

            rbd_document_free(doc);
            rbd_engine_free(engine);
        }
    }

    #[test]
    fn test_render_tile_invalid_stride_regression() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let sample_pdf =
            manifest_dir.join("../../../corpus/files/pdf-basic/pdfium-hello-world.pdf");
        assert!(sample_pdf.exists());

        unsafe {
            let mut engine = std::ptr::null_mut();
            assert_eq!(rbd_engine_init(&mut engine), RBD_OK);

            let path_c = CString::new(sample_pdf.to_str().unwrap()).unwrap();
            let mut doc = std::ptr::null_mut();
            assert_eq!(
                rbd_document_open_file(engine, path_c.as_ptr(), &mut doc),
                RBD_OK
            );

            let mut dummy_buf = [0u8; 1024];

            // Regression test: buffer_stride = 0 must fail with RBD_ERR_INVALID_PARAM
            let res_zero = rbd_render_tile_into_buffer(
                doc,
                0,
                0,
                0,
                256,
                256,
                1.0,
                dummy_buf.as_mut_ptr(),
                0,
                0,
            );
            assert_eq!(res_zero, RBD_ERR_INVALID_PARAM);

            // Regression test: negative buffer_stride must fail with RBD_ERR_INVALID_PARAM (not cast to huge usize / UB)
            let res_neg = rbd_render_tile_into_buffer(
                doc,
                0,
                0,
                0,
                256,
                256,
                1.0,
                dummy_buf.as_mut_ptr(),
                -1024,
                0,
            );
            assert_eq!(res_neg, RBD_ERR_INVALID_PARAM);

            rbd_document_free(doc);
            rbd_engine_free(engine);
        }
    }

    #[test]
    fn test_ffi_scanner() {
        unsafe {
            // Null pointer checks
            assert_eq!(
                rbd_scan_compute_luminance(255, 255, 255, std::ptr::null_mut()),
                RBD_ERR_NULL_PTR
            );
            assert_eq!(
                rbd_scan_calculate_otsu_threshold(std::ptr::null(), 100, std::ptr::null_mut()),
                RBD_ERR_NULL_PTR
            );
            assert_eq!(
                rbd_scan_to_grayscale(std::ptr::null_mut(), 10, 10, 40),
                RBD_ERR_NULL_PTR
            );
            assert_eq!(
                rbd_scan_to_binarized(std::ptr::null_mut(), 10, 10, 40, std::ptr::null_mut()),
                RBD_ERR_NULL_PTR
            );
            assert_eq!(
                rbd_scan_generate_pdf(std::ptr::null(), std::ptr::null()),
                RBD_ERR_NULL_PTR
            );

            // 1. Luminance computation
            let mut lum = 0u8;
            assert_eq!(rbd_scan_compute_luminance(255, 255, 255, &mut lum), RBD_OK);
            assert_eq!(lum, 255);
            assert_eq!(rbd_scan_compute_luminance(255, 0, 0, &mut lum), RBD_OK);
            assert_eq!(lum, 76);
            assert_eq!(rbd_scan_compute_luminance(0, 255, 0, &mut lum), RBD_OK);
            assert_eq!(lum, 149);
            assert_eq!(rbd_scan_compute_luminance(0, 0, 255, &mut lum), RBD_OK);
            assert_eq!(lum, 29);

            // 2. Otsu threshold calculation
            let mut histogram = [0u32; 256];
            histogram[40] = 500;
            histogram[220] = 1000;
            let mut threshold = 0u8;
            assert_eq!(
                rbd_scan_calculate_otsu_threshold(histogram.as_ptr(), 1500, &mut threshold),
                RBD_OK
            );
            assert!(threshold > 40 && threshold < 220);

            // 3. Grayscale conversion on 2x2 RGBA buffer
            let mut buf = vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 0, 255, 255, 255, 255, 255,
            ];
            assert_eq!(rbd_scan_to_grayscale(buf.as_mut_ptr(), 2, 2, 8), RBD_OK);
            assert_eq!(buf[0..4], [76, 76, 76, 255]);
            assert_eq!(buf[4..8], [149, 149, 149, 255]);
            assert_eq!(buf[8..12], [0, 0, 0, 255]);
            assert_eq!(buf[12..16], [255, 255, 255, 255]);

            // 4. Binarization on 2x2 RGBA buffer
            let mut bin_buf = vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 0, 255, 255, 255, 255, 255,
            ];
            let mut bin_threshold = 0u8;
            assert_eq!(
                rbd_scan_to_binarized(bin_buf.as_mut_ptr(), 2, 2, 8, &mut bin_threshold),
                RBD_OK
            );
            assert!(bin_threshold > 0);
            for chunk in bin_buf.as_chunks::<4>().0 {
                assert!(chunk[0] == 0 || chunk[0] == 255);
                assert_eq!(chunk[0], chunk[1]);
                assert_eq!(chunk[1], chunk[2]);
                assert_eq!(chunk[3], 255);
            }

            // 5. Invalid arguments
            assert_eq!(
                rbd_scan_to_grayscale(buf.as_mut_ptr(), 0, 2, 8),
                RBD_ERR_INVALID_PARAM
            );
            assert_eq!(
                rbd_scan_to_grayscale(buf.as_mut_ptr(), 2, 2, 4),
                RBD_ERR_INVALID_PARAM
            );
            assert_eq!(
                rbd_scan_to_binarized(buf.as_mut_ptr(), 2, 0, 8, std::ptr::null_mut()),
                RBD_ERR_INVALID_PARAM
            );
            assert_eq!(
                rbd_scan_to_binarized(buf.as_mut_ptr(), 2, 2, 4, std::ptr::null_mut()),
                RBD_ERR_INVALID_PARAM
            );

            // 6. PDF generation with empty paths
            let empty_json = CString::new("[]").unwrap();
            let out_pdf = CString::new("out.pdf").unwrap();
            assert_eq!(
                rbd_scan_generate_pdf(empty_json.as_ptr(), out_pdf.as_ptr()),
                RBD_ERR_INVALID_PARAM
            );
        }
    }

    #[test]
    fn test_regression_item1_rbd_document_handle_rwlock_concurrency() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let pdf_path = manifest_dir.join("../../../corpus/files/pdf-basic/pdfium-hello-world.pdf");
        assert!(pdf_path.exists(), "Test PDF must exist");

        let c_path = CString::new(pdf_path.to_str().unwrap()).unwrap();

        unsafe {
            let mut engine: *mut c_void = ptr::null_mut();
            assert_eq!(rbd_engine_init(&mut engine), RBD_OK);
            let mut doc: *mut c_void = ptr::null_mut();
            assert_eq!(
                rbd_document_open_file(engine, c_path.as_ptr(), &mut doc),
                RBD_OK
            );

            // Transmit raw pointer as usize across threads
            let doc_addr = doc as usize;

            let handle_read1 = std::thread::spawn(move || {
                let doc = doc_addr as *mut c_void;
                for _ in 0..50 {
                    let mut count = 0i32;
                    let res = rbd_document_get_page_count(doc, &mut count);
                    assert_eq!(res, RBD_OK);
                    assert!(count >= 1);

                    let mut w = 0.0f64;
                    let mut h = 0.0f64;
                    let res_dim = rbd_document_get_page_size(doc, 0, &mut w, &mut h);
                    assert_eq!(res_dim, RBD_OK);
                }
            });

            let handle_read2 = std::thread::spawn(move || {
                let doc = doc_addr as *mut c_void;
                let mut tile_buf = vec![0u8; 128 * 128 * 4];
                for _ in 0..20 {
                    let res = rbd_render_tile_into_buffer(
                        doc,
                        0,
                        0,
                        0,
                        128,
                        128,
                        1.0,
                        tile_buf.as_mut_ptr(),
                        128 * 4,
                        1,
                    );
                    assert_eq!(res, RBD_OK);
                }
            });

            let handle_write = std::thread::spawn(move || {
                let doc = doc_addr as *mut c_void;
                let annot_json = CString::new(
                    r#"{
                    "id": 9999,
                    "page_index": 0,
                    "kind": "Highlight",
                    "rect": { "x0": 50.0, "y0": 50.0, "x1": 150.0, "y1": 70.0 },
                    "color_rgba": [255, 255, 0, 128],
                    "contents": "Thread test"
                }"#,
                )
                .unwrap();

                for _ in 0..10 {
                    let res_add = rbd_document_add_annotation_json(doc, annot_json.as_ptr());
                    assert_eq!(res_add, RBD_OK);

                    let mut undone = 0i32;
                    let res_undo = rbd_document_undo(doc, &mut undone);
                    assert_eq!(res_undo, RBD_OK);
                }
            });

            let handle_extract = std::thread::spawn(move || {
                let doc = doc_addr as *mut c_void;
                for _ in 0..30 {
                    let mut text_ptr: *mut c_char = ptr::null_mut();
                    let res = rbd_document_extract_page_text(doc, 0, &mut text_ptr);
                    assert_eq!(res, RBD_OK);
                    assert!(!text_ptr.is_null());
                    rbd_free_string(text_ptr);
                }
            });

            handle_read1.join().expect("Reader 1 failed");
            handle_read2.join().expect("Reader 2 failed");
            handle_write.join().expect("Writer failed");
            handle_extract.join().expect("Extractor failed");

            rbd_document_free(doc);
            rbd_engine_free(engine);
        }
    }

    #[test]
    fn test_regression_item3_buffer_checked_arithmetic_overflow() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let pdf_path = manifest_dir.join("../../../corpus/files/pdf-basic/pdfium-hello-world.pdf");
        let c_path = CString::new(pdf_path.to_str().unwrap()).unwrap();

        unsafe {
            let mut engine: *mut c_void = ptr::null_mut();
            assert_eq!(rbd_engine_init(&mut engine), RBD_OK);
            let mut doc: *mut c_void = ptr::null_mut();
            assert_eq!(
                rbd_document_open_file(engine, c_path.as_ptr(), &mut doc),
                RBD_OK
            );

            let mut dummy_buf = [0u8; 128];

            // 1. tile_width / tile_height causing multiplication overflow
            let res = rbd_render_tile_into_buffer(
                doc,
                0,
                0,
                0,
                i32::MAX,
                i32::MAX,
                1.0,
                dummy_buf.as_mut_ptr(),
                128,
                0,
            );
            assert_eq!(res, RBD_ERR_INVALID_PARAM);

            // 2. buffer_stride overflow
            let res_stride = rbd_render_tile_into_buffer(
                doc,
                0,
                0,
                0,
                64,
                64,
                1.0,
                dummy_buf.as_mut_ptr(),
                i32::MAX,
                0,
            );
            assert_eq!(res_stride, RBD_ERR_INVALID_PARAM);

            // 3. rbd_session_render_page with overflowing dimensions
            let mut session: *mut RbdSession = ptr::null_mut();
            assert_eq!(
                rbd_session_open_file(c_path.as_ptr(), ptr::null(), &mut session),
                RBD_OK
            );

            let res_sess_overflow = rbd_session_render_page(
                session,
                0,
                u32::MAX,
                u32::MAX,
                dummy_buf.as_mut_ptr(),
                dummy_buf.len(),
            );
            assert_eq!(res_sess_overflow, RBD_ERR_INVALID_PARAM);

            // 4. rbd_session_render_tile_into_buffer with overflowing dimensions
            let res_tile_overflow = rbd_session_render_tile_into_buffer(
                session,
                0,
                0,
                0,
                u32::MAX,
                u32::MAX,
                72.0,
                dummy_buf.as_mut_ptr(),
                dummy_buf.len(),
            );
            assert_eq!(res_tile_overflow, RBD_ERR_INVALID_PARAM);

            rbd_session_close(session);
            rbd_document_free(doc);
            rbd_engine_free(engine);
        }
    }
}
