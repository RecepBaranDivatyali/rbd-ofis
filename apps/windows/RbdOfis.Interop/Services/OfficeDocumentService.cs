using System;
using System.IO;
using System.Text.Json;
using RbdOfis.Interop.Models;
using RbdOfis.Interop.Native;

namespace RbdOfis.Interop.Services;

/// <summary>
/// High-level managed service for Office document operations (DOCX, XLSX, PPTX).
/// Provides text extraction, metadata inspection, and DOCX generation backed by the Rust core.
/// </summary>
public sealed class OfficeDocumentService
{
    private readonly INativeRbdApi _api;

    public OfficeDocumentService(INativeRbdApi? api = null)
    {
        _api = api ?? NativeRbdApi.Instance;
    }

    /// <summary>
    /// Checks if a file is a supported Office document format (.docx, .doc, .xlsx, .xls, .pptx, .ppt).
    /// </summary>
    public static bool IsSupportedOfficeFile(string path)
    {
        if (string.IsNullOrWhiteSpace(path)) return false;
        string ext = Path.GetExtension(path).ToLowerInvariant();
        return ext is ".docx" or ".doc" or ".xlsx" or ".xls" or ".pptx" or ".ppt";
    }

    /// <summary>
    /// Extracts plain text from an Office document file.
    /// </summary>
    public string ExtractText(string path)
    {
        if (string.IsNullOrWhiteSpace(path))
            throw new ArgumentException("Path cannot be null or empty.", nameof(path));
        if (!File.Exists(path))
            throw new FileNotFoundException($"Office file not found: {path}", path);

        int status = _api.OfficeExtractText(path, out string text);
        if (status != (int)RbdStatusCode.Success)
        {
            throw new InvalidOperationException($"Office text extraction failed with status code {status}.");
        }
        return text;
    }

    /// <summary>
    /// Inspects an Office document and returns structured metadata.
    /// </summary>
    public OfficeDocumentInfoDto Inspect(string path)
    {
        if (string.IsNullOrWhiteSpace(path))
            throw new ArgumentException("Path cannot be null or empty.", nameof(path));
        if (!File.Exists(path))
            throw new FileNotFoundException($"Office file not found: {path}", path);

        int status = _api.OfficeInspect(path, out string json);
        if (status != (int)RbdStatusCode.Success)
        {
            throw new InvalidOperationException($"Office document inspection failed with status code {status}.");
        }

        return JsonSerializer.Deserialize<OfficeDocumentInfoDto>(json)
               ?? new OfficeDocumentInfoDto { Title = Path.GetFileNameWithoutExtension(path) };
    }

    /// <summary>
    /// Creates a minimal, standard-compliant DOCX file from raw text.
    /// </summary>
    public void CreateDocx(string text, string outputPath)
    {
        if (string.IsNullOrWhiteSpace(outputPath))
            throw new ArgumentException("Output path cannot be null or empty.", nameof(outputPath));

        string? dir = Path.GetDirectoryName(outputPath);
        if (!string.IsNullOrEmpty(dir))
        {
            Directory.CreateDirectory(dir);
        }

        int status = _api.OfficeCreateDocx(text ?? string.Empty, outputPath);
        if (status != (int)RbdStatusCode.Success)
        {
            throw new InvalidOperationException($"DOCX creation failed with status code {status}.");
        }
    }
}
