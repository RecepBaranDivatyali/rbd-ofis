using System;

namespace RbdOfis.Interop.Native;

/// <summary>
/// Managed wrapper for the RBD Core Engine instance.
/// Responsible for initializing native engine state and opening documents.
/// </summary>
public sealed class RbdEngine : IDisposable
{
    private readonly SafeEngineHandle _handle;
    private readonly INativeRbdApi _api;
    private bool _disposed;

    public SafeEngineHandle Handle => _handle;
    public INativeRbdApi Api => _api;

    public RbdEngine(INativeRbdApi? api = null)
    {
        _api = api ?? NativeRbdApi.Instance;
        int status = _api.EngineInit(out IntPtr rawHandle);
        if (status != (int)RbdStatusCode.Success || rawHandle == IntPtr.Zero)
        {
            throw new RbdNativeException(status, "Failed to initialize native RBD engine.");
        }

        _handle = new SafeEngineHandle(rawHandle, _api.EngineFree);
    }

    /// <summary>
    /// Opens a document from the local filesystem path.
    /// </summary>
    public RbdDocument OpenDocument(string filePath)
    {
        ThrowIfDisposed();
        ArgumentException.ThrowIfNullOrWhiteSpace(filePath);

        int status = _api.DocumentOpenFile(_handle, filePath, out IntPtr rawDoc);
        if (status != (int)RbdStatusCode.Success || rawDoc == IntPtr.Zero)
        {
            throw new RbdNativeException(status, $"Failed to open document: '{filePath}'.");
        }

        var docHandle = new SafeDocumentHandle(rawDoc, _api.DocumentFree);
        return new RbdDocument(docHandle, _api);
    }

    /// <summary>
    /// Opens a document directly from an unmanaged memory buffer.
    /// </summary>
    public RbdDocument OpenDocument(IntPtr memoryPtr, nuint length)
    {
        ThrowIfDisposed();
        if (memoryPtr == IntPtr.Zero || length == 0)
        {
            throw new ArgumentException("Invalid memory pointer or length.", nameof(memoryPtr));
        }

        int status = _api.DocumentOpenMemory(_handle, memoryPtr, length, out IntPtr rawDoc);
        if (status != (int)RbdStatusCode.Success || rawDoc == IntPtr.Zero)
        {
            throw new RbdNativeException(status, "Failed to open document from native memory.");
        }

        var docHandle = new SafeDocumentHandle(rawDoc, _api.DocumentFree);
        return new RbdDocument(docHandle, _api);
    }

    /// <summary>
    /// Opens a document from a managed byte array using pinned pointer arithmetic (zero-copy).
    /// </summary>
    public unsafe RbdDocument OpenDocument(byte[] bytes)
    {
        ArgumentNullException.ThrowIfNull(bytes);
        if (bytes.Length == 0)
        {
            throw new ArgumentException("Byte array is empty.", nameof(bytes));
        }

        fixed (byte* ptr = bytes)
        {
            return OpenDocument((IntPtr)ptr, (nuint)bytes.Length);
        }
    }

    /// <summary>
    /// Opens a document from a ReadOnlySpan of byte using pinned pointer arithmetic.
    /// </summary>
    public unsafe RbdDocument OpenDocument(ReadOnlySpan<byte> bytes)
    {
        if (bytes.IsEmpty)
        {
            throw new ArgumentException("Byte span is empty.", nameof(bytes));
        }

        fixed (byte* ptr = bytes)
        {
            return OpenDocument((IntPtr)ptr, (nuint)bytes.Length);
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
