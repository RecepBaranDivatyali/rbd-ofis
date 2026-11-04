using System;

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
        return ptr != IntPtr.Zero ? System.Runtime.InteropServices.Marshal.PtrToStringUTF8(ptr) ?? string.Empty : string.Empty;
    }

    public string GetVersionString()
    {
        IntPtr ptr = NativeMethods.rbd_version_string();
        return ptr != IntPtr.Zero ? System.Runtime.InteropServices.Marshal.PtrToStringUTF8(ptr) ?? string.Empty : string.Empty;
    }
}
