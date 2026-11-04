use rbd_core::{CoreError, DocumentSession};
use rbd_engine::{EngineError, PdfEngine, PixelFormat, create_default_engine};
use std::cell::RefCell;
use std::ffi::{CStr, CString, c_void};
use std::os::raw::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::slice;
use std::sync::{Arc, OnceLock};

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
    pub session: DocumentSession,
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

        let doc_handle = Box::new(RbdDocumentHandle { session });
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

        let doc_handle = Box::new(RbdDocumentHandle { session });
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

        let row_bytes = (tile_width as usize) * 4;
        if (buffer_stride as usize) < row_bytes {
            set_last_error("buffer_stride is smaller than tile_width * 4");
            return Err(RBD_ERR_INVALID_PARAM);
        }

        let format = PixelFormat::try_from(pixel_format).map_err(|e| map_engine_error(&e))?;
        let scale_dpi = (zoom * 72.0) as f32;
        let total_bytes = (buffer_stride as usize) * (tile_height as usize);

        let slice = unsafe { slice::from_raw_parts_mut(destination_buffer, total_bytes) };
        let doc_handle = unsafe { &*(doc as *const RbdDocumentHandle) };

        doc_handle
            .session
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
        let required = (width as usize) * (height as usize) * 4;
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
        let required = (tile_w as usize) * (tile_h as usize) * 4;
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
}
