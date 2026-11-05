using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using RbdOfis.Interop.Models;
using RbdOfis.Interop.Native;

namespace RbdOfis.Interop.Services;

/// <summary>
/// High-level managed service for Document Conversion (Phase 5).
/// Provides PDF to Images, Images to PDF, and Markdown to PDF conversions.
/// </summary>
public sealed class DocumentConverterService
{
    private readonly INativeRbdApi _api;

    public DocumentConverterService(INativeRbdApi? api = null)
    {
        _api = api ?? NativeRbdApi.Instance;
    }

    /// <summary>
    /// Converts pages of a PDF document into individual PNG or JPEG images.
    /// </summary>
    /// <param name="inPath">Path to input PDF file.</param>
    /// <param name="outDir">Directory where generated image files will be saved.</param>
    /// <param name="options">Conversion options such as DPI, format, and optional page range.</param>
    /// <returns>List of generated image file paths.</returns>
    public List<string> ConvertPdfToImages(string inPath, string outDir, PdfToImageOptionsDto? options = null)
    {
        if (string.IsNullOrWhiteSpace(inPath))
            throw new ArgumentException("Input path cannot be null or empty.", nameof(inPath));
        if (!File.Exists(inPath))
            throw new FileNotFoundException($"Input PDF file not found: {inPath}", inPath);
        if (string.IsNullOrWhiteSpace(outDir))
            throw new ArgumentException("Output directory cannot be null or empty.", nameof(outDir));

        Directory.CreateDirectory(outDir);
        options ??= new PdfToImageOptionsDto();

        if (options.Dpi == 0)
            throw new ArgumentException("DPI must be greater than 0.", nameof(options));

        string? pageRangeJson = null;
        if (options.PageRange != null && options.PageRange.Count > 0)
        {
            pageRangeJson = JsonSerializer.Serialize(options.PageRange);
        }

        int status = _api.ConvertPdfToImages(
            inPath,
            outDir,
            options.Dpi,
            (int)options.Format,
            pageRangeJson,
            out var generatedPaths);

        if (status != (int)RbdStatusCode.Success)
        {
            throw new RbdNativeException(status, $"Failed to convert PDF to images: {_api.GetLastError()}");
        }

        return generatedPaths;
    }

    /// <summary>
    /// Converts a collection of image files into a single paginated PDF document.
    /// </summary>
    /// <param name="imagePaths">List of image file paths to include.</param>
    /// <param name="outPath">Path where the resulting PDF document will be saved.</param>
    /// <param name="options">Page size and margin options.</param>
    public void ConvertImagesToPdf(IReadOnlyList<string> imagePaths, string outPath, ImageToPdfOptionsDto? options = null)
    {
        ArgumentNullException.ThrowIfNull(imagePaths);
        if (imagePaths.Count == 0)
            throw new ArgumentException("Image list cannot be empty.", nameof(imagePaths));

        foreach (var img in imagePaths)
        {
            if (string.IsNullOrWhiteSpace(img))
                throw new ArgumentException("Image path cannot be null or empty.", nameof(imagePaths));
            if (!File.Exists(img))
                throw new FileNotFoundException($"Image file not found: {img}", img);
        }

        if (string.IsNullOrWhiteSpace(outPath))
            throw new ArgumentException("Output path cannot be null or empty.", nameof(outPath));

        string? outDir = Path.GetDirectoryName(outPath);
        if (!string.IsNullOrEmpty(outDir))
        {
            Directory.CreateDirectory(outDir);
        }

        options ??= new ImageToPdfOptionsDto();
        if (options.MarginPt < 0.0f)
            throw new ArgumentException("Margin cannot be negative.", nameof(options));

        string imagesJson = JsonSerializer.Serialize(imagePaths);
        int status = _api.ConvertImagesToPdf(
            imagesJson,
            (int)options.PageSize,
            options.MarginPt,
            outPath);

        if (status != (int)RbdStatusCode.Success)
        {
            throw new RbdNativeException(status, $"Failed to convert images to PDF: {_api.GetLastError()}");
        }
    }

    /// <summary>
    /// Converts markdown text into a paginated PDF document.
    /// </summary>
    /// <param name="markdownText">Markdown source text.</param>
    /// <param name="outPath">Path where the resulting PDF document will be saved.</param>
    public void ConvertMarkdownToPdf(string markdownText, string outPath)
    {
        ArgumentNullException.ThrowIfNull(markdownText);
        if (string.IsNullOrWhiteSpace(outPath))
            throw new ArgumentException("Output path cannot be null or empty.", nameof(outPath));

        string? outDir = Path.GetDirectoryName(outPath);
        if (!string.IsNullOrEmpty(outDir))
        {
            Directory.CreateDirectory(outDir);
        }

        int status = _api.ConvertMarkdownToPdf(markdownText, outPath);
        if (status != (int)RbdStatusCode.Success)
        {
            throw new RbdNativeException(status, $"Failed to convert markdown to PDF: {_api.GetLastError()}");
        }
    }

    /// <summary>
    /// Parses human-friendly page numbers (1-based, e.g. "1, 3, 5-8") into 0-based page indices.
    /// Returns null if empty/whitespace.
    /// </summary>
    public static List<uint>? ParsePageRange(string? input)
    {
        if (string.IsNullOrWhiteSpace(input)) return null;

        var set = new SortedSet<uint>();
        var tokens = input.Split(new[] { ',', ';' }, StringSplitOptions.RemoveEmptyEntries);

        foreach (var rawToken in tokens)
        {
            string token = rawToken.Trim();
            if (string.IsNullOrEmpty(token)) continue;

            if (token.Contains('-') || token.Contains(".."))
            {
                string[] parts = token.Split(new[] { '-', '.' }, StringSplitOptions.RemoveEmptyEntries);
                if (parts.Length == 2 && uint.TryParse(parts[0], out uint start) && uint.TryParse(parts[1], out uint end))
                {
                    uint min = Math.Min(start, end);
                    uint max = Math.Max(start, end);
                    for (uint p = min; p <= max; p++)
                    {
                        if (p > 0) set.Add(p - 1);
                    }
                }
            }
            else if (uint.TryParse(token, out uint page))
            {
                if (page > 0) set.Add(page - 1);
            }
        }

        return set.Count > 0 ? new List<uint>(set) : null;
    }
}
