using System;
using System.Runtime.InteropServices;
using System.Text;

namespace RbdOfis.Interop.Native;

/// <summary>
/// Fully self-contained mock implementation of the rbd-ffi C-ABI contract.
/// Allows verifying zero-copy pinned buffer rendering, handle lifecycles, and viewport rendering
/// in unit tests and automated test runners without requiring rbd_ffi.dll to be pre-built.
/// </summary>
public sealed class MockNativeRbdApi : INativeRbdApi
{
    private static int _handleCounter = 0x1000;

    public int ConfiguredPageCount { get; set; } = 2000;
    public double ConfiguredPageWidth { get; set; } = 595.0; // Standard A4 width in pt
    public double ConfiguredPageHeight { get; set; } = 842.0; // Standard A4 height in pt

    public int RenderCallCount { get; private set; }
    public IntPtr LastRenderedBuffer { get; private set; }
    public int LastStride { get; private set; }
    public int LastTileWidth { get; private set; }
    public int LastTileHeight { get; private set; }

    public int EngineInit(out IntPtr outEngine)
    {
        nint handle = System.Threading.Interlocked.Increment(ref _handleCounter);
        outEngine = new IntPtr(handle);
        return (int)RbdStatusCode.Success;
    }

    public void EngineFree(IntPtr engine)
    {
        // Handle released
    }

    public int DocumentOpenFile(SafeEngineHandle engine, string filePath, out IntPtr outDoc)
    {
        if (engine.IsInvalid)
        {
            outDoc = IntPtr.Zero;
            return (int)RbdStatusCode.ErrorNullPointer;
        }

        nint handle = System.Threading.Interlocked.Increment(ref _handleCounter);
        outDoc = new IntPtr(handle);
        return (int)RbdStatusCode.Success;
    }

    public int DocumentOpenMemory(SafeEngineHandle engine, IntPtr bytes, nuint length, out IntPtr outDoc)
    {
        if (engine.IsInvalid || bytes == IntPtr.Zero || length == 0)
        {
            outDoc = IntPtr.Zero;
            return (int)RbdStatusCode.ErrorNullPointer;
        }

        nint handle = System.Threading.Interlocked.Increment(ref _handleCounter);
        outDoc = new IntPtr(handle);
        return (int)RbdStatusCode.Success;
    }

    public void DocumentFree(IntPtr doc)
    {
        // Document handle released
    }

    public int DocumentGetPageCount(SafeDocumentHandle doc, out int outCount)
    {
        if (doc.IsInvalid)
        {
            outCount = 0;
            return (int)RbdStatusCode.ErrorNullPointer;
        }

        outCount = ConfiguredPageCount;
        return (int)RbdStatusCode.Success;
    }

    public int DocumentGetPageSize(SafeDocumentHandle doc, int pageIndex, out double outWidth, out double outHeight)
    {
        if (doc.IsInvalid || pageIndex < 0 || pageIndex >= ConfiguredPageCount)
        {
            outWidth = 0;
            outHeight = 0;
            return (int)RbdStatusCode.ErrorInvalidArgument;
        }

        outWidth = ConfiguredPageWidth;
        outHeight = ConfiguredPageHeight;
        return (int)RbdStatusCode.Success;
    }

    public unsafe int RenderTileIntoBuffer(
        SafeDocumentHandle doc,
        int pageIndex,
        int tileX,
        int tileY,
        int tileWidth,
        int tileHeight,
        double zoom,
        IntPtr buffer,
        int stride,
        int pixelFormat)
    {
        if (doc.IsInvalid || buffer == IntPtr.Zero)
        {
            return (int)RbdStatusCode.ErrorNullPointer;
        }

        if (tileWidth <= 0 || tileHeight <= 0 || stride < tileWidth * 4)
        {
            return (int)RbdStatusCode.ErrorInvalidArgument;
        }

        RenderCallCount++;
        LastRenderedBuffer = buffer;
        LastStride = stride;
        LastTileWidth = tileWidth;
        LastTileHeight = tileHeight;

        // Zero-copy direct pointer write: fill pixel data directly into the provided unmanaged buffer address
        byte* basePtr = (byte*)buffer.ToPointer();
        byte r = (byte)((pageIndex * 37 + tileX * 13) % 256);
        byte g = (byte)((pageIndex * 53 + tileY * 29) % 256);
        byte b = (byte)(240); // Document paper-like tone
        byte a = 255;

        for (int y = 0; y < tileHeight; y++)
        {
            uint* row = (uint*)(basePtr + y * stride);
            for (int x = 0; x < tileWidth; x++)
            {
                // Format: Bgra32 (0) vs Rgba32 (1)
                uint pixel = (pixelFormat == (int)RbdPixelFormat.Bgra32)
                    ? (uint)(b | (g << 8) | (r << 16) | (a << 24))
                    : (uint)(r | (g << 8) | (b << 16) | (a << 24));

                row[x] = pixel;
            }
        }

        return (int)RbdStatusCode.Success;
    }

    public string GetLastError() => "No error";

    public string GetVersionString() => "rbd-mock-0.1.0";
}
