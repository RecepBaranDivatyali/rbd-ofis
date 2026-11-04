//! Android JNI (Java Native Interface) bridge for RBD OFİS core.
//! Matches `com.rbdofis.app.core.RbdNativeBridge` in apps/android.
//!
//! All entrypoints are wrapped in `catch_unwind` to ensure panics
//! never escape across the JNI boundary into the ART runtime VM.

#![allow(clippy::missing_safety_doc)]

use crate::{
    RBD_ERR_BUFFER_TOO_SMALL, RBD_ERR_INVALID_PARAM, RBD_ERR_NULL_PTR, RBD_ERR_PAGE_OUT_OF_RANGE,
    RBD_ERR_PANIC, RBD_OK, rbd_document_add_annotation_json, rbd_document_extract_page_text,
    rbd_document_free, rbd_document_get_annotations_json, rbd_document_get_page_count,
    rbd_document_get_page_size, rbd_document_open_file, rbd_document_open_memory,
    rbd_document_redo, rbd_document_remove_annotation, rbd_document_save_annotations,
    rbd_document_search_json, rbd_document_undo, rbd_engine_free, rbd_engine_init, rbd_free_string,
    rbd_page_op_crop, rbd_page_op_decrypt, rbd_page_op_delete_pages, rbd_page_op_insert_blank,
    rbd_page_op_merge, rbd_page_op_reorder, rbd_page_op_rotate, rbd_page_op_split,
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
}
