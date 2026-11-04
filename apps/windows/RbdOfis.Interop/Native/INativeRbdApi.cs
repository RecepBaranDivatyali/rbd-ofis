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
}
