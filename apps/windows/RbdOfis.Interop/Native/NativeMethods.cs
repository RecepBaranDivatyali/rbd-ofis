using System;
using System.IO;
using System.Reflection;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace RbdOfis.Interop.Native;

/// <summary>
/// Direct C-ABI P/Invoke declarations for rbd_ffi native dynamic library.
/// Uses .NET 9 source-generated [LibraryImport] and [DllImport] for zero-overhead native calling.
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
        var probeLocations = new System.Collections.Generic.List<string>
        {
            Path.Combine(baseDir, $"{LibraryName}.dll"),
            Path.Combine(baseDir, "runtimes", "win-x64", "native", $"{LibraryName}.dll")
        };

        string? current = baseDir;
        for (int i = 0; i < 8 && current != null; i++)
        {
            probeLocations.Add(Path.Combine(current, "core", "target", "release", $"{LibraryName}.dll"));
            probeLocations.Add(Path.Combine(current, "core", "target", "debug", $"{LibraryName}.dll"));
            probeLocations.Add(Path.Combine(current, "target", "release", $"{LibraryName}.dll"));
            probeLocations.Add(Path.Combine(current, "target", "debug", $"{LibraryName}.dll"));
            current = Directory.GetParent(current)?.FullName;
        }

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

    [LibraryImport(LibraryName, EntryPoint = "rbd_free_string")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial int rbd_free_string(IntPtr textPtr);

    [LibraryImport(LibraryName, EntryPoint = "rbd_document_extract_page_text")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial int rbd_document_extract_page_text(SafeDocumentHandle doc, uint pageIndex, out IntPtr outUtf8Ptr);

    [LibraryImport(LibraryName, EntryPoint = "rbd_document_search_json", StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial int rbd_document_search_json(SafeDocumentHandle doc, string queryUtf8, int caseSensitive, out IntPtr outJsonUtf8Ptr);

    [LibraryImport(LibraryName, EntryPoint = "rbd_document_get_annotations_json")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial int rbd_document_get_annotations_json(SafeDocumentHandle doc, uint pageIndex, out IntPtr outJsonUtf8Ptr);

    [LibraryImport(LibraryName, EntryPoint = "rbd_document_add_annotation_json", StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial int rbd_document_add_annotation_json(SafeDocumentHandle doc, string annotJsonUtf8);

    [LibraryImport(LibraryName, EntryPoint = "rbd_document_remove_annotation")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial int rbd_document_remove_annotation(SafeDocumentHandle doc, ulong annotId);

    [LibraryImport(LibraryName, EntryPoint = "rbd_document_update_annotation_json", StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial int rbd_document_update_annotation_json(SafeDocumentHandle doc, string annotJsonUtf8);

    [LibraryImport(LibraryName, EntryPoint = "rbd_document_undo")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial int rbd_document_undo(SafeDocumentHandle doc, out int outDidUndo);

    [LibraryImport(LibraryName, EntryPoint = "rbd_document_redo")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial int rbd_document_redo(SafeDocumentHandle doc, out int outDidRedo);

    [LibraryImport(LibraryName, EntryPoint = "rbd_document_can_undo")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial int rbd_document_can_undo(SafeDocumentHandle doc, out int outCanUndo);

    [LibraryImport(LibraryName, EntryPoint = "rbd_document_can_redo")]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial int rbd_document_can_redo(SafeDocumentHandle doc, out int outCanRedo);

    [LibraryImport(LibraryName, EntryPoint = "rbd_document_save_annotations", StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial int rbd_document_save_annotations(SafeDocumentHandle doc, string outputPathUtf8);

    [LibraryImport(LibraryName, EntryPoint = "rbd_document_save_annotations_to_path", StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial int rbd_document_save_annotations_to_path(SafeDocumentHandle doc, string inputPathUtf8, string outputPathUtf8);

    // Standalone Page operations
    [LibraryImport(LibraryName, EntryPoint = "rbd_page_op_rotate", StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial int rbd_page_op_rotate(string inPath, string outPath, int degrees);

    [LibraryImport(LibraryName, EntryPoint = "rbd_page_op_delete_pages", StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial int rbd_page_op_delete_pages(string inPath, string outPath, string pagesJson);

    [LibraryImport(LibraryName, EntryPoint = "rbd_page_op_split", StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial int rbd_page_op_split(string inPath, string outPath, uint startPage, uint endPage);

    [LibraryImport(LibraryName, EntryPoint = "rbd_page_op_merge", StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial int rbd_page_op_merge(string outPath, string inPathsJson);

    [LibraryImport(LibraryName, EntryPoint = "rbd_page_op_reorder", StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial int rbd_page_op_reorder(string inPath, string outPath, string orderJson);

    [LibraryImport(LibraryName, EntryPoint = "rbd_page_op_crop", StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial int rbd_page_op_crop(string inPath, string outPath, uint pageNum, float x0, float y0, float x1, float y1);

    [LibraryImport(LibraryName, EntryPoint = "rbd_page_op_insert_blank", StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial int rbd_page_op_insert_blank(string inPath, string outPath, uint afterPage, float widthPt, float heightPt);

    [LibraryImport(LibraryName, EntryPoint = "rbd_page_op_decrypt", StringMarshalling = StringMarshalling.Utf8)]
    [UnmanagedCallConv(CallConvs = [typeof(CallConvCdecl)])]
    public static partial int rbd_page_op_decrypt(string inPath, string outPath, string passwordUtf8);
}
