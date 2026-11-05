using System.Collections.Generic;
using System.Text.Json.Serialization;

namespace RbdOfis.Interop.Models;

/// <summary>
/// Image format for PDF page export.
/// Matches rbd_core::ImageFormatType (0: Png, 1: Jpeg).
/// </summary>
public enum ImageFormatEnum
{
    Png = 0,
    Jpeg = 1
}

/// <summary>
/// Page size specification for Image to PDF conversion.
/// Matches rbd_core::PageSize (0: A4, 1: Letter, 2: FitImage).
/// </summary>
public enum PageSizeEnum
{
    A4 = 0,
    Letter = 1,
    FitImage = 2
}

/// <summary>
/// Options for converting PDF pages to images.
/// </summary>
public sealed class PdfToImageOptionsDto
{
    /// <summary>
    /// Rendering DPI (dots per inch). Typically 72, 150, or 300. Must be &gt; 0. Defaults to 150.
    /// </summary>
    [JsonPropertyName("dpi")]
    public uint Dpi { get; set; } = 150;

    /// <summary>
    /// Target image format (PNG or JPEG). Defaults to PNG.
    /// </summary>
    [JsonPropertyName("format")]
    public ImageFormatEnum Format { get; set; } = ImageFormatEnum.Png;

    /// <summary>
    /// Optional 0-based page indices to convert. If null or empty, all pages are converted.
    /// </summary>
    [JsonPropertyName("page_range")]
    public List<uint>? PageRange { get; set; }
}

/// <summary>
/// Options for converting image files into a single PDF document.
/// </summary>
public sealed class ImageToPdfOptionsDto
{
    /// <summary>
    /// Target page size (A4, Letter, or FitImage). Defaults to A4.
    /// </summary>
    [JsonPropertyName("page_size")]
    public PageSizeEnum PageSize { get; set; } = PageSizeEnum.A4;

    /// <summary>
    /// Page margin in points (1/72 inch). Must be &gt;= 0.0. Defaults to 20.0f.
    /// Note: Margin is ignored when PageSize is FitImage.
    /// </summary>
    [JsonPropertyName("margin_pt")]
    public float MarginPt { get; set; } = 20.0f;
}
