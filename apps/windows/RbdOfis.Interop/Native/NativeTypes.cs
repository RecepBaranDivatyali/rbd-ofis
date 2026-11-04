using System;

namespace RbdOfis.Interop.Native;

/// <summary>
/// Pixel format for tile rendering buffers.
/// </summary>
public enum RbdPixelFormat : int
{
    /// <summary>
    /// 32-bit BGRA (Blue, Green, Red, Alpha), standard for Direct2D, GDI, and WPF backbuffers.
    /// </summary>
    Bgra32 = 0,

    /// <summary>
    /// 32-bit RGBA (Red, Green, Blue, Alpha).
    /// </summary>
    Rgba32 = 1
}

/// <summary>
/// Status codes returned by the native rbd_ffi C-ABI library.
/// </summary>
public enum RbdStatusCode : int
{
    Success = 0,
    Ok = 0,
    ErrorNullPointer = -1,
    ErrorInvalidArgument = -2,
    ErrorFileIo = -3,
    ErrorIo = -3,
    ErrorCorruptPdf = -4,
    ErrorUnsupportedFormat = -4,
    ErrorPasswordRequired = -5,
    ErrorPageOutOfRange = -6,
    ErrorBufferTooSmall = -7,
    ErrorEngineFailure = -8,
    ErrorRenderFailed = -8,
    ErrorOutOfMemory = -6,
    ErrorPanic = -99
}

/// <summary>
/// Exception thrown when native rbd-ffi C-ABI operation fails.
/// </summary>
public sealed class RbdNativeException : Exception
{
    public RbdStatusCode StatusCode { get; }

    public RbdNativeException(RbdStatusCode statusCode, string message)
        : base($"[rbd-ffi {statusCode}] {message}")
    {
        StatusCode = statusCode;
    }

    public RbdNativeException(int statusCode, string message)
        : this((RbdStatusCode)statusCode, message)
    {
    }
}
