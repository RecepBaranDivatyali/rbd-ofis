using System;
using System.IO;
using RbdOfis.Interop.Models;
using RbdOfis.Interop.Native;

namespace RbdOfis.Interop.Services;

/// <summary>
/// High-level service for PDF page operations (Rotate, Delete, Split, Merge, Reorder, Crop, Blank Page, Decrypt).
/// Delegates to native rbd-core / rbd-engine page-ops.
/// </summary>
public sealed class PageOperationService
{
    private readonly INativeRbdApi _api;

    public PageOperationService(INativeRbdApi? api = null)
    {
        _api = api ?? NativeRbdApi.Instance;
    }

    public void Rotate(string inputPath, int degrees, string outputPath)
    {
        ValidateFiles(inputPath, outputPath);
        if (degrees % 90 != 0)
        {
            throw new ArgumentException("Rotation degrees must be a multiple of 90 (e.g. 90, 180, 270).", nameof(degrees));
        }

        int status = _api.PageOpRotate(inputPath, outputPath, degrees);
        if (status != (int)RbdStatusCode.Success)
        {
            throw new RbdNativeException(status, $"Rotate failed for '{inputPath}' by {degrees}° -> '{outputPath}'.");
        }
    }

    public void DeletePages(string inputPath, int[] pageNumbers, string outputPath)
    {
        ValidateFiles(inputPath, outputPath);
        ArgumentNullException.ThrowIfNull(pageNumbers);
        if (pageNumbers.Length == 0)
        {
            throw new ArgumentException("Page numbers list to delete cannot be empty.", nameof(pageNumbers));
        }

        string pagesJson = System.Text.Json.JsonSerializer.Serialize(pageNumbers);
        int status = _api.PageOpDeletePages(inputPath, outputPath, pagesJson);
        if (status != (int)RbdStatusCode.Success)
        {
            throw new RbdNativeException(status, $"Delete pages failed for '{inputPath}'.");
        }
    }

    public void Split(string inputPath, uint startPage, uint endPage, string outputPath)
    {
        ValidateFiles(inputPath, outputPath);
        if (startPage < 1 || startPage > endPage)
        {
            throw new ArgumentException($"Invalid page range: start {startPage} must be >= 1 and <= end {endPage}.");
        }

        int status = _api.PageOpSplit(inputPath, outputPath, startPage, endPage);
        if (status != (int)RbdStatusCode.Success)
        {
            throw new RbdNativeException(status, $"Split failed for '{inputPath}' range [{startPage}..{endPage}].");
        }
    }

    public void Merge(string[] inputPaths, string outputPath)
    {
        ArgumentNullException.ThrowIfNull(inputPaths);
        if (inputPaths.Length < 2)
        {
            throw new ArgumentException("At least two PDF files are required to perform a merge.", nameof(inputPaths));
        }
        foreach (var path in inputPaths)
        {
            if (!File.Exists(path))
            {
                throw new FileNotFoundException($"Input file not found: '{path}'", path);
            }
        }
        ArgumentException.ThrowIfNullOrWhiteSpace(outputPath);

        string inPathsJson = System.Text.Json.JsonSerializer.Serialize(inputPaths);
        int status = _api.PageOpMerge(outputPath, inPathsJson);
        if (status != (int)RbdStatusCode.Success)
        {
            throw new RbdNativeException(status, $"Merge failed into '{outputPath}'.");
        }
    }

    public void ReorderPages(string inputPath, uint[] newOrder, string outputPath)
    {
        ValidateFiles(inputPath, outputPath);
        ArgumentNullException.ThrowIfNull(newOrder);
        if (newOrder.Length == 0)
        {
            throw new ArgumentException("New page order list cannot be empty.", nameof(newOrder));
        }

        string orderJson = System.Text.Json.JsonSerializer.Serialize(newOrder);
        int status = _api.PageOpReorder(inputPath, outputPath, orderJson);
        if (status != (int)RbdStatusCode.Success)
        {
            throw new RbdNativeException(status, $"Reorder pages failed for '{inputPath}'.");
        }
    }

    public void CropPage(string inputPath, uint pageNum, PdfRect mediaBox, string outputPath)
    {
        ValidateFiles(inputPath, outputPath);
        ArgumentNullException.ThrowIfNull(mediaBox);

        int status = _api.PageOpCrop(inputPath, outputPath, pageNum, mediaBox.X0, mediaBox.Y0, mediaBox.X1, mediaBox.Y1);
        if (status != (int)RbdStatusCode.Success)
        {
            throw new RbdNativeException(status, $"Crop page {pageNum} failed for '{inputPath}'.");
        }
    }

    public void InsertBlankPage(string inputPath, uint afterPage, float widthPt, float heightPt, string outputPath)
    {
        ValidateFiles(inputPath, outputPath);
        if (widthPt <= 0 || heightPt <= 0)
        {
            throw new ArgumentException("Page dimensions must be positive numbers.");
        }

        int status = _api.PageOpInsertBlank(inputPath, outputPath, afterPage, widthPt, heightPt);
        if (status != (int)RbdStatusCode.Success)
        {
            throw new RbdNativeException(status, $"Insert blank page failed for '{inputPath}'.");
        }
    }

    public void Decrypt(string inputPath, string password, string outputPath)
    {
        ValidateFiles(inputPath, outputPath);
        ArgumentNullException.ThrowIfNull(password);

        int status = _api.PageOpDecrypt(inputPath, outputPath, password);
        if (status != (int)RbdStatusCode.Success)
        {
            throw new RbdNativeException(status, $"Decrypt failed for '{inputPath}'.");
        }
    }

    private static void ValidateFiles(string inputPath, string outputPath)
    {
        if (!File.Exists(inputPath))
        {
            throw new FileNotFoundException($"Input file not found: '{inputPath}'", inputPath);
        }
        ArgumentException.ThrowIfNullOrWhiteSpace(outputPath);
    }
}
