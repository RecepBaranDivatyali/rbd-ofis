using System;
using System.IO;
using System.Reflection;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace RbdOfis.Interop.Native;

/// <summary>
/// Direct C-ABI P/Invoke declarations for rbd_ffi native dynamic library.
/// Uses .NET 9 source-generated [LibraryImport] for zero-overhead native calling.
/// </summary>
internal static partial class NativeMethods
{
    public const string LibraryName = "rbd_ffi";

    static NativeMethods()
    {
        NativeLibrary.SetDllImportResolver(typeof(NativeMethods).Assembly, ResolveDll);
    }

    private static IntPtr ResolveDll(string libraryName, Assembly assembly, DllImportSearchPath? searchPath)
    {
        if (libraryName != LibraryName)
        {
            return IntPtr.Zero;
        }

        // Try standard resolution first
        if (NativeLibrary.TryLoad(LibraryName, assembly, searchPath, out IntPtr handle))
        {
            return handle;
        }

        // Probing paths relative to current base directory
        string baseDir = AppContext.BaseDirectory;
        string[] probeLocations =
        [
            Path.Combine(baseDir, $"{LibraryName}.dll"),
            Path.Combine(baseDir, "runtimes", "win-x64", "native", $"{LibraryName}.dll"),
            Path.Combine(baseDir, "..", "..", "..", "..", "..", "core", "target", "release", $"{LibraryName}.dll"),
            Path.Combine(baseDir, "..", "..", "..", "..", "..", "core", "target", "debug", $"{LibraryName}.dll"),
            Path.Combine(baseDir, "..", "..", "..", "..", "..", "target", "release", $"{LibraryName}.dll"),
            Path.Combine(baseDir, "..", "..", "..", "..", "..", "target", "debug", $"{LibraryName}.dll"),
            Path.Combine(baseDir, "..", "..", "..", "..", "..", "..", "core", "target", "release", $"{LibraryName}.dll"),
            Path.Combine(baseDir, "..", "..", "..", "..", "..", "..", "core", "target", "debug", $"{LibraryName}.dll"),
            Path.Combine(baseDir, "..", "..", "..", "..", "..", "..", "target", "release", $"{LibraryName}.dll"),
            Path.Combine(baseDir, "..", "..", "..", "..", "..", "..", "target", "debug", $"{LibraryName}.dll")
        ];

        foreach (string path in probeLocations)
        {
            string fullPath = Path.GetFullPath(path);
            if (File.Exists(fullPath) && NativeLibrary.TryLoad(fullPath, out handle))
            {
                return handle;
            }
        }

        return IntPtr.Zero;
    }

    [LibraryImport(LibraryName, EntryPoint = "rbd_engine_init")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial int rbd_engine_init(out IntPtr outEngine);

    [LibraryImport(LibraryName, EntryPoint = "rbd_engine_free")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial void rbd_engine_free(IntPtr engine);

    [LibraryImport(LibraryName, EntryPoint = "rbd_document_open_file", StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial int rbd_document_open_file(SafeEngineHandle engine, string filePath, out IntPtr outDoc);

    [LibraryImport(LibraryName, EntryPoint = "rbd_document_open_memory")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial int rbd_document_open_memory(SafeEngineHandle engine, IntPtr bytes, nuint length, out IntPtr outDoc);

    [LibraryImport(LibraryName, EntryPoint = "rbd_document_free")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial void rbd_document_free(IntPtr doc);

    [LibraryImport(LibraryName, EntryPoint = "rbd_document_get_page_count")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial int rbd_document_get_page_count(SafeDocumentHandle doc, out int outCount);

    [LibraryImport(LibraryName, EntryPoint = "rbd_document_get_page_size")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial int rbd_document_get_page_size(SafeDocumentHandle doc, int pageIndex, out double outWidth, out double outHeight);

    [LibraryImport(LibraryName, EntryPoint = "rbd_render_tile_into_buffer")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial int rbd_render_tile_into_buffer(
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

    [LibraryImport(LibraryName, EntryPoint = "rbd_get_last_error")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial IntPtr rbd_get_last_error();

    [LibraryImport(LibraryName, EntryPoint = "rbd_version_string")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial IntPtr rbd_version_string();
}
