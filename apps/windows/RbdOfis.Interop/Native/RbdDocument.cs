using System;

namespace RbdOfis.Interop.Native;

/// <summary>
/// Managed wrapper for an open RBD document instance.
/// Provides safe zero-copy pinned buffer rendering into native backbuffers,
/// byte arrays, and memory spans without intermediate marshaling or copies.
/// </summary>
public sealed class RbdDocument : IDisposable
{
    private readonly SafeDocumentHandle _handle;
    private readonly INativeRbdApi _api;
    private bool _disposed;

    public SafeDocumentHandle Handle => _handle;

    public RbdDocument(SafeDocumentHandle handle, INativeRbdApi api)
    {
        _handle = handle ?? throw new ArgumentNullException(nameof(handle));
        _api = api ?? throw new ArgumentNullException(nameof(api));
    }

    /// <summary>
    /// Gets the total number of pages in the document.
    /// </summary>
    public int PageCount
    {
        get
        {
            ThrowIfDisposed();
            int status = _api.DocumentGetPageCount(_handle, out int count);
            if (status != (int)RbdStatusCode.Success)
            {
                throw new RbdNativeException(status, "Failed to get document page count.");
            }
            return count;
        }
    }

    /// <summary>
    /// Gets the page dimensions in points (pt) for the specified page.
    /// </summary>
    public (double Width, double Height) GetPageSize(int pageIndex)
    {
        ThrowIfDisposed();
        int status = _api.DocumentGetPageSize(_handle, pageIndex, out double width, out double height);
        if (status != (int)RbdStatusCode.Success)
        {
            throw new RbdNativeException(status, $"Failed to get page size for page {pageIndex}.");
        }
        return (width, height);
    }

    /// <summary>
    /// Renders a single tile directly into an unmanaged memory address (e.g., WriteableBitmap.BackBuffer).
    /// Pure zero-copy execution with no managed array allocations.
    /// </summary>
    public void RenderTileIntoBuffer(
        int pageIndex,
        int tileX,
        int tileY,
        int tileWidth,
        int tileHeight,
        double zoom,
        IntPtr backBufferPtr,
        int stride,
        RbdPixelFormat format = RbdPixelFormat.Bgra32)
    {
        ThrowIfDisposed();
        if (backBufferPtr == IntPtr.Zero)
        {
            throw new ArgumentNullException(nameof(backBufferPtr));
        }

        int status = _api.RenderTileIntoBuffer(
            _handle,
            pageIndex,
            tileX,
            tileY,
            tileWidth,
            tileHeight,
            zoom,
            backBufferPtr,
            stride,
            (int)format);

        if (status != (int)RbdStatusCode.Success)
        {
            throw new RbdNativeException(status, $"RenderTileIntoBuffer failed on page {pageIndex} tile ({tileX}, {tileY}).");
        }
    }

    /// <summary>
    /// Zero-copy pinned buffer rendering for managed byte arrays.
    /// Pins the managed array with fixed pointer arithmetic and passes IntPtr directly to the native C-ABI.
    /// </summary>
    public unsafe void RenderTileIntoBuffer(
        int pageIndex,
        int tileX,
        int tileY,
        int tileWidth,
        int tileHeight,
        double zoom,
        byte[] destinationBuffer,
        int stride,
        RbdPixelFormat format = RbdPixelFormat.Bgra32)
    {
        ArgumentNullException.ThrowIfNull(destinationBuffer);
        if (destinationBuffer.Length < stride * tileHeight)
        {
            throw new ArgumentException("Destination buffer size is smaller than required stride * tileHeight.", nameof(destinationBuffer));
        }

        fixed (byte* ptr = destinationBuffer)
        {
            RenderTileIntoBuffer(pageIndex, tileX, tileY, tileWidth, tileHeight, zoom, (IntPtr)ptr, stride, format);
        }
    }

    /// <summary>
    /// Zero-copy pinned buffer rendering for Span of byte.
    /// </summary>
    public unsafe void RenderTileIntoBuffer(
        int pageIndex,
        int tileX,
        int tileY,
        int tileWidth,
        int tileHeight,
        double zoom,
        Span<byte> destinationBuffer,
        int stride,
        RbdPixelFormat format = RbdPixelFormat.Bgra32)
    {
        if (destinationBuffer.Length < stride * tileHeight)
        {
            throw new ArgumentException("Destination span size is smaller than required stride * tileHeight.", nameof(destinationBuffer));
        }

        fixed (byte* ptr = destinationBuffer)
        {
            RenderTileIntoBuffer(pageIndex, tileX, tileY, tileWidth, tileHeight, zoom, (IntPtr)ptr, stride, format);
        }
    }

    private void ThrowIfDisposed()
    {
        ObjectDisposedException.ThrowIf(_disposed, this);
    }

    public void Dispose()
    {
        if (!_disposed)
        {
            _handle.Dispose();
            _disposed = true;
        }
    }
}
