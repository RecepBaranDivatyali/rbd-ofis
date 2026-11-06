/**
 * RBD OFİS Native C-ABI Header (rbd_ffi.h)
 * Multiplatform C/C++ and Swift header for librbd_ffi.
 * 
 * Supports Windows, Linux, macOS, iOS, iPadOS, and Android.
 * Memory safe: all public entrypoints are catch_unwind guarded.
 */

#ifndef RBD_FFI_H
#define RBD_FFI_H

#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Status Codes */
#define RBD_OK 0
#define RBD_ERR_NULL_PTR -1
#define RBD_ERR_INVALID_PARAM -2
#define RBD_ERR_FILE_IO -3
#define RBD_ERR_CORRUPT_PDF -4
#define RBD_ERR_PASSWORD_REQUIRED -5
#define RBD_ERR_PAGE_OUT_OF_RANGE -6
#define RBD_ERR_BUFFER_TOO_SMALL -7
#define RBD_ERR_ENGINE_FAILURE -8
#define RBD_ERR_PANIC -99

/* Opaque handles */
typedef void* RbdEngineHandle;
typedef void* RbdDocumentHandle;
typedef void* RbdSessionHandle;

/* Engine & Document Lifecycle */
int32_t rbd_engine_init(RbdEngineHandle* out_engine);
void rbd_engine_free(RbdEngineHandle engine);

int32_t rbd_document_open_file(RbdEngineHandle engine, const char* file_path, RbdDocumentHandle* out_doc);
int32_t rbd_document_open_memory(RbdEngineHandle engine, const uint8_t* bytes, size_t length, RbdDocumentHandle* out_doc);
void rbd_document_free(RbdDocumentHandle doc);

int32_t rbd_document_get_page_count(RbdDocumentHandle doc, int32_t* out_count);
int32_t rbd_document_get_page_size(RbdDocumentHandle doc, int32_t page_index, double* out_width, double* out_height);

/* Tile & Viewport Rendering */
int32_t rbd_render_tile_into_buffer(
    RbdDocumentHandle doc,
    int32_t page_index,
    int32_t tile_x,
    int32_t tile_y,
    int32_t tile_width,
    int32_t tile_height,
    double zoom,
    uint8_t* buffer,
    int32_t stride,
    int32_t pixel_format
);

/* Text Extraction & Search */
int32_t rbd_document_extract_page_text(RbdDocumentHandle doc, int32_t page_index, char** out_text);
int32_t rbd_document_search_json(RbdDocumentHandle doc, const char* query, int32_t case_sensitive, char** out_json);

/* Annotations & Undo/Redo */
int32_t rbd_document_add_annotation_json(RbdDocumentHandle doc, int32_t page_index, const char* annot_json);
int32_t rbd_document_remove_annotation(RbdDocumentHandle doc, int32_t page_index, const char* annot_id);
int32_t rbd_document_undo(RbdDocumentHandle doc);
int32_t rbd_document_redo(RbdDocumentHandle doc);
int32_t rbd_document_get_annotations_json(RbdDocumentHandle doc, int32_t page_index, char** out_json);
int32_t rbd_document_save_annotations(RbdDocumentHandle doc, const char* out_path);
int32_t rbd_document_save_annotations_to_path(RbdDocumentHandle doc, const char* in_path, const char* out_path);

/* Diagnostics & Error Handling */
int32_t rbd_free_string(char* text);
const char* rbd_get_last_error(void);
const char* rbd_version_string(void);

/* Page Operations */
int32_t rbd_page_op_rotate(const char* in_path, const char* out_path, int32_t degrees);
int32_t rbd_page_op_delete_pages(const char* in_path, const char* out_path, const char* pages_json);
int32_t rbd_page_op_split(const char* in_path, const char* out_path, uint32_t start_page, uint32_t end_page);
int32_t rbd_page_op_merge(const char* out_path, const char* in_paths_json);
int32_t rbd_page_op_reorder(const char* in_path, const char* out_path, const char* order_json);
int32_t rbd_page_op_crop(const char* in_path, const char* out_path, uint32_t page_num, float x0, float y0, float x1, float y1);
int32_t rbd_page_op_insert_blank(const char* in_path, const char* out_path, uint32_t after_page, float width_pt, float height_pt);
int32_t rbd_page_op_decrypt(const char* in_path, const char* out_path, const char* password_utf8);

/* Professional PDF Tools */
int32_t rbd_pro_redact_document(const char* in_path, const char* out_path, const char* redactions_json);
int32_t rbd_pro_compress_document(const char* in_path, const char* out_path, int32_t remove_unused, int32_t recompress, int32_t strip_metadata, char* out_savings_json, size_t max_len);
int32_t rbd_pro_apply_watermark(const char* in_path, const char* out_path, const char* text, float font_size, float rotation_deg, float opacity, uint8_t r, uint8_t g, uint8_t b, int32_t on_top);
int32_t rbd_pro_apply_page_numbers(const char* in_path, const char* out_path, const char* pattern, int32_t position, float font_size, float margin);
int32_t rbd_pro_apply_bates_stamp(const char* in_path, const char* out_path, const char* prefix, uint32_t start_number, uint32_t padding_digits, const char* suffix, int32_t position);
int32_t rbd_pro_read_form_fields(const char* in_path, char* out_json, size_t max_len);
int32_t rbd_pro_fill_form_fields(const char* in_path, const char* out_path, const char* field_values_json, int32_t flatten);

/* Document Converters */
int32_t rbd_convert_pdf_to_images(const char* in_path, const char* out_dir, uint32_t dpi, int32_t format, const char* page_range_json, char* out_paths_json, size_t max_paths_len);
int32_t rbd_convert_images_to_pdf(const char* images_json, int32_t page_size, float margin_pt, const char* out_path);
int32_t rbd_convert_markdown_to_pdf(const char* markdown_text, const char* out_path);

/* Office Documents (DOCX, XLSX, PPTX) */
int32_t rbd_office_extract_text(const char* in_path, char* out_text_buf, size_t max_len);
int32_t rbd_office_inspect(const char* in_path, char* out_json_buf, size_t max_len);
int32_t rbd_office_create_docx(const char* text, const char* out_path);

#ifdef __cplusplus
}
#endif

#endif /* RBD_FFI_H */
