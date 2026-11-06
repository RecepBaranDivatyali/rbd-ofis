//! Android JNI (Java Native Interface) bridge for RBD OFİS core.
//! Matches `com.rbdofis.app.core.RbdNativeBridge` in apps/android.
//!
//! All entrypoints are wrapped in `catch_unwind` to ensure panics
//! never escape across the JNI boundary into the ART runtime VM.

#![allow(clippy::missing_safety_doc)]

use crate::{
    RBD_ERR_BUFFER_TOO_SMALL, RBD_ERR_INVALID_PARAM, RBD_ERR_NULL_PTR, RBD_ERR_PAGE_OUT_OF_RANGE,
    RBD_ERR_PANIC, RBD_OK, rbd_convert_images_to_pdf, rbd_convert_markdown_to_pdf,
    rbd_convert_pdf_to_images, rbd_document_add_annotation_json, rbd_document_extract_page_text,
    rbd_document_free, rbd_document_get_annotations_json, rbd_document_get_page_count,
    rbd_document_get_page_size, rbd_document_open_file, rbd_document_open_memory,
    rbd_document_redo, rbd_document_remove_annotation, rbd_document_save_annotations,
    rbd_document_search_json, rbd_document_undo, rbd_engine_free, rbd_engine_init, rbd_free_string,
    rbd_office_create_docx, rbd_office_extract_text, rbd_office_inspect, rbd_page_op_crop,
    rbd_page_op_decrypt, rbd_page_op_delete_pages, rbd_page_op_insert_blank, rbd_page_op_merge,
    rbd_page_op_reorder, rbd_page_op_rotate, rbd_page_op_split, rbd_pro_apply_bates_stamp,
    rbd_pro_apply_page_numbers, rbd_pro_apply_watermark, rbd_pro_compress_document,
    rbd_pro_fill_form_fields, rbd_pro_read_form_fields, rbd_pro_redact_document,
    rbd_render_tile_into_buffer,
};
use jni::JNIEnv;
use jni::objects::{JByteArray, JByteBuffer, JClass, JString};
use jni::sys::{
    JNI_FALSE, JNI_TRUE, jboolean, jbyteArray, jdouble, jdoubleArray, jfloat, jint, jlong, jobject,
    jstring,
};
use std::ffi::{CStr, CString, c_char, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;

/// Helper to safely convert a Java `jstring` into a Rust `CString`.
unsafe fn jstring_to_cstring(env: &mut JNIEnv, jstr: jstring) -> Option<CString> {
    if jstr.is_null() {
        return None;
    }
    let j_obj = unsafe { JString::from_raw(jstr) };
    let rust_str: String = env.get_string(&j_obj).ok()?.into();
    CString::new(rust_str).ok()
}

/// Helper executing code inside `catch_unwind` and returning a default value on panic.
fn jni_boundary<T, F>(default: T, f: F) -> T
where
    F: FnOnce() -> T,
{
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(val) => val,
        Err(_) => {
            eprintln!("[RBD-JNI] Caught panic at JNI boundary! Preventing ART VM abort.");
            default
        }
    }
}

/// Initialize the native RBD engine instance.
/// Returns pointer handle as `jlong`, or `0` on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeEngineInit(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    jni_boundary(0, || {
        let mut engine_ptr: *mut c_void = ptr::null_mut();
        let status = unsafe { rbd_engine_init(&mut engine_ptr) };
        if status == RBD_OK && !engine_ptr.is_null() {
            engine_ptr as jlong
        } else {
            0
        }
    })
}

/// Free a native RBD engine instance.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeEngineFree(
    _env: JNIEnv,
    _class: JClass,
    engine_handle: jlong,
) {
    jni_boundary((), || {
        if engine_handle != 0 {
            unsafe {
                rbd_engine_free(engine_handle as *mut c_void);
            }
        }
    });
}

/// Open a PDF document from a filesystem path.
/// Returns document handle as `jlong`, or `0` on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeDocumentOpenFile(
    mut env: JNIEnv,
    _class: JClass,
    engine_handle: jlong,
    path: jstring,
) -> jlong {
    jni_boundary(0, || {
        if engine_handle == 0 || path.is_null() {
            return 0;
        }

        let c_path = match unsafe { jstring_to_cstring(&mut env, path) } {
            Some(p) => p,
            None => return 0,
        };

        let mut doc_ptr: *mut c_void = ptr::null_mut();
        let status = unsafe {
            rbd_document_open_file(engine_handle as *mut c_void, c_path.as_ptr(), &mut doc_ptr)
        };

        if status == RBD_OK && !doc_ptr.is_null() {
            doc_ptr as jlong
        } else {
            0
        }
    })
}

/// Open a PDF document from an in-memory byte array.
/// Returns document handle as `jlong`, or `0` on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeDocumentOpenMemory(
    env: JNIEnv,
    _class: JClass,
    engine_handle: jlong,
    bytes: jbyteArray,
) -> jlong {
    jni_boundary(0, || {
        if engine_handle == 0 || bytes.is_null() {
            return 0;
        }

        let j_barr = unsafe { JByteArray::from_raw(bytes) };
        let data: Vec<u8> = match env.convert_byte_array(&j_barr) {
            Ok(d) => d,
            Err(_) => return 0,
        };

        if data.is_empty() {
            return 0;
        }

        let mut doc_ptr: *mut c_void = ptr::null_mut();
        let status = unsafe {
            rbd_document_open_memory(
                engine_handle as *mut c_void,
                data.as_ptr(),
                data.len(),
                &mut doc_ptr,
            )
        };

        if status == RBD_OK && !doc_ptr.is_null() {
            doc_ptr as jlong
        } else {
            0
        }
    })
}

/// Free an opened PDF document instance.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeDocumentFree(
    _env: JNIEnv,
    _class: JClass,
    doc_handle: jlong,
) {
    jni_boundary((), || {
        if doc_handle != 0 {
            unsafe {
                rbd_document_free(doc_handle as *mut c_void);
            }
        }
    });
}

