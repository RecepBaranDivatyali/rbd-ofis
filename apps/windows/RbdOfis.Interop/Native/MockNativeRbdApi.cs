using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text.Json;
using RbdOfis.Interop.Models;

namespace RbdOfis.Interop.Native;

/// <summary>
/// Fully self-contained mock implementation of the rbd-ffi C-ABI contract.
/// Allows verifying zero-copy pinned buffer rendering, handle lifecycles, annotations, search, and page operations
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

    // In-memory annotation and undo/redo stacks per document
    private readonly Dictionary<IntPtr, List<AnnotationDto>> _annotations = new();
    private readonly Dictionary<IntPtr, Stack<List<AnnotationDto>>> _undoStack = new();
    private readonly Dictionary<IntPtr, Stack<List<AnnotationDto>>> _redoStack = new();

    private static List<AnnotationDto> CloneList(List<AnnotationDto> list) => new(list);

    public int EngineInit(out IntPtr outEngine)
    {
        int handle = System.Threading.Interlocked.Increment(ref _handleCounter);
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

        int handle = System.Threading.Interlocked.Increment(ref _handleCounter);
        outDoc = new IntPtr(handle);
        _annotations[outDoc] = new List<AnnotationDto>();
        _undoStack[outDoc] = new Stack<List<AnnotationDto>>();
        _redoStack[outDoc] = new Stack<List<AnnotationDto>>();
        return (int)RbdStatusCode.Success;
    }

    public int DocumentOpenMemory(SafeEngineHandle engine, IntPtr bytes, nuint length, out IntPtr outDoc)
    {
        if (engine.IsInvalid || bytes == IntPtr.Zero || length == 0)
        {
            outDoc = IntPtr.Zero;
            return (int)RbdStatusCode.ErrorNullPointer;
        }

        int handle = System.Threading.Interlocked.Increment(ref _handleCounter);
        outDoc = new IntPtr(handle);
        _annotations[outDoc] = new List<AnnotationDto>();
        _undoStack[outDoc] = new Stack<List<AnnotationDto>>();
        _redoStack[outDoc] = new Stack<List<AnnotationDto>>();
        return (int)RbdStatusCode.Success;
    }

    public void DocumentFree(IntPtr doc)
    {
        _annotations.Remove(doc);
        _undoStack.Remove(doc);
        _redoStack.Remove(doc);
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
        byte b = 240; // Document paper-like tone
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
    public string GetVersionString() => "rbd-mock-0.1.0-alpha";

    public int DocumentExtractPageText(SafeDocumentHandle doc, uint pageIndex, out string text)
    {
        if (doc.IsInvalid)
        {
            text = string.Empty;
            return (int)RbdStatusCode.ErrorNullPointer;
        }

        text = $"RBD OFİS Document Page {pageIndex + 1}\nHello world! Bu bir test PDF dökümanıdır. Türkçe karakterler: ç, ğ, ı, ö, ş, ü, İ.\nEnd of page {pageIndex + 1}.";
        return (int)RbdStatusCode.Success;
    }

    public int DocumentSearchJson(SafeDocumentHandle doc, string query, bool caseSensitive, out string json)
    {
        if (doc.IsInvalid || string.IsNullOrEmpty(query))
        {
            json = "[]";
            return (int)RbdStatusCode.Success;
        }

        var hits = new List<SearchHitDto>();
        // Return simulated hits across early pages
        int samplePages = Math.Min(ConfiguredPageCount, 5);
        for (uint p = 0; p < samplePages; p++)
        {
            hits.Add(new SearchHitDto
            {
                Page = p,
                Text = query,
                CharOffset = 25,
                Rects =
                [
                    new PdfRect(72f, 700f, 150f, 720f)
                ]
            });
        }

        json = JsonSerializer.Serialize(hits);
        return (int)RbdStatusCode.Success;
    }

    public int DocumentGetAnnotationsJson(SafeDocumentHandle doc, uint pageIndex, out string json)
    {
        IntPtr key = doc.DangerousGetHandle();
        if (_annotations.TryGetValue(key, out var list))
        {
            var pageAnnots = list.Where(a => a.PageIndex == pageIndex).ToList();
            json = JsonSerializer.Serialize(pageAnnots);
            return (int)RbdStatusCode.Success;
        }

        json = "[]";
        return (int)RbdStatusCode.Success;
    }

    public int DocumentAddAnnotationJson(SafeDocumentHandle doc, string annotJson)
    {
        if (string.IsNullOrWhiteSpace(annotJson)) return (int)RbdStatusCode.ErrorInvalidArgument;
        var annot = JsonSerializer.Deserialize<AnnotationDto>(annotJson);
        if (annot == null) return (int)RbdStatusCode.ErrorInvalidArgument;

        IntPtr key = doc.DangerousGetHandle();
        if (!_annotations.ContainsKey(key)) _annotations[key] = new List<AnnotationDto>();
        if (!_undoStack.ContainsKey(key)) _undoStack[key] = new Stack<List<AnnotationDto>>();
        if (!_redoStack.ContainsKey(key)) _redoStack[key] = new Stack<List<AnnotationDto>>();

        var list = _annotations[key];
        _undoStack[key].Push(CloneList(list));
        _redoStack[key].Clear();

        list.RemoveAll(a => a.Id == annot.Id);
        list.Add(annot);

        return (int)RbdStatusCode.Success;
    }

    public int DocumentRemoveAnnotation(SafeDocumentHandle doc, ulong annotId)
    {
        IntPtr key = doc.DangerousGetHandle();
        if (_annotations.TryGetValue(key, out var list))
        {
            if (!_undoStack.ContainsKey(key)) _undoStack[key] = new Stack<List<AnnotationDto>>();
            if (!_redoStack.ContainsKey(key)) _redoStack[key] = new Stack<List<AnnotationDto>>();

            var oldList = CloneList(list);
            int count = list.RemoveAll(a => a.Id == annotId);
            if (count > 0)
            {
                _undoStack[key].Push(oldList);
                _redoStack[key].Clear();
            }
        }
        return (int)RbdStatusCode.Success;
    }

    public int DocumentUpdateAnnotationJson(SafeDocumentHandle doc, string annotJson)
    {
        if (string.IsNullOrWhiteSpace(annotJson)) return (int)RbdStatusCode.ErrorInvalidArgument;
        var annot = JsonSerializer.Deserialize<AnnotationDto>(annotJson);
        if (annot == null) return (int)RbdStatusCode.ErrorInvalidArgument;

        IntPtr key = doc.DangerousGetHandle();
        if (_annotations.TryGetValue(key, out var list))
        {
            if (!_undoStack.ContainsKey(key)) _undoStack[key] = new Stack<List<AnnotationDto>>();
            if (!_redoStack.ContainsKey(key)) _redoStack[key] = new Stack<List<AnnotationDto>>();

            int idx = list.FindIndex(a => a.Id == annot.Id);
            if (idx >= 0)
            {
                _undoStack[key].Push(CloneList(list));
                _redoStack[key].Clear();
                list[idx] = annot;
                return (int)RbdStatusCode.Success;
            }
        }
        return DocumentAddAnnotationJson(doc, annotJson);
    }

    public int DocumentUndo(SafeDocumentHandle doc, out bool didUndo)
    {
        IntPtr key = doc.DangerousGetHandle();
        if (_undoStack.TryGetValue(key, out var uStack) && uStack.Count > 0)
        {
            if (!_redoStack.ContainsKey(key)) _redoStack[key] = new Stack<List<AnnotationDto>>();
            _redoStack[key].Push(CloneList(_annotations[key]));
            _annotations[key] = uStack.Pop();
            didUndo = true;
            return (int)RbdStatusCode.Success;
        }

        didUndo = false;
        return (int)RbdStatusCode.Success;
    }

    public int DocumentRedo(SafeDocumentHandle doc, out bool didRedo)
    {
        IntPtr key = doc.DangerousGetHandle();
        if (_redoStack.TryGetValue(key, out var rStack) && rStack.Count > 0)
        {
            if (!_undoStack.ContainsKey(key)) _undoStack[key] = new Stack<List<AnnotationDto>>();
            _undoStack[key].Push(CloneList(_annotations[key]));
            _annotations[key] = rStack.Pop();
            didRedo = true;
            return (int)RbdStatusCode.Success;
        }

        didRedo = false;
        return (int)RbdStatusCode.Success;
    }

    public int DocumentCanUndo(SafeDocumentHandle doc, out bool canUndo)
    {
        IntPtr key = doc.DangerousGetHandle();
        canUndo = _undoStack.TryGetValue(key, out var stack) && stack.Count > 0;
        return (int)RbdStatusCode.Success;
    }

    public int DocumentCanRedo(SafeDocumentHandle doc, out bool canRedo)
    {
        IntPtr key = doc.DangerousGetHandle();
        canRedo = _redoStack.TryGetValue(key, out var stack) && stack.Count > 0;
        return (int)RbdStatusCode.Success;
    }

    public int DocumentSaveAnnotations(SafeDocumentHandle doc, string outputPath)
    {
        File.WriteAllText(outputPath, "%PDF-1.7 mock output with annotations");
        return (int)RbdStatusCode.Success;
    }

    public int DocumentSaveAnnotationsToPath(SafeDocumentHandle doc, string inputPath, string outputPath)
    {
        if (File.Exists(inputPath))
        {
            File.Copy(inputPath, outputPath, overwrite: true);
        }
        else
        {
            File.WriteAllText(outputPath, "%PDF-1.7 mock output with annotations");
        }
        return (int)RbdStatusCode.Success;
    }

    // Page operations
    public int PageOpRotate(string inPath, string outPath, int degrees)
    {
        File.Copy(inPath, outPath, overwrite: true);
        return (int)RbdStatusCode.Success;
    }

    public int PageOpDeletePages(string inPath, string outPath, string pagesJson)
    {
        File.Copy(inPath, outPath, overwrite: true);
        return (int)RbdStatusCode.Success;
    }

    public int PageOpSplit(string inPath, string outPath, uint startPage, uint endPage)
    {
        File.Copy(inPath, outPath, overwrite: true);
        return (int)RbdStatusCode.Success;
    }

    public int PageOpMerge(string outPath, string inPathsJson)
    {
        try
        {
            var paths = JsonSerializer.Deserialize<List<string>>(inPathsJson);
            if (paths != null && paths.Count > 0 && File.Exists(paths[0]))
            {
                File.Copy(paths[0], outPath, overwrite: true);
                return (int)RbdStatusCode.Success;
            }
        }
        catch { }
        File.WriteAllText(outPath, "%PDF-1.7 merged mock");
        return (int)RbdStatusCode.Success;
    }

    public int PageOpReorder(string inPath, string outPath, string orderJson)
    {
        File.Copy(inPath, outPath, overwrite: true);
        return (int)RbdStatusCode.Success;
    }

    public int PageOpCrop(string inPath, string outPath, uint pageNum, float x0, float y0, float x1, float y1)
    {
        File.Copy(inPath, outPath, overwrite: true);
        return (int)RbdStatusCode.Success;
    }

    public int PageOpInsertBlank(string inPath, string outPath, uint afterPage, float widthPt, float heightPt)
    {
        File.Copy(inPath, outPath, overwrite: true);
        return (int)RbdStatusCode.Success;
    }

    public int PageOpDecrypt(string inPath, string outPath, string passwordUtf8)
    {
        File.Copy(inPath, outPath, overwrite: true);
        return (int)RbdStatusCode.Success;
    }

    // Professional PDF Tools (Phase 4 Track 4B/4C)
    public int ProRedactDocument(string inPath, string outPath, string redactionsJson)
    {
        if (File.Exists(inPath)) File.Copy(inPath, outPath, overwrite: true);
        else File.WriteAllText(outPath, "%PDF-1.7 mock redacted");
        return (int)RbdStatusCode.Success;
    }

    public int ProCompressDocument(string inPath, string outPath, bool removeUnused, bool recompress, bool stripMeta, out string savingsJson)
    {
        long inSize = File.Exists(inPath) ? new FileInfo(inPath).Length : 10000;
        if (File.Exists(inPath)) File.Copy(inPath, outPath, overwrite: true);
        else File.WriteAllText(outPath, "%PDF-1.7 mock compressed");
        long outSize = (long)(inSize * 0.7);
        savingsJson = $"{{\"original_size\":{inSize},\"compressed_size\":{outSize},\"savings_percent\":30.0}}";
        return (int)RbdStatusCode.Success;
    }

    public int ProApplyWatermark(string inPath, string outPath, string text, float fontSize, float rotationDeg, float opacity, byte r, byte g, byte b, bool onTop)
    {
        if (File.Exists(inPath)) File.Copy(inPath, outPath, overwrite: true);
        else File.WriteAllText(outPath, "%PDF-1.7 mock watermarked");
        return (int)RbdStatusCode.Success;
    }

    public int ProApplyPageNumbers(string inPath, string outPath, string pattern, int position, float fontSize, float margin)
    {
        if (File.Exists(inPath)) File.Copy(inPath, outPath, overwrite: true);
        else File.WriteAllText(outPath, "%PDF-1.7 mock numbered");
        return (int)RbdStatusCode.Success;
    }

    public int ProApplyBatesStamp(string inPath, string outPath, string prefix, uint startNumber, uint paddingDigits, string suffix, int position)
    {
        if (File.Exists(inPath)) File.Copy(inPath, outPath, overwrite: true);
        else File.WriteAllText(outPath, "%PDF-1.7 mock bates");
        return (int)RbdStatusCode.Success;
    }

    public int ProReadFormFields(string inPath, out string fieldsJson)
    {
        fieldsJson = "[{\"name\":\"sample_field\",\"field_type\":\"Text\",\"value\":\"Demo Value\",\"read_only\":false}]";
        return (int)RbdStatusCode.Success;
    }

    public int ProFillFormFields(string inPath, string outPath, string fieldValuesJson, bool flatten)
    {
        if (File.Exists(inPath)) File.Copy(inPath, outPath, overwrite: true);
        else File.WriteAllText(outPath, "%PDF-1.7 mock form filled");
        return (int)RbdStatusCode.Success;
    }

    // Document Converter (Phase 5 Track 5B/5C)
    public int ConvertPdfToImages(string inPath, string outDir, uint dpi, int format, string? pageRangeJson, out List<string> generatedPaths)
    {
        generatedPaths = [];
        if (string.IsNullOrWhiteSpace(inPath) || string.IsNullOrWhiteSpace(outDir))
            return (int)RbdStatusCode.ErrorNullPointer;
        if (dpi == 0)
            return (int)RbdStatusCode.ErrorInvalidArgument;
        if (format != 0 && format != 1)
            return (int)RbdStatusCode.ErrorInvalidArgument;

        try
        {
            Directory.CreateDirectory(outDir);
            string ext = format == 0 ? "png" : "jpg";

            List<uint> pages = [0];
            if (!string.IsNullOrWhiteSpace(pageRangeJson))
            {
                try
                {
                    var parsed = JsonSerializer.Deserialize<List<uint>>(pageRangeJson);
                    if (parsed != null && parsed.Count > 0) pages = parsed;
                }
                catch { }
            }

            foreach (var page in pages)
            {
                string path = Path.Combine(outDir, $"page_{page}.{ext}");
                File.WriteAllText(path, $"mock image content for page {page}");
                generatedPaths.Add(path);
            }

            return (int)RbdStatusCode.Success;
        }
        catch
        {
            return (int)RbdStatusCode.ErrorEngineFailure;
        }
    }

    public int ConvertImagesToPdf(string imagesJson, int pageSize, float marginPt, string outPath)
    {
        if (string.IsNullOrWhiteSpace(imagesJson) || string.IsNullOrWhiteSpace(outPath))
            return (int)RbdStatusCode.ErrorNullPointer;
        if (marginPt < 0.0f)
            return (int)RbdStatusCode.ErrorInvalidArgument;
        if (pageSize < 0 || pageSize > 2)
            return (int)RbdStatusCode.ErrorInvalidArgument;

        try
        {
            var images = JsonSerializer.Deserialize<List<string>>(imagesJson);
            if (images == null || images.Count == 0)
                return (int)RbdStatusCode.ErrorInvalidArgument;

            File.WriteAllText(outPath, "%PDF-1.7 mock converted from images");
            return (int)RbdStatusCode.Success;
        }
        catch
        {
            return (int)RbdStatusCode.ErrorInvalidArgument;
        }
    }

    public int ConvertMarkdownToPdf(string markdownText, string outPath)
    {
        if (markdownText == null || string.IsNullOrWhiteSpace(outPath))
            return (int)RbdStatusCode.ErrorNullPointer;

        try
        {
            File.WriteAllText(outPath, $"%PDF-1.7 mock converted from markdown:\n{markdownText}");
            return (int)RbdStatusCode.Success;
        }
        catch
        {
            return (int)RbdStatusCode.ErrorEngineFailure;
        }
    }

    public int OfficeExtractText(string inPath, out string text)
    {
        if (string.IsNullOrWhiteSpace(inPath))
        {
            text = string.Empty;
            return (int)RbdStatusCode.ErrorNullPointer;
        }

        if (!File.Exists(inPath))
        {
            text = string.Empty;
            return (int)RbdStatusCode.ErrorFileIo;
        }

        try
        {
            text = File.ReadAllText(inPath);
            return (int)RbdStatusCode.Success;
        }
        catch
        {
            text = string.Empty;
            return (int)RbdStatusCode.ErrorFileIo;
        }
    }

    public int OfficeInspect(string inPath, out string metadataJson)
    {
        if (string.IsNullOrWhiteSpace(inPath))
        {
            metadataJson = "{}";
            return (int)RbdStatusCode.ErrorNullPointer;
        }

        if (!File.Exists(inPath))
        {
            metadataJson = "{}";
            return (int)RbdStatusCode.ErrorFileIo;
        }

        string ext = Path.GetExtension(inPath).TrimStart('.').ToUpperInvariant();
        metadataJson = JsonSerializer.Serialize(new
        {
            doc_type = ext,
            title = Path.GetFileNameWithoutExtension(inPath),
            word_count = 120,
            item_count = 5,
            summary = "Mock office document summary"
        });
        return (int)RbdStatusCode.Success;
    }

    public int OfficeCreateDocx(string text, string outPath)
    {
        if (text == null || string.IsNullOrWhiteSpace(outPath))
            return (int)RbdStatusCode.ErrorNullPointer;

        try
        {
            File.WriteAllText(outPath, $"MOCK_DOCX_CONTENT:\n{text}");
            return (int)RbdStatusCode.Success;
        }
        catch
        {
            return (int)RbdStatusCode.ErrorFileIo;
        }
    }
}
