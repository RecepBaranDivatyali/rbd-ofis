using System;
using Microsoft.Win32.SafeHandles;

namespace RbdOfis.Interop.Native;

/// <summary>
/// SafeHandle wrapper for unmanaged RbdEngine native pointer.
/// Ensures deterministic cleanup via rbd_engine_free.
/// </summary>
public sealed class SafeEngineHandle : SafeHandleZeroOrMinusOneIsInvalid
{
    private readonly Action<IntPtr>? _customRelease;

    public SafeEngineHandle() : base(ownsHandle: true)
    {
    }

    public SafeEngineHandle(IntPtr preexistingHandle, bool ownsHandle = true) : base(ownsHandle)
    {
        SetHandle(preexistingHandle);
    }

    public SafeEngineHandle(IntPtr preexistingHandle, Action<IntPtr> customRelease) : base(ownsHandle: true)
    {
        _customRelease = customRelease;
        SetHandle(preexistingHandle);
    }

    protected override bool ReleaseHandle()
    {
        if (!IsInvalid)
        {
            if (_customRelease != null)
            {
                _customRelease(handle);
            }
            else
            {
                NativeMethods.rbd_engine_free(handle);
            }
            return true;
        }
        return false;
    }
}

/// <summary>
/// SafeHandle wrapper for unmanaged RbdDocument native pointer.
/// Ensures deterministic cleanup via rbd_document_free.
/// </summary>
public sealed class SafeDocumentHandle : SafeHandleZeroOrMinusOneIsInvalid
{
    private readonly Action<IntPtr>? _customRelease;

    public SafeDocumentHandle() : base(ownsHandle: true)
    {
    }

    public SafeDocumentHandle(IntPtr preexistingHandle, bool ownsHandle = true) : base(ownsHandle)
    {
        SetHandle(preexistingHandle);
    }

    public SafeDocumentHandle(IntPtr preexistingHandle, Action<IntPtr> customRelease) : base(ownsHandle: true)
    {
        _customRelease = customRelease;
        SetHandle(preexistingHandle);
    }

    protected override bool ReleaseHandle()
    {
        if (!IsInvalid)
        {
            if (_customRelease != null)
            {
                _customRelease(handle);
            }
            else
            {
                NativeMethods.rbd_document_free(handle);
            }
            return true;
        }
        return false;
    }
}