/// Query the total page count of the document.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeDocumentGetPageCount(
    _env: JNIEnv,
    _class: JClass,
    doc_handle: jlong,
) -> jint {
    jni_boundary(RBD_ERR_PANIC, || {
        if doc_handle == 0 {
            return RBD_ERR_NULL_PTR;
        }

        let mut count: i32 = 0;
        let status = unsafe { rbd_document_get_page_count(doc_handle as *mut c_void, &mut count) };

        if status == RBD_OK { count } else { status }
    })
}

/// Query page dimensions (width, height in points) for a given page index.
/// Returns a double array `[width, height]` or `null` on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeDocumentGetPageSize(
    env: JNIEnv,
    _class: JClass,
    doc_handle: jlong,
    page_index: jint,
) -> jdoubleArray {
    jni_boundary(ptr::null_mut(), || {
        if doc_handle == 0 || page_index < 0 {
            return ptr::null_mut();
        }

        let mut width: f64 = 0.0;
        let mut height: f64 = 0.0;
        let status = unsafe {
            rbd_document_get_page_size(
                doc_handle as *mut c_void,
                page_index,
                &mut width,
                &mut height,
            )
        };

        if status != RBD_OK {
            return ptr::null_mut();
        }

        let arr = match env.new_double_array(2) {
            Ok(a) => a,
            Err(_) => return ptr::null_mut(),
        };

        if env
            .set_double_array_region(&arr, 0, &[width, height])
            .is_err()
        {
            return ptr::null_mut();
        }

        arr.into_raw()
    })
}

/// Render a single tile directly into a `DirectByteBuffer` with zero memory copies.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeRenderTileIntoBuffer(
    env: JNIEnv,
    _class: JClass,
    doc_handle: jlong,
    page_index: jint,
    tile_x: jint,
    tile_y: jint,
    tile_w: jint,
    tile_h: jint,
    zoom: jdouble,
    direct_byte_buffer: jobject,
    stride: jint,
    pixel_format: jint,
) -> jint {
    jni_boundary(RBD_ERR_PANIC, || {
        if doc_handle == 0 || direct_byte_buffer.is_null() {
            return RBD_ERR_NULL_PTR;
        }

        let jbuf = unsafe { JByteBuffer::from_raw(direct_byte_buffer) };
        let buf_ptr = match env.get_direct_buffer_address(&jbuf) {
            Ok(p) => p,
            Err(_) => return RBD_ERR_INVALID_PARAM,
        };

        let capacity = match env.get_direct_buffer_capacity(&jbuf) {
            Ok(c) => c,
            Err(_) => return RBD_ERR_INVALID_PARAM,
        };

        let required = (stride as usize) * (tile_h as usize);
        if capacity < required {
            return RBD_ERR_BUFFER_TOO_SMALL;
        }

        unsafe {
            rbd_render_tile_into_buffer(
                doc_handle as *mut c_void,
                page_index,
                tile_x,
                tile_y,
                tile_w,
                tile_h,
                zoom,
                buf_ptr,
                stride,
                pixel_format,
            )
        }
    })
}

/// Extract UTF-8 plain text from a page.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeDocumentExtractPageText(
    env: JNIEnv,
    _class: JClass,
    doc_handle: jlong,
    page_index: jint,
) -> jstring {
    jni_boundary(ptr::null_mut(), || {
        if doc_handle == 0 || page_index < 0 {
            return ptr::null_mut();
        }

        let mut text_ptr: *mut c_char = ptr::null_mut();
        let status = unsafe {
            rbd_document_extract_page_text(
                doc_handle as *mut c_void,
                page_index as u32,
                &mut text_ptr,
            )
        };

        if status != RBD_OK || text_ptr.is_null() {
            return ptr::null_mut();
        }

        let text_str = unsafe { CStr::from_ptr(text_ptr) }
            .to_str()
            .unwrap_or_default();
        let js = match env.new_string(text_str) {
            Ok(s) => s.into_raw(),
            Err(_) => ptr::null_mut(),
        };

        unsafe {
            rbd_free_string(text_ptr);
        }

        js
    })
}

/// Search for query text across the document and return hits serialized as JSON.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeDocumentSearchJson(
    mut env: JNIEnv,
    _class: JClass,
    doc_handle: jlong,
    query: jstring,
    case_sensitive: jboolean,
) -> jstring {
    jni_boundary(ptr::null_mut(), || {
        if doc_handle == 0 || query.is_null() {
            return ptr::null_mut();
        }

        let query_c = match unsafe { jstring_to_cstring(&mut env, query) } {
            Some(q) => q,
            None => return ptr::null_mut(),
        };

        let mut json_ptr: *mut c_char = ptr::null_mut();
        let status = unsafe {
            rbd_document_search_json(
                doc_handle as *mut c_void,
                query_c.as_ptr(),
                if case_sensitive != JNI_FALSE { 1 } else { 0 },
                &mut json_ptr,
            )
        };

        if status != RBD_OK || json_ptr.is_null() {
            return ptr::null_mut();
        }

        let json_str = unsafe { CStr::from_ptr(json_ptr) }
            .to_str()
            .unwrap_or_default();
        let js = match env.new_string(json_str) {
            Ok(s) => s.into_raw(),
            Err(_) => ptr::null_mut(),
        };

        unsafe {
            rbd_free_string(json_ptr);
        }

        js
    })
}

/// Add an annotation from JSON with undo/redo support.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeDocumentAddAnnotationJson(
    mut env: JNIEnv,
    _class: JClass,
    doc_handle: jlong,
    annot_json: jstring,
) -> jint {
    jni_boundary(RBD_ERR_PANIC, || {
        if doc_handle == 0 || annot_json.is_null() {
            return RBD_ERR_NULL_PTR;
        }

        let annot_c = match unsafe { jstring_to_cstring(&mut env, annot_json) } {
            Some(a) => a,
            None => return RBD_ERR_INVALID_PARAM,
        };

        unsafe { rbd_document_add_annotation_json(doc_handle as *mut c_void, annot_c.as_ptr()) }
    })
}

/// Remove an annotation by ID with undo/redo support.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeDocumentRemoveAnnotation(
    _env: JNIEnv,
    _class: JClass,
    doc_handle: jlong,
    annotation_id: jlong,
) -> jint {
    jni_boundary(RBD_ERR_PANIC, || {
        if doc_handle == 0 {
            return RBD_ERR_NULL_PTR;
        }

        unsafe { rbd_document_remove_annotation(doc_handle as *mut c_void, annotation_id as u64) }
    })
}

