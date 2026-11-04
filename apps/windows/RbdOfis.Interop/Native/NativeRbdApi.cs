using System;
using System.Runtime.InteropServices;

namespace RbdOfis.Interop.Native;

/// <summary>
/// Production implementation of INativeRbdApi dispatching directly to rbd_ffi.dll P/Invoke entrypoints.
/// </summary>
public sealed class NativeRbdApi : INativeRbdApi
{
    public static NativeRbdApi Instance { get; } = new();

    private NativeRbdApi() { }

    public int EngineInit(out IntPtr outEngine) =>
        NativeMethods.rbd_engine_init(out outEngine);

    public void EngineFree(IntPtr engine) =>
        NativeMethods.rbd_engine_free(engine);

    public int DocumentOpenFile(SafeEngineHandle engine, string filePath, out IntPtr outDoc) =>
        NativeMethods.rbd_document_open_file(engine, filePath, out outDoc);

    public int DocumentOpenMemory(SafeEngineHandle engine, IntPtr bytes, nuint length, out IntPtr outDoc) =>
        NativeMethods.rbd_document_open_memory(engine, bytes, length, out outDoc);

    public void DocumentFree(IntPtr doc) =>
        NativeMethods.rbd_document_free(doc);

    public int DocumentGetPageCount(SafeDocumentHandle doc, out int outCount) =>
        NativeMethods.rbd_document_get_page_count(doc, out outCount);

    public int DocumentGetPageSize(SafeDocumentHandle doc, int pageIndex, out double outWidth, out double outHeight) =>
        NativeMethods.rbd_document_get_page_size(doc, pageIndex, out outWidth, out outHeight);

    public int RenderTileIntoBuffer(
        SafeDocumentHandle doc,
        int pageIndex,
        int tileX,
        int tileY,
        int tileWidth,
        int tileHeight,
        double zoom,
        IntPtr buffer,
        int stride,
        int pixelFormat) =>
        NativeMethods.rbd_render_tile_into_buffer(doc, pageIndex, tileX, tileY, tileWidth, tileHeight, zoom, buffer, stride, pixelFormat);

    public string GetLastError()
    {
        IntPtr ptr = NativeMethods.rbd_get_last_error();
        return ptr != IntPtr.Zero ? (Marshal.PtrToStringUTF8(ptr) ?? string.Empty) : string.Empty;
    }

    public string GetVersionString()
    {
        IntPtr ptr = NativeMethods.rbd_version_string();
        return ptr != IntPtr.Zero ? (Marshal.PtrToStringUTF8(ptr) ?? string.Empty) : string.Empty;
    }

    public int DocumentExtractPageText(SafeDocumentHandle doc, uint pageIndex, out string text)
    {
        int status = NativeMethods.rbd_document_extract_page_text(doc, pageIndex, out IntPtr outPtr);
        if (status == (int)RbdStatusCode.Success && outPtr != IntPtr.Zero)
        {
            text = Marshal.PtrToStringUTF8(outPtr) ?? string.Empty;
            NativeMethods.rbd_free_string(outPtr);
            return status;
        }
        text = string.Empty;
        return status;
    }

    public int DocumentSearchJson(SafeDocumentHandle doc, string query, bool caseSensitive, out string json)
    {
        int status = NativeMethods.rbd_document_search_json(doc, query, caseSensitive ? 1 : 0, out IntPtr outPtr);
        if (status == (int)RbdStatusCode.Success && outPtr != IntPtr.Zero)
        {
            json = Marshal.PtrToStringUTF8(outPtr) ?? "[]";
            NativeMethods.rbd_free_string(outPtr);
            return status;
        }
        json = "[]";
        return status;
    }

    public int DocumentGetAnnotationsJson(SafeDocumentHandle doc, uint pageIndex, out string json)
    {
        int status = NativeMethods.rbd_document_get_annotations_json(doc, pageIndex, out IntPtr outPtr);
        if (status == (int)RbdStatusCode.Success && outPtr != IntPtr.Zero)
        {
            json = Marshal.PtrToStringUTF8(outPtr) ?? "[]";
            NativeMethods.rbd_free_string(outPtr);
            return status;
        }
        json = "[]";
        return status;
    }

    public int DocumentAddAnnotationJson(SafeDocumentHandle doc, string annotJson) =>
        NativeMethods.rbd_document_add_annotation_json(doc, annotJson);

    public int DocumentRemoveAnnotation(SafeDocumentHandle doc, ulong annotId) =>
        NativeMethods.rbd_document_remove_annotation(doc, annotId);

    public int DocumentUpdateAnnotationJson(SafeDocumentHandle doc, string annotJson) =>
        NativeMethods.rbd_document_update_annotation_json(doc, annotJson);

    public int DocumentUndo(SafeDocumentHandle doc, out bool didUndo)
    {
        int status = NativeMethods.rbd_document_undo(doc, out int result);
        didUndo = result != 0;
        return status;
    }

    public int DocumentRedo(SafeDocumentHandle doc, out bool didRedo)
    {
        int status = NativeMethods.rbd_document_redo(doc, out int result);
        didRedo = result != 0;
        return status;
    }

    public int DocumentCanUndo(SafeDocumentHandle doc, out bool canUndo)
    {
        int status = NativeMethods.rbd_document_can_undo(doc, out int result);
        canUndo = result != 0;
        return status;
    }

