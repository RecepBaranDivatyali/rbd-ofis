using System;

namespace RbdOfis.Interop.Native;

/// <summary>
/// Abstraction for the rbd-ffi C-ABI operations.
/// Allows production execution via NativeRbdApi (P/Invoke to rbd_ffi.dll)
/// or testing/mock execution via MockNativeRbdApi without requiring native build artifacts.
/// </summary>
public interface INativeRbdApi
{
    int EngineInit(out IntPtr outEngine);
    void EngineFree(IntPtr engine);

    int DocumentOpenFile(SafeEngineHandle engine, string filePath, out IntPtr outDoc);
    int DocumentOpenMemory(SafeEngineHandle engine, IntPtr bytes, nuint length, out IntPtr outDoc);
    void DocumentFree(IntPtr doc);

    int DocumentGetPageCount(SafeDocumentHandle doc, out int outCount);
    int DocumentGetPageSize(SafeDocumentHandle doc, int pageIndex, out double outWidth, out double outHeight);

    int RenderTileIntoBuffer(
        SafeDocumentHandle doc,
        int pageIndex,
        int tileX,
        int tileY,
        int tileWidth,
        int tileHeight,
        double zoom,
        IntPtr buffer,
        int stride,
        int pixelFormat);

    string GetLastError();
    string GetVersionString();

    int DocumentExtractPageText(SafeDocumentHandle doc, uint pageIndex, out string text);
    int DocumentSearchJson(SafeDocumentHandle doc, string query, bool caseSensitive, out string json);

    int DocumentGetAnnotationsJson(SafeDocumentHandle doc, uint pageIndex, out string json);
    int DocumentAddAnnotationJson(SafeDocumentHandle doc, string annotJson);
    int DocumentRemoveAnnotation(SafeDocumentHandle doc, ulong annotId);
    int DocumentUpdateAnnotationJson(SafeDocumentHandle doc, string annotJson);

    int DocumentUndo(SafeDocumentHandle doc, out bool didUndo);
    int DocumentRedo(SafeDocumentHandle doc, out bool didRedo);
    int DocumentCanUndo(SafeDocumentHandle doc, out bool canUndo);
    int DocumentCanRedo(SafeDocumentHandle doc, out bool canRedo);

    int DocumentSaveAnnotations(SafeDocumentHandle doc, string outputPath);
    int DocumentSaveAnnotationsToPath(SafeDocumentHandle doc, string inputPath, string outputPath);

    // Standalone Page operations
    int PageOpRotate(string inPath, string outPath, int degrees);
    int PageOpDeletePages(string inPath, string outPath, string pagesJson);
    int PageOpSplit(string inPath, string outPath, uint startPage, uint endPage);
    int PageOpMerge(string outPath, string inPathsJson);
    int PageOpReorder(string inPath, string outPath, string orderJson);
    int PageOpCrop(string inPath, string outPath, uint pageNum, float x0, float y0, float x1, float y1);
    int PageOpInsertBlank(string inPath, string outPath, uint afterPage, float widthPt, float heightPt);
    int PageOpDecrypt(string inPath, string outPath, string passwordUtf8);
}