/// Retrieve all annotations on a page serialized as JSON.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeDocumentGetAnnotationsJson(
    env: JNIEnv,
    _class: JClass,
    doc_handle: jlong,
    page_index: jint,
) -> jstring {
    jni_boundary(ptr::null_mut(), || {
        if doc_handle == 0 || page_index < 0 {
            return ptr::null_mut();
        }

        let mut json_ptr: *mut c_char = ptr::null_mut();
        let status = unsafe {
            rbd_document_get_annotations_json(
                doc_handle as *mut c_void,
                page_index as u32,
                &mut json_ptr,
            )
        };

        if status != RBD_OK || json_ptr.is_null() {
            return ptr::null_mut();
        }

        let json_str = unsafe { CStr::from_ptr(json_ptr) }
            .to_str()
            .unwrap_or_default();
        let js = match env.new_string(json_str) {
            Ok(s) => s.into_raw(),
            Err(_) => ptr::null_mut(),
        };

        unsafe {
            rbd_free_string(json_ptr);
        }

        js
    })
}

/// Undo the last annotation action. Returns `true` if an action was undone.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeDocumentUndo(
    _env: JNIEnv,
    _class: JClass,
    doc_handle: jlong,
) -> jboolean {
    jni_boundary(JNI_FALSE, || {
        if doc_handle == 0 {
            return JNI_FALSE;
        }

        let mut undone: i32 = 0;
        let status = unsafe { rbd_document_undo(doc_handle as *mut c_void, &mut undone) };

        if status == RBD_OK && undone != 0 {
            JNI_TRUE
        } else {
            JNI_FALSE
        }
    })
}

/// Redo the last undone annotation action. Returns `true` if an action was redone.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeDocumentRedo(
    _env: JNIEnv,
    _class: JClass,
    doc_handle: jlong,
) -> jboolean {
    jni_boundary(JNI_FALSE, || {
        if doc_handle == 0 {
            return JNI_FALSE;
        }

        let mut redone: i32 = 0;
        let status = unsafe { rbd_document_redo(doc_handle as *mut c_void, &mut redone) };

        if status == RBD_OK && redone != 0 {
            JNI_TRUE
        } else {
            JNI_FALSE
        }
    })
}