    public int DocumentCanRedo(SafeDocumentHandle doc, out bool canRedo)
    {
        int status = NativeMethods.rbd_document_can_redo(doc, out int result);
        canRedo = result != 0;
        return status;
    }

    public int DocumentSaveAnnotations(SafeDocumentHandle doc, string outputPath) =>
        NativeMethods.rbd_document_save_annotations(doc, outputPath);

    public int DocumentSaveAnnotationsToPath(SafeDocumentHandle doc, string inputPath, string outputPath) =>
        NativeMethods.rbd_document_save_annotations_to_path(doc, inputPath, outputPath);

    // Page operations
    public int PageOpRotate(string inPath, string outPath, int degrees) =>
        NativeMethods.rbd_page_op_rotate(inPath, outPath, degrees);

    public int PageOpDeletePages(string inPath, string outPath, string pagesJson) =>
        NativeMethods.rbd_page_op_delete_pages(inPath, outPath, pagesJson);

    public int PageOpSplit(string inPath, string outPath, uint startPage, uint endPage) =>
        NativeMethods.rbd_page_op_split(inPath, outPath, startPage, endPage);

    public int PageOpMerge(string outPath, string inPathsJson) =>
        NativeMethods.rbd_page_op_merge(outPath, inPathsJson);

    public int PageOpReorder(string inPath, string outPath, string orderJson) =>
        NativeMethods.rbd_page_op_reorder(inPath, outPath, orderJson);

    public int PageOpCrop(string inPath, string outPath, uint pageNum, float x0, float y0, float x1, float y1) =>
        NativeMethods.rbd_page_op_crop(inPath, outPath, pageNum, x0, y0, x1, y1);

    public int PageOpInsertBlank(string inPath, string outPath, uint afterPage, float widthPt, float heightPt) =>
        NativeMethods.rbd_page_op_insert_blank(inPath, outPath, afterPage, widthPt, heightPt);

    public int PageOpDecrypt(string inPath, string outPath, string passwordUtf8) =>
        NativeMethods.rbd_page_op_decrypt(inPath, outPath, passwordUtf8);

    // Professional PDF Tools (Phase 4 Track 4B/4C)
    public int ProRedactDocument(string inPath, string outPath, string redactionsJson) =>
        NativeMethods.rbd_pro_redact_document(inPath, outPath, redactionsJson);

    public unsafe int ProCompressDocument(string inPath, string outPath, bool removeUnused, bool recompress, bool stripMeta, out string savingsJson)
    {
        savingsJson = string.Empty;
        byte[] buffer = new byte[4096];
        fixed (byte* p = buffer)
        {
            int status = NativeMethods.rbd_pro_compress_document(
                inPath,
                outPath,
                removeUnused ? 1 : 0,
                recompress ? 1 : 0,
                stripMeta ? 1 : 0,
                (IntPtr)p,
                (nuint)buffer.Length);

            if (status == 0)
            {
                int nullIdx = Array.IndexOf(buffer, (byte)0);
                int len = nullIdx >= 0 ? nullIdx : buffer.Length;
                savingsJson = System.Text.Encoding.UTF8.GetString(buffer, 0, len);
            }
            return status;
        }
    }

    public int ProApplyWatermark(string inPath, string outPath, string text, float fontSize, float rotationDeg, float opacity, byte r, byte g, byte b, bool onTop) =>
        NativeMethods.rbd_pro_apply_watermark(inPath, outPath, text, fontSize, rotationDeg, opacity, r, g, b, onTop ? 1 : 0);

    public int ProApplyPageNumbers(string inPath, string outPath, string pattern, int position, float fontSize, float margin) =>
        NativeMethods.rbd_pro_apply_page_numbers(inPath, outPath, pattern, position, fontSize, margin);

    public int ProApplyBatesStamp(string inPath, string outPath, string prefix, uint startNumber, uint paddingDigits, string suffix, int position) =>
        NativeMethods.rbd_pro_apply_bates_stamp(inPath, outPath, prefix, startNumber, paddingDigits, suffix, position);

    public unsafe int ProReadFormFields(string inPath, out string fieldsJson)
    {
        fieldsJson = "[]";
        int bufSize = 65536;
        for (int attempt = 0; attempt < 4; attempt++)
        {
            byte[] buffer = new byte[bufSize];
            fixed (byte* p = buffer)
            {
                int status = NativeMethods.rbd_pro_read_form_fields(inPath, (IntPtr)p, (nuint)buffer.Length);
                if (status == 0)
                {
                    int nullIdx = Array.IndexOf(buffer, (byte)0);
                    int len = nullIdx >= 0 ? nullIdx : buffer.Length;
                    fieldsJson = System.Text.Encoding.UTF8.GetString(buffer, 0, len);
                    return 0;
                }
                if (status == (int)RbdStatusCode.ErrorBufferTooSmall)
                {
                    bufSize *= 4;
                    continue;
                }
                return status;
            }
        }
        return (int)RbdStatusCode.ErrorBufferTooSmall;
    }

    public int ProFillFormFields(string inPath, string outPath, string fieldValuesJson, bool flatten) =>
        NativeMethods.rbd_pro_fill_form_fields(inPath, outPath, fieldValuesJson, flatten ? 1 : 0);
}
