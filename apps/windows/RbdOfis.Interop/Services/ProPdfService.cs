using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using RbdOfis.Interop.Models;
using RbdOfis.Interop.Native;

namespace RbdOfis.Interop.Services;

/// <summary>
/// High-level managed service for Professional PDF Tools (Phase 4).
/// Provides Redaction, Compression, Watermarks, Dynamic Page Numbering, Bates Stamping, and Interactive AcroForms.
/// </summary>
public sealed class ProPdfService
{
    private readonly INativeRbdApi _api;

    public ProPdfService(INativeRbdApi? api = null)
    {
        _api = api ?? NativeRbdApi.Instance;
    }

    /// <summary>
    /// Permanently redacts visual content and underlying text from specified page rectangular areas.
    /// </summary>
    public void Redact(string inPath, string outPath, IReadOnlyList<PageRedactionDto> redactions)
    {
        ValidateFiles(inPath, outPath);
        ArgumentNullException.ThrowIfNull(redactions);
        if (redactions.Count == 0) throw new ArgumentException("Redactions list cannot be empty.", nameof(redactions));

        string json = JsonSerializer.Serialize(redactions);
        int status = _api.ProRedactDocument(inPath, outPath, json);
        if (status != (int)RbdStatusCode.Success)
        {
            throw new RbdNativeException(status, $"Failed to redact document: {_api.GetLastError()}");
        }
    }

    /// <summary>
    /// Compresses and optimizes a PDF document, returning size savings metrics.
    /// </summary>
    public CompressionResultDto Compress(string inPath, string outPath, CompressionOptionsDto? options = null)
    {
        ValidateFiles(inPath, outPath);
        options ??= new CompressionOptionsDto();

        int status = _api.ProCompressDocument(
            inPath,
            outPath,
            options.RemoveUnusedObjects,
            options.RecompressStreams,
            options.StripMetadata,
            out string savingsJson);

        if (status != (int)RbdStatusCode.Success)
        {
            throw new RbdNativeException(status, $"Failed to compress document: {_api.GetLastError()}");
        }

        try
        {
            var result = JsonSerializer.Deserialize<CompressionResultDto>(savingsJson);
            if (result != null) return result;
        }
        catch
        {
            // Fallback calculation if JSON deserialization fails
        }

        long original = File.Exists(inPath) ? new FileInfo(inPath).Length : 1;
        long compressed = File.Exists(outPath) ? new FileInfo(outPath).Length : 1;
        double pct = original > 0 ? (double)(original - compressed) / original * 100.0 : 0.0;
        return new CompressionResultDto
        {
            OriginalSize = (ulong)original,
            CompressedSize = (ulong)compressed,
            SavingsPercent = Math.Max(0.0, pct)
        };
    }

    /// <summary>
    /// Applies a text watermark to all pages of a PDF document.
    /// </summary>
    public void ApplyWatermark(string inPath, string outPath, WatermarkOptionsDto options)
    {
        ValidateFiles(inPath, outPath);
        ArgumentNullException.ThrowIfNull(options);
        if (string.IsNullOrWhiteSpace(options.Text)) throw new ArgumentException("Watermark text cannot be empty.", nameof(options));

        byte r = options.ColorRgb.Length > 0 ? options.ColorRgb[0] : (byte)180;
        byte g = options.ColorRgb.Length > 1 ? options.ColorRgb[1] : (byte)180;
        byte b = options.ColorRgb.Length > 2 ? options.ColorRgb[2] : (byte)180;

        int status = _api.ProApplyWatermark(
            inPath,
            outPath,
            options.Text,
            options.FontSize,
            options.RotationDegrees,
            options.Opacity,
            r,
            g,
            b,
            options.OnTop);

        if (status != (int)RbdStatusCode.Success)
        {
            throw new RbdNativeException(status, $"Failed to apply watermark: {_api.GetLastError()}");
        }
    }

    /// <summary>
    /// Applies dynamic page numbers to pages using a template string (e.g. "Sayfa {page} / {total}").
    /// </summary>
    public void ApplyPageNumbers(string inPath, string outPath, PageNumberOptionsDto options)
    {
        ValidateFiles(inPath, outPath);
        ArgumentNullException.ThrowIfNull(options);
        if (string.IsNullOrWhiteSpace(options.Template)) throw new ArgumentException("Page number template cannot be empty.", nameof(options));

        int status = _api.ProApplyPageNumbers(
            inPath,
            outPath,
            options.Template,
            (int)options.Position,
            options.FontSize,
            options.Margin);

        if (status != (int)RbdStatusCode.Success)
        {
            throw new RbdNativeException(status, $"Failed to apply page numbers: {_api.GetLastError()}");
        }
    }

    /// <summary>
    /// Applies sequential Bates stamping across document pages.
    /// </summary>
    public void ApplyBatesStamp(string inPath, string outPath, BatesOptionsDto options)
    {
        ValidateFiles(inPath, outPath);
        ArgumentNullException.ThrowIfNull(options);

        int status = _api.ProApplyBatesStamp(
            inPath,
            outPath,
            options.Prefix ?? string.Empty,
            options.StartNumber,
            options.PaddingDigits,
            options.Suffix ?? string.Empty,
            (int)options.Position);

        if (status != (int)RbdStatusCode.Success)
        {
            throw new RbdNativeException(status, $"Failed to apply Bates stamp: {_api.GetLastError()}");
        }
    }

    /// <summary>
    /// Reads all interactive form fields and their values from a PDF document.
    /// </summary>
    public List<FormFieldDto> ReadFormFields(string inPath)
    {
        if (string.IsNullOrWhiteSpace(inPath)) throw new ArgumentException("Input path cannot be empty.", nameof(inPath));
        if (!File.Exists(inPath)) throw new FileNotFoundException("Input file not found.", inPath);

        int status = _api.ProReadFormFields(inPath, out string fieldsJson);
        if (status != (int)RbdStatusCode.Success)
        {
            throw new RbdNativeException(status, $"Failed to read form fields: {_api.GetLastError()}");
        }

        try
        {
            var list = JsonSerializer.Deserialize<List<FormFieldDto>>(fieldsJson);
            return list ?? new List<FormFieldDto>();
        }
        catch
        {
            return new List<FormFieldDto>();
        }
    }

    /// <summary>
    /// Fills form fields with specified values, with optional flattening to prevent further edits.
    /// </summary>
    public void FillFormFields(string inPath, string outPath, IReadOnlyDictionary<string, string> values, bool flatten)
    {
        ValidateFiles(inPath, outPath);
        ArgumentNullException.ThrowIfNull(values);

        string json = JsonSerializer.Serialize(values);
        int status = _api.ProFillFormFields(inPath, outPath, json, flatten);
        if (status != (int)RbdStatusCode.Success)
        {
            throw new RbdNativeException(status, $"Failed to fill form fields: {_api.GetLastError()}");
        }
    }

    private static void ValidateFiles(string inPath, string outPath)
    {
        if (string.IsNullOrWhiteSpace(inPath)) throw new ArgumentException("Input path cannot be empty.", nameof(inPath));
        if (string.IsNullOrWhiteSpace(outPath)) throw new ArgumentException("Output path cannot be empty.", nameof(outPath));
        if (!File.Exists(inPath)) throw new FileNotFoundException("Input file not found.", inPath);
    }
}