/// Save annotations to the specified PDF file path.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeDocumentSaveAnnotations(
    mut env: JNIEnv,
    _class: JClass,
    doc_handle: jlong,
    out_path: jstring,
) -> jint {
    jni_boundary(RBD_ERR_PANIC, || {
        if doc_handle == 0 || out_path.is_null() {
            return RBD_ERR_NULL_PTR;
        }

        let out_c = match unsafe { jstring_to_cstring(&mut env, out_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };

        unsafe { rbd_document_save_annotations(doc_handle as *mut c_void, out_c.as_ptr()) }
    })
}

/// Rotate all pages in a PDF document by `degrees` (multiple of 90).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativePageOpRotate(
    mut env: JNIEnv,
    _class: JClass,
    in_path: jstring,
    out_path: jstring,
    degrees: jint,
) -> jint {
    jni_boundary(RBD_ERR_PANIC, || {
        if in_path.is_null() || out_path.is_null() {
            return RBD_ERR_NULL_PTR;
        }

        let in_c = match unsafe { jstring_to_cstring(&mut env, in_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };
        let out_c = match unsafe { jstring_to_cstring(&mut env, out_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };

        unsafe { rbd_page_op_rotate(in_c.as_ptr(), out_c.as_ptr(), degrees) }
    })
}

/// Delete pages specified in `pages_json` (1-based JSON array of page numbers).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativePageOpDeletePages(
    mut env: JNIEnv,
    _class: JClass,
    in_path: jstring,
    out_path: jstring,
    pages_json: jstring,
) -> jint {
    jni_boundary(RBD_ERR_PANIC, || {
        if in_path.is_null() || out_path.is_null() || pages_json.is_null() {
            return RBD_ERR_NULL_PTR;
        }

        let in_c = match unsafe { jstring_to_cstring(&mut env, in_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };
        let out_c = match unsafe { jstring_to_cstring(&mut env, out_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };
        let json_c = match unsafe { jstring_to_cstring(&mut env, pages_json) } {
            Some(j) => j,
            None => return RBD_ERR_INVALID_PARAM,
        };

        unsafe { rbd_page_op_delete_pages(in_c.as_ptr(), out_c.as_ptr(), json_c.as_ptr()) }
    })
}

/// Merge multiple PDF documents into a single output file.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativePageOpMerge(
    mut env: JNIEnv,
    _class: JClass,
    out_path: jstring,
    in_paths_json: jstring,
) -> jint {
    jni_boundary(RBD_ERR_PANIC, || {
        if out_path.is_null() || in_paths_json.is_null() {
            return RBD_ERR_NULL_PTR;
        }

        let out_c = match unsafe { jstring_to_cstring(&mut env, out_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };
        let json_c = match unsafe { jstring_to_cstring(&mut env, in_paths_json) } {
            Some(j) => j,
            None => return RBD_ERR_INVALID_PARAM,
        };

        unsafe { rbd_page_op_merge(out_c.as_ptr(), json_c.as_ptr()) }
    })
}

/// Extract a 1-based page range `[start_page, end_page]` into an output PDF.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativePageOpSplit(
    mut env: JNIEnv,
    _class: JClass,
    in_path: jstring,
    out_path: jstring,
    start_page: jint,
    end_page: jint,
) -> jint {
    jni_boundary(RBD_ERR_PANIC, || {
        if in_path.is_null() || out_path.is_null() {
            return RBD_ERR_NULL_PTR;
        }

        if start_page < 1 || end_page < start_page {
            return RBD_ERR_PAGE_OUT_OF_RANGE;
        }

        let in_c = match unsafe { jstring_to_cstring(&mut env, in_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };
        let out_c = match unsafe { jstring_to_cstring(&mut env, out_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };

        unsafe {
            rbd_page_op_split(
                in_c.as_ptr(),
                out_c.as_ptr(),
                start_page as u32,
                end_page as u32,
            )
        }
    })
}

/// Reorder pages according to a 1-based JSON array.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativePageOpReorder(
    mut env: JNIEnv,
    _class: JClass,
    in_path: jstring,
    out_path: jstring,
    order_json: jstring,
) -> jint {
    jni_boundary(RBD_ERR_PANIC, || {
        if in_path.is_null() || out_path.is_null() || order_json.is_null() {
            return RBD_ERR_NULL_PTR;
        }

        let in_c = match unsafe { jstring_to_cstring(&mut env, in_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };
        let out_c = match unsafe { jstring_to_cstring(&mut env, out_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };
        let json_c = match unsafe { jstring_to_cstring(&mut env, order_json) } {
            Some(j) => j,
            None => return RBD_ERR_INVALID_PARAM,
        };

        unsafe { rbd_page_op_reorder(in_c.as_ptr(), out_c.as_ptr(), json_c.as_ptr()) }
    })
}

/// Crop page to bounding box `[x0, y0, x1, y1]`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativePageOpCrop(
    mut env: JNIEnv,
    _class: JClass,
    in_path: jstring,
    out_path: jstring,
    page_num: jint,
    x0: jfloat,
    y0: jfloat,
    x1: jfloat,
    y1: jfloat,
) -> jint {
    jni_boundary(RBD_ERR_PANIC, || {
        if in_path.is_null() || out_path.is_null() {
            return RBD_ERR_NULL_PTR;
        }

        if page_num < 1 {
            return RBD_ERR_PAGE_OUT_OF_RANGE;
        }

        let in_c = match unsafe { jstring_to_cstring(&mut env, in_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };
        let out_c = match unsafe { jstring_to_cstring(&mut env, out_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };

        unsafe {
            rbd_page_op_crop(
                in_c.as_ptr(),
                out_c.as_ptr(),
                page_num as u32,
                x0,
                y0,
                x1,
                y1,
            )
        }
    })
}

/// Insert a blank page after 1-based `after_page`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativePageOpInsertBlank(
    mut env: JNIEnv,
    _class: JClass,
    in_path: jstring,
    out_path: jstring,
    after_page: jint,
    width_pt: jfloat,
    height_pt: jfloat,
) -> jint {
    jni_boundary(RBD_ERR_PANIC, || {
        if in_path.is_null() || out_path.is_null() {
            return RBD_ERR_NULL_PTR;
        }

        if width_pt <= 0.0 || height_pt <= 0.0 {
            return RBD_ERR_INVALID_PARAM;
        }

        let in_c = match unsafe { jstring_to_cstring(&mut env, in_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };
        let out_c = match unsafe { jstring_to_cstring(&mut env, out_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };

        unsafe {
            rbd_page_op_insert_blank(
                in_c.as_ptr(),
                out_c.as_ptr(),
                after_page as u32,
                width_pt,
                height_pt,
            )
        }
    })
}

/// Decrypt a password-protected PDF document.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativePageOpDecrypt(
    mut env: JNIEnv,
    _class: JClass,
    in_path: jstring,
    out_path: jstring,
    password: jstring,
) -> jint {
    jni_boundary(RBD_ERR_PANIC, || {
        if in_path.is_null() || out_path.is_null() {
            return RBD_ERR_NULL_PTR;
        }

        let in_c = match unsafe { jstring_to_cstring(&mut env, in_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };
        let out_c = match unsafe { jstring_to_cstring(&mut env, out_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };
        let pass_c = if password.is_null() {
            CString::new("").unwrap()
        } else {
            match unsafe { jstring_to_cstring(&mut env, password) } {
                Some(p) => p,
                None => return RBD_ERR_INVALID_PARAM,
            }
        };

        unsafe { rbd_page_op_decrypt(in_c.as_ptr(), out_c.as_ptr(), pass_c.as_ptr()) }
    })
}

// ---------------------------------------------------------------------------
// Professional PDF Tools JNI Entrypoints (Phase 4 Track 4B)
// ---------------------------------------------------------------------------

/// Apply permanent redactions to a PDF document via JNI.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeRedactDocument(
    mut env: JNIEnv,
    _class: JClass,
    in_path: jstring,
    out_path: jstring,
    redactions_json: jstring,
) -> jint {
    jni_boundary(RBD_ERR_PANIC, || {
        if in_path.is_null() || out_path.is_null() || redactions_json.is_null() {
            return RBD_ERR_NULL_PTR;
        }

        let in_c = match unsafe { jstring_to_cstring(&mut env, in_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };
        let out_c = match unsafe { jstring_to_cstring(&mut env, out_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };
        let red_c = match unsafe { jstring_to_cstring(&mut env, redactions_json) } {
            Some(r) => r,
            None => return RBD_ERR_INVALID_PARAM,
        };

        unsafe { rbd_pro_redact_document(in_c.as_ptr(), out_c.as_ptr(), red_c.as_ptr()) }
    })
}

/// Compress a PDF document via JNI and return savings statistics as a JSON string (or null on failure).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeCompressDocument(
    mut env: JNIEnv,
    _class: JClass,
    in_path: jstring,
    out_path: jstring,
    remove_unused: jboolean,
    recompress: jboolean,
    strip_meta: jboolean,
) -> jstring {
    jni_boundary(ptr::null_mut(), || {
        if in_path.is_null() || out_path.is_null() {
            return ptr::null_mut();
        }

        let in_c = match unsafe { jstring_to_cstring(&mut env, in_path) } {
            Some(p) => p,
            None => return ptr::null_mut(),
        };
        let out_c = match unsafe { jstring_to_cstring(&mut env, out_path) } {
            Some(p) => p,
            None => return ptr::null_mut(),
        };

        let mut buffer = vec![0u8; 1024];
        let status = unsafe {
            rbd_pro_compress_document(
                in_c.as_ptr(),
                out_c.as_ptr(),
                if remove_unused != JNI_FALSE { 1 } else { 0 },
                if recompress != JNI_FALSE { 1 } else { 0 },
                if strip_meta != JNI_FALSE { 1 } else { 0 },
                buffer.as_mut_ptr() as *mut c_char,
                buffer.len(),
            )
        };

        if status != RBD_OK {
            return ptr::null_mut();
        }

        let c_str = unsafe { CStr::from_ptr(buffer.as_ptr() as *const c_char) };
        let rust_str = c_str.to_str().unwrap_or("{}");
        match env.new_string(rust_str) {
            Ok(js) => js.into_raw(),
            Err(_) => ptr::null_mut(),
        }
    })
}

/// Apply a watermark to all pages in a PDF document via JNI.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeApplyWatermark(
    mut env: JNIEnv,
    _class: JClass,
    in_path: jstring,
    out_path: jstring,
    text: jstring,
    font_size: jfloat,
    rotation_deg: jfloat,
    opacity: jfloat,
    r: jint,
    g: jint,
    b: jint,
    on_top: jboolean,
) -> jint {
    jni_boundary(RBD_ERR_PANIC, || {
        if in_path.is_null() || out_path.is_null() || text.is_null() {
            return RBD_ERR_NULL_PTR;
        }

        let in_c = match unsafe { jstring_to_cstring(&mut env, in_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };
        let out_c = match unsafe { jstring_to_cstring(&mut env, out_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };
        let text_c = match unsafe { jstring_to_cstring(&mut env, text) } {
            Some(t) => t,
            None => return RBD_ERR_INVALID_PARAM,
        };

        unsafe {
            rbd_pro_apply_watermark(
                in_c.as_ptr(),
                out_c.as_ptr(),
                text_c.as_ptr(),
                font_size,
                rotation_deg,
                opacity,
                r.clamp(0, 255) as u8,
                g.clamp(0, 255) as u8,
                b.clamp(0, 255) as u8,
                if on_top != JNI_FALSE { 1 } else { 0 },
            )
        }
    })
}

/// Apply dynamic page numbers to pages via JNI.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeApplyPageNumbers(
    mut env: JNIEnv,
    _class: JClass,
    in_path: jstring,
    out_path: jstring,
    pattern: jstring,
    position: jint,
    font_size: jfloat,
    margin: jfloat,
) -> jint {
    jni_boundary(RBD_ERR_PANIC, || {
        if in_path.is_null() || out_path.is_null() || pattern.is_null() {
            return RBD_ERR_NULL_PTR;
        }

        let in_c = match unsafe { jstring_to_cstring(&mut env, in_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };
        let out_c = match unsafe { jstring_to_cstring(&mut env, out_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };
        let pattern_c = match unsafe { jstring_to_cstring(&mut env, pattern) } {
            Some(pat) => pat,
            None => return RBD_ERR_INVALID_PARAM,
        };

        unsafe {
            rbd_pro_apply_page_numbers(
                in_c.as_ptr(),
                out_c.as_ptr(),
                pattern_c.as_ptr(),
                position,
                font_size,
                margin,
            )
        }
    })
}

/// Apply sequential legal Bates stamp numbers via JNI.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeApplyBatesStamp(
    mut env: JNIEnv,
    _class: JClass,
    in_path: jstring,
    out_path: jstring,
    prefix: jstring,
    start_number: jint,
    padding_digits: jint,
    suffix: jstring,
    position: jint,
) -> jint {
    jni_boundary(RBD_ERR_PANIC, || {
        if in_path.is_null() || out_path.is_null() {
            return RBD_ERR_NULL_PTR;
        }

        let in_c = match unsafe { jstring_to_cstring(&mut env, in_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };
        let out_c = match unsafe { jstring_to_cstring(&mut env, out_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };
        let prefix_c = match unsafe { jstring_to_cstring(&mut env, prefix) } {
            Some(p) => p,
            None => CString::new("").unwrap(),
        };
        let suffix_c = match unsafe { jstring_to_cstring(&mut env, suffix) } {
            Some(s) => s,
            None => CString::new("").unwrap(),
        };

        unsafe {
            rbd_pro_apply_bates_stamp(
                in_c.as_ptr(),
                out_c.as_ptr(),
                prefix_c.as_ptr(),
                start_number.max(0) as u32,
                padding_digits.max(0) as u32,
                suffix_c.as_ptr(),
                position,
            )
        }
    })
}

/// Read all interactive form fields as a JSON string via JNI.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeReadFormFields(
    mut env: JNIEnv,
    _class: JClass,
    in_path: jstring,
) -> jstring {
    jni_boundary(ptr::null_mut(), || {
        if in_path.is_null() {
            return ptr::null_mut();
        }

        let in_c = match unsafe { jstring_to_cstring(&mut env, in_path) } {
            Some(p) => p,
            None => return ptr::null_mut(),
        };

        let mut buffer = vec![0u8; 65536];
        let status = unsafe {
            rbd_pro_read_form_fields(
                in_c.as_ptr(),
                buffer.as_mut_ptr() as *mut c_char,
                buffer.len(),
            )
        };

        if status != RBD_OK {
            return ptr::null_mut();
        }

        let c_str = unsafe { CStr::from_ptr(buffer.as_ptr() as *const c_char) };
        let rust_str = c_str.to_str().unwrap_or("[]");
        match env.new_string(rust_str) {
            Ok(js) => js.into_raw(),
            Err(_) => ptr::null_mut(),
        }
    })
}

/// Fill interactive form fields and optionally flatten them via JNI.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeFillFormFields(
    mut env: JNIEnv,
    _class: JClass,
    in_path: jstring,
    out_path: jstring,
    field_values_json: jstring,
    flatten: jboolean,
) -> jint {
    jni_boundary(RBD_ERR_PANIC, || {
        if in_path.is_null() || out_path.is_null() || field_values_json.is_null() {
            return RBD_ERR_NULL_PTR;
        }

        let in_c = match unsafe { jstring_to_cstring(&mut env, in_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };
        let out_c = match unsafe { jstring_to_cstring(&mut env, out_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };
        let fv_c = match unsafe { jstring_to_cstring(&mut env, field_values_json) } {
            Some(v) => v,
            None => return RBD_ERR_INVALID_PARAM,
        };

        unsafe {
            rbd_pro_fill_form_fields(
                in_c.as_ptr(),
                out_c.as_ptr(),
                fv_c.as_ptr(),
                if flatten != JNI_FALSE { 1 } else { 0 },
            )
        }
    })
}

/// Convert PDF pages to image files (PNG/JPEG) via JNI.
/// Returns JSON array of output file paths, or null on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativePdfToImages(
    mut env: JNIEnv,
    _class: JClass,
    in_path: jstring,
    out_dir: jstring,
    dpi: jint,
    format: jint,
    page_range_json: jstring,
) -> jstring {
    jni_boundary(ptr::null_mut(), || {
        if in_path.is_null() || out_dir.is_null() {
            return ptr::null_mut();
        }

        let in_c = match unsafe { jstring_to_cstring(&mut env, in_path) } {
            Some(p) => p,
            None => return ptr::null_mut(),
        };
        let out_c = match unsafe { jstring_to_cstring(&mut env, out_dir) } {
            Some(p) => p,
            None => return ptr::null_mut(),
        };
        let pr_c = if page_range_json.is_null() {
            None
        } else {
            unsafe { jstring_to_cstring(&mut env, page_range_json) }
        };

        let pr_ptr = pr_c.as_ref().map(|c| c.as_ptr()).unwrap_or(ptr::null());

        let mut buffer = vec![0u8; 65536];
        let status = unsafe {
            rbd_convert_pdf_to_images(
                in_c.as_ptr(),
                out_c.as_ptr(),
                dpi as u32,
                format,
                pr_ptr,
                buffer.as_mut_ptr() as *mut c_char,
                buffer.len(),
            )
        };

        if status != RBD_OK {
            return ptr::null_mut();
        }

        let c_str = unsafe { CStr::from_ptr(buffer.as_ptr() as *const c_char) };
        let rust_str = c_str.to_str().unwrap_or("[]");
        match env.new_string(rust_str) {
            Ok(js) => js.into_raw(),
            Err(_) => ptr::null_mut(),
        }
    })
}

/// Convert one or more images into a single PDF document via JNI.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeImagesToPdf(
    mut env: JNIEnv,
    _class: JClass,
    images_json: jstring,
    page_size: jint,
    margin_pt: jfloat,
    out_path: jstring,
) -> jint {
    jni_boundary(RBD_ERR_PANIC, || {
        if images_json.is_null() || out_path.is_null() {
            return RBD_ERR_NULL_PTR;
        }

        let img_c = match unsafe { jstring_to_cstring(&mut env, images_json) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };
        let out_c = match unsafe { jstring_to_cstring(&mut env, out_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };

        unsafe { rbd_convert_images_to_pdf(img_c.as_ptr(), page_size, margin_pt, out_c.as_ptr()) }
    })
}

/// Convert markdown text into a paginated PDF document via JNI.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeMarkdownToPdf(
    mut env: JNIEnv,
    _class: JClass,
    markdown_text: jstring,
    out_path: jstring,
) -> jint {
    jni_boundary(RBD_ERR_PANIC, || {
        if markdown_text.is_null() || out_path.is_null() {
            return RBD_ERR_NULL_PTR;
        }

        let md_c = match unsafe { jstring_to_cstring(&mut env, markdown_text) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };
        let out_c = match unsafe { jstring_to_cstring(&mut env, out_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };

        unsafe { rbd_convert_markdown_to_pdf(md_c.as_ptr(), out_c.as_ptr()) }
    })
}

/// Extract text from Office document (DOCX, XLSX, PPTX) via JNI.
/// Returns text string or null on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeOfficeExtractText(
    mut env: JNIEnv,
    _class: JClass,
    in_path: jstring,
) -> jstring {
    jni_boundary(ptr::null_mut(), || {
        if in_path.is_null() {
            return ptr::null_mut();
        }
        let in_c = match unsafe { jstring_to_cstring(&mut env, in_path) } {
            Some(p) => p,
            None => return ptr::null_mut(),
        };

        let mut buffer = vec![0u8; 131072]; // 128 KB initial buffer
        let status = unsafe {
            rbd_office_extract_text(
                in_c.as_ptr(),
                buffer.as_mut_ptr() as *mut c_char,
                buffer.len(),
            )
        };

        if status != RBD_OK {
            return ptr::null_mut();
        }

        let c_str = unsafe { CStr::from_ptr(buffer.as_ptr() as *const c_char) };
        let rust_str = c_str.to_str().unwrap_or("");
        match env.new_string(rust_str) {
            Ok(js) => js.into_raw(),
            Err(_) => ptr::null_mut(),
        }
    })
}

/// Inspect Office document and return JSON metadata via JNI.
/// Returns JSON string or null on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeOfficeInspect(
    mut env: JNIEnv,
    _class: JClass,
    in_path: jstring,
) -> jstring {
    jni_boundary(ptr::null_mut(), || {
        if in_path.is_null() {
            return ptr::null_mut();
        }
        let in_c = match unsafe { jstring_to_cstring(&mut env, in_path) } {
            Some(p) => p,
            None => return ptr::null_mut(),
        };

        let mut buffer = vec![0u8; 16384];
        let status = unsafe {
            rbd_office_inspect(
                in_c.as_ptr(),
                buffer.as_mut_ptr() as *mut c_char,
                buffer.len(),
            )
        };

        if status != RBD_OK {
            return ptr::null_mut();
        }

        let c_str = unsafe { CStr::from_ptr(buffer.as_ptr() as *const c_char) };
        let rust_str = c_str.to_str().unwrap_or("{}");
        match env.new_string(rust_str) {
            Ok(js) => js.into_raw(),
            Err(_) => ptr::null_mut(),
        }
    })
}

/// Create a minimal standard-compliant DOCX file from raw text via JNI.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeOfficeCreateDocx(
    mut env: JNIEnv,
    _class: JClass,
    text: jstring,
    out_path: jstring,
) -> jint {
    jni_boundary(RBD_ERR_PANIC, || {
        if text.is_null() || out_path.is_null() {
            return RBD_ERR_NULL_PTR;
        }

        let text_c = match unsafe { jstring_to_cstring(&mut env, text) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };
        let out_c = match unsafe { jstring_to_cstring(&mut env, out_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };

        unsafe { rbd_office_create_docx(text_c.as_ptr(), out_c.as_ptr()) }
    })
}

/// Summarize document text using offline natural language processing via JNI.
/// Returns JSON-encoded `DocumentSummary` or null on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeSummarizeDocument(
    env: JNIEnv,
    _class: JClass,
    doc_handle: jlong,
    max_sentences: jint,
) -> jstring {
    jni_boundary(ptr::null_mut(), || {
        if doc_handle == 0 {
            return ptr::null_mut();
        }

        let mut buffer = vec![0u8; 65536]; // 64 KB buffer
        let status = unsafe {
            crate::rbd_ai_summarize_document(
                doc_handle as *mut c_void,
                max_sentences as u32,
                buffer.as_mut_ptr() as *mut c_char,
                buffer.len(),
            )
        };

        if status != RBD_OK {
            return ptr::null_mut();
        }

        let c_str = unsafe { CStr::from_ptr(buffer.as_ptr() as *const c_char) };
        let rust_str = c_str.to_str().unwrap_or("{}");
        match env.new_string(rust_str) {
            Ok(js) => js.into_raw(),
            Err(_) => ptr::null_mut(),
        }
    })
}

/// Query the document using natural language via JNI.
/// Returns JSON-encoded `DocumentAnswer` or null on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeAskDocument(
    mut env: JNIEnv,
    _class: JClass,
    doc_handle: jlong,
    query: jstring,
) -> jstring {
    jni_boundary(ptr::null_mut(), || {
        if doc_handle == 0 || query.is_null() {
            return ptr::null_mut();
        }

        let query_c = match unsafe { jstring_to_cstring(&mut env, query) } {
            Some(p) => p,
            None => return ptr::null_mut(),
        };

        let mut buffer = vec![0u8; 65536];
        let status = unsafe {
            crate::rbd_ai_ask_document(
                doc_handle as *mut c_void,
                query_c.as_ptr(),
                buffer.as_mut_ptr() as *mut c_char,
                buffer.len(),
            )
        };

        if status != RBD_OK {
            return ptr::null_mut();
        }

        let c_str = unsafe { CStr::from_ptr(buffer.as_ptr() as *const c_char) };
        let rust_str = c_str.to_str().unwrap_or("{}");
        match env.new_string(rust_str) {
            Ok(js) => js.into_raw(),
            Err(_) => ptr::null_mut(),
        }
    })
}

/// Extract fluid, structured paragraphs from a page for reflow editing via JNI.
/// Returns JSON-encoded array of `ReflowParagraph` or null on error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeReflowPageText(
    env: JNIEnv,
    _class: JClass,
    doc_handle: jlong,
    page_index: jint,
) -> jstring {
    jni_boundary(ptr::null_mut(), || {
        if doc_handle == 0 {
            return ptr::null_mut();
        }

        let mut buffer = vec![0u8; 65536];
        let status = unsafe {
            crate::rbd_document_reflow_page_text(
                doc_handle as *mut c_void,
                page_index as u32,
                buffer.as_mut_ptr() as *mut c_char,
                buffer.len(),
            )
        };

        if status != RBD_OK {
            return ptr::null_mut();
        }

        let c_str = unsafe { CStr::from_ptr(buffer.as_ptr() as *const c_char) };
        let rust_str = c_str.to_str().unwrap_or("[]");
        match env.new_string(rust_str) {
            Ok(js) => js.into_raw(),
            Err(_) => ptr::null_mut(),
        }
    })
}

/// Replace text in a paragraph bounding box with automatic multi-line reflow via JNI.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn Java_com_rbdofis_app_core_RbdNativeBridge_nativeReplaceParagraphText(
    mut env: JNIEnv,
    _class: JClass,
    doc_handle: jlong,
    input_path: jstring,
    page_index: jint,
    x0: jfloat,
    y0: jfloat,
    x1: jfloat,
    y1: jfloat,
    new_text: jstring,
    font_size: jfloat,
    output_path: jstring,
) -> jint {
    jni_boundary(RBD_ERR_PANIC, || {
        if doc_handle == 0 || input_path.is_null() || new_text.is_null() || output_path.is_null() {
            return RBD_ERR_NULL_PTR;
        }

        let in_c = match unsafe { jstring_to_cstring(&mut env, input_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };
        let text_c = match unsafe { jstring_to_cstring(&mut env, new_text) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };
        let out_c = match unsafe { jstring_to_cstring(&mut env, output_path) } {
            Some(p) => p,
            None => return RBD_ERR_INVALID_PARAM,
        };

        unsafe {
            crate::rbd_document_replace_paragraph_text(
                doc_handle as *mut c_void,
                in_c.as_ptr(),
                page_index as u32,
                x0,
                y0,
                x1,
                y1,
                text_c.as_ptr(),
                font_size,
                out_c.as_ptr(),
            )
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe fn fake_env() -> JNIEnv<'static> {
        unsafe { std::mem::zeroed() }
    }

    unsafe fn fake_class() -> JClass<'static> {
        unsafe { std::mem::zeroed() }
    }

    #[test]
    fn test_jni_engine_init_and_free() {
        unsafe {
            let handle = Java_com_rbdofis_app_core_RbdNativeBridge_nativeEngineInit(
                fake_env(),
                fake_class(),
            );
            assert_ne!(
                handle, 0,
                "Engine init must return a non-zero native handle"
            );

            Java_com_rbdofis_app_core_RbdNativeBridge_nativeEngineFree(
                fake_env(),
                fake_class(),
                handle,
            );
        }
    }

    #[test]
    fn test_jni_null_safety_and_defaults() {
        unsafe {
            // Null handles must return error codes / nulls cleanly without panicking
            assert_eq!(
                Java_com_rbdofis_app_core_RbdNativeBridge_nativeDocumentGetPageCount(
                    fake_env(),
                    fake_class(),
                    0,
                ),
                RBD_ERR_NULL_PTR
            );

            assert_eq!(
                Java_com_rbdofis_app_core_RbdNativeBridge_nativeDocumentUndo(
                    fake_env(),
                    fake_class(),
                    0,
                ),
                JNI_FALSE
            );

            assert_eq!(
                Java_com_rbdofis_app_core_RbdNativeBridge_nativeDocumentRedo(
                    fake_env(),
                    fake_class(),
                    0,
                ),
                JNI_FALSE
            );

            assert!(
                Java_com_rbdofis_app_core_RbdNativeBridge_nativeDocumentGetPageSize(
                    fake_env(),
                    fake_class(),
                    0,
                    0,
                )
                .is_null()
            );

            assert_eq!(
                Java_com_rbdofis_app_core_RbdNativeBridge_nativeDocumentRemoveAnnotation(
                    fake_env(),
                    fake_class(),
                    0,
                    1,
                ),
                RBD_ERR_NULL_PTR
            );

            assert_eq!(
                Java_com_rbdofis_app_core_RbdNativeBridge_nativePageOpRotate(
                    fake_env(),
                    fake_class(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    90,
                ),
                RBD_ERR_NULL_PTR
            );

            assert_eq!(
                Java_com_rbdofis_app_core_RbdNativeBridge_nativePageOpDeletePages(
                    fake_env(),
                    fake_class(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                ),
                RBD_ERR_NULL_PTR
            );

            assert_eq!(
                Java_com_rbdofis_app_core_RbdNativeBridge_nativePageOpMerge(
                    fake_env(),
                    fake_class(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                ),
                RBD_ERR_NULL_PTR
            );

            assert_eq!(
                Java_com_rbdofis_app_core_RbdNativeBridge_nativePageOpSplit(
                    fake_env(),
                    fake_class(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    1,
                    2,
                ),
                RBD_ERR_NULL_PTR
            );

            assert_eq!(
                Java_com_rbdofis_app_core_RbdNativeBridge_nativePageOpReorder(
                    fake_env(),
                    fake_class(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                ),
                RBD_ERR_NULL_PTR
            );

            assert_eq!(
                Java_com_rbdofis_app_core_RbdNativeBridge_nativePageOpCrop(
                    fake_env(),
                    fake_class(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    1,
                    0.0,
                    0.0,
                    100.0,
                    100.0,
                ),
                RBD_ERR_NULL_PTR
            );

            assert_eq!(
                Java_com_rbdofis_app_core_RbdNativeBridge_nativePageOpInsertBlank(
                    fake_env(),
                    fake_class(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    1,
                    100.0,
                    100.0,
                ),
                RBD_ERR_NULL_PTR
            );

            assert_eq!(
                Java_com_rbdofis_app_core_RbdNativeBridge_nativePageOpDecrypt(
                    fake_env(),
                    fake_class(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                ),
                RBD_ERR_NULL_PTR
            );
        }
    }

    #[test]
    fn test_jni_panic_containment() {
        let default_val = 1234;
        let res = jni_boundary(default_val, || {
            panic!("Simulated critical panic inside JNI boundary!");
        });
        assert_eq!(res, default_val, "Panic must be contained by jni_boundary");
    }

    #[test]
    fn test_jni_pro_tools() {
        unsafe {
            // Null pointer safety checks
            assert_eq!(
                Java_com_rbdofis_app_core_RbdNativeBridge_nativeRedactDocument(
                    fake_env(),
                    fake_class(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                ),
                RBD_ERR_NULL_PTR
            );

            assert!(
                Java_com_rbdofis_app_core_RbdNativeBridge_nativeCompressDocument(
                    fake_env(),
                    fake_class(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    JNI_TRUE,
                    JNI_TRUE,
                    JNI_FALSE,
                )
                .is_null()
            );

            assert_eq!(
                Java_com_rbdofis_app_core_RbdNativeBridge_nativeApplyWatermark(
                    fake_env(),
                    fake_class(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    36.0,
                    45.0,
                    0.5,
                    0,
                    0,
                    0,
                    JNI_TRUE,
                ),
                RBD_ERR_NULL_PTR
            );

            assert_eq!(
                Java_com_rbdofis_app_core_RbdNativeBridge_nativeApplyPageNumbers(
                    fake_env(),
                    fake_class(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    4,
                    10.0,
                    36.0,
                ),
                RBD_ERR_NULL_PTR
            );

            assert_eq!(
                Java_com_rbdofis_app_core_RbdNativeBridge_nativeApplyBatesStamp(
                    fake_env(),
                    fake_class(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    1,
                    6,
                    ptr::null_mut(),
                    5,
                ),
                RBD_ERR_NULL_PTR
            );

            assert!(
                Java_com_rbdofis_app_core_RbdNativeBridge_nativeReadFormFields(
                    fake_env(),
                    fake_class(),
                    ptr::null_mut(),
                )
                .is_null()
            );

            assert_eq!(
                Java_com_rbdofis_app_core_RbdNativeBridge_nativeFillFormFields(
                    fake_env(),
                    fake_class(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    JNI_TRUE,
                ),
                RBD_ERR_NULL_PTR
            );
        }
    }

    #[test]
    fn test_jni_converter() {
        unsafe {
            // Null pointer safety checks
            assert!(
                Java_com_rbdofis_app_core_RbdNativeBridge_nativePdfToImages(
                    fake_env(),
                    fake_class(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    150,
                    0,
                    ptr::null_mut(),
                )
                .is_null()
            );

            assert_eq!(
                Java_com_rbdofis_app_core_RbdNativeBridge_nativeImagesToPdf(
                    fake_env(),
                    fake_class(),
                    ptr::null_mut(),
                    0,
                    10.0,
                    ptr::null_mut(),
                ),
                RBD_ERR_NULL_PTR
            );

            assert_eq!(
                Java_com_rbdofis_app_core_RbdNativeBridge_nativeMarkdownToPdf(
                    fake_env(),
                    fake_class(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                ),
                RBD_ERR_NULL_PTR
            );

            assert!(
                Java_com_rbdofis_app_core_RbdNativeBridge_nativeOfficeExtractText(
                    fake_env(),
                    fake_class(),
                    ptr::null_mut(),
                )
                .is_null()
            );

            assert!(
                Java_com_rbdofis_app_core_RbdNativeBridge_nativeOfficeInspect(
                    fake_env(),
                    fake_class(),
                    ptr::null_mut(),
                )
                .is_null()
            );

            assert_eq!(
                Java_com_rbdofis_app_core_RbdNativeBridge_nativeOfficeCreateDocx(
                    fake_env(),
                    fake_class(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                ),
                RBD_ERR_NULL_PTR
            );
        }
    }
}
