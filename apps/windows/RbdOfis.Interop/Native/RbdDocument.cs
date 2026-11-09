using System;
using System.Collections.Generic;
using System.Text.Json;
using RbdOfis.Interop.Models;

namespace RbdOfis.Interop.Native;

/// <summary>
/// Managed wrapper for an open RBD document instance.
/// Provides safe zero-copy pinned buffer rendering into native backbuffers,
/// text extraction, fast search, annotations, and undo/redo operations.
/// </summary>
public sealed class RbdDocument : IDisposable
{
    private readonly SafeDocumentHandle _handle;
    private readonly INativeRbdApi _api;
    private bool _disposed;

    public SafeDocumentHandle Handle => _handle;
    public string FilePath { get; set; } = string.Empty;
    public bool IsDisposed => _disposed;

    public RbdDocument(SafeDocumentHandle handle, INativeRbdApi api, string filePath = "")
    {
        _handle = handle ?? throw new ArgumentNullException(nameof(handle));
        _api = api ?? throw new ArgumentNullException(nameof(api));
        FilePath = filePath;
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
    /// Extract raw UTF-8 text from a specific 0-based page index.
    /// </summary>
    public string ExtractText(uint pageIndex)
    {
        ThrowIfDisposed();
        int status = _api.DocumentExtractPageText(_handle, pageIndex, out string text);
        if (status != (int)RbdStatusCode.Success)
        {
            throw new RbdNativeException(status, $"Failed to extract text from page {pageIndex}.");
        }
        return text;
    }

    /// <summary>
    /// Search for text across the document and return hits with bounding box coordinates for highlight rendering.
    /// </summary>
    public IReadOnlyList<SearchHitDto> Search(string query, bool caseSensitive = false)
    {
        ThrowIfDisposed();
        if (string.IsNullOrEmpty(query)) return Array.Empty<SearchHitDto>();

        int status = _api.DocumentSearchJson(_handle, query, caseSensitive, out string json);
        if (status != (int)RbdStatusCode.Success)
        {
            throw new RbdNativeException(status, $"Search failed for query '{query}'.");
        }

        try
        {
            var hits = JsonSerializer.Deserialize<List<SearchHitDto>>(json);
            return hits ?? (IReadOnlyList<SearchHitDto>)Array.Empty<SearchHitDto>();
        }
        catch
        {
            return Array.Empty<SearchHitDto>();
        }
    }

    /// <summary>
    /// Retrieve all annotations active on the specified 0-based page.
    /// </summary>
    public IReadOnlyList<AnnotationDto> GetAnnotations(uint pageIndex)
    {
        ThrowIfDisposed();
        int status = _api.DocumentGetAnnotationsJson(_handle, pageIndex, out string json);
        if (status != (int)RbdStatusCode.Success)
        {
            throw new RbdNativeException(status, $"Failed to retrieve annotations for page {pageIndex}.");
        }

        try
        {
            var annots = JsonSerializer.Deserialize<List<AnnotationDto>>(json);
            return annots ?? (IReadOnlyList<AnnotationDto>)Array.Empty<AnnotationDto>();
        }
        catch
        {
            return Array.Empty<AnnotationDto>();
        }
    }

    /// <summary>
    /// Add an annotation to the document with undo/redo support.
    /// </summary>
    public void AddAnnotation(AnnotationDto annotation)
    {
        ThrowIfDisposed();
        ArgumentNullException.ThrowIfNull(annotation);

        string json = JsonSerializer.Serialize(annotation);
        int status = _api.DocumentAddAnnotationJson(_handle, json);
        if (status != (int)RbdStatusCode.Success)
        {
            throw new RbdNativeException(status, $"Failed to add annotation: {_api.GetLastError()}");
        }
    }

    /// <summary>
    /// Remove an annotation by ID with undo/redo support.
    /// </summary>
    public void RemoveAnnotation(ulong annotId)
    {
        ThrowIfDisposed();
        int status = _api.DocumentRemoveAnnotation(_handle, annotId);
        if (status != (int)RbdStatusCode.Success)
        {
            throw new RbdNativeException(status, $"Failed to remove annotation with ID {annotId}: {_api.GetLastError()}");
        }
    }

    /// <summary>
    /// Update an existing annotation with undo/redo support.
    /// </summary>
    public void UpdateAnnotation(AnnotationDto annotation)
    {
        ThrowIfDisposed();
        ArgumentNullException.ThrowIfNull(annotation);

        string json = JsonSerializer.Serialize(annotation);
        int status = _api.DocumentUpdateAnnotationJson(_handle, json);
        if (status != (int)RbdStatusCode.Success)
        {
            throw new RbdNativeException(status, $"Failed to update annotation with ID {annotation.Id}: {_api.GetLastError()}");
        }
    }

    /// <summary>
    /// Undo the last annotation action. Returns true if an action was undone.
    /// </summary>
    public bool Undo()
    {
        ThrowIfDisposed();
        int status = _api.DocumentUndo(_handle, out bool didUndo);
        if (status != (int)RbdStatusCode.Success)
        {
            throw new RbdNativeException(status, "Undo operation failed.");
        }
        return didUndo;
    }

    /// <summary>
    /// Redo the last undone annotation action. Returns true if an action was redone.
    /// </summary>
    public bool Redo()
    {
        ThrowIfDisposed();
        int status = _api.DocumentRedo(_handle, out bool didRedo);
        if (status != (int)RbdStatusCode.Success)
        {
            throw new RbdNativeException(status, "Redo operation failed.");
        }
        return didRedo;
    }

    public bool CanUndo
    {
        get
        {
            if (_disposed) return false;
            _api.DocumentCanUndo(_handle, out bool canUndo);
            return canUndo;
        }
    }

    public bool CanRedo
    {
        get
        {
            if (_disposed) return false;
            _api.DocumentCanRedo(_handle, out bool canRedo);
            return canRedo;
        }
    }

    /// <summary>
    /// Embed all current session annotations and save to output PDF file using document's known source.
    /// </summary>
    public void SaveAnnotations(string outputPath)
    {
        ThrowIfDisposed();
        int status = _api.DocumentSaveAnnotations(_handle, outputPath);
        if (status != (int)RbdStatusCode.Success)
        {
            throw new RbdNativeException(status, $"Failed to save annotations to '{outputPath}'.");
        }
    }

    /// <summary>
    /// Embed all current session annotations specifying both input template path and output path.
    /// </summary>
    public void SaveAnnotations(string inputPath, string outputPath)
    {
        ThrowIfDisposed();
        int status = _api.DocumentSaveAnnotationsToPath(_handle, inputPath, outputPath);
        if (status != (int)RbdStatusCode.Success)
        {
            throw new RbdNativeException(status, $"Failed to save annotations from '{inputPath}' to '{outputPath}'.");
        }
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
