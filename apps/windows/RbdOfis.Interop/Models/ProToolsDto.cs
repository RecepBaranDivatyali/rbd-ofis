using System.Collections.Generic;
using System.Text.Json.Serialization;

namespace RbdOfis.Interop.Models;

/// <summary>
/// Redaction target specification for a single page.
/// </summary>
public sealed class PageRedactionDto
{
    [JsonPropertyName("page_index")]
    public uint PageIndex { get; set; }

    [JsonPropertyName("rects")]
    public List<PdfRect> Rects { get; set; } = new();

    /// <summary>
    /// Fill color for blackout box [R, G, B]. Defaults to black [0, 0, 0].
    /// </summary>
    [JsonPropertyName("fill_color")]
    [JsonConverter(typeof(ByteArrayAsIntArrayJsonConverter))]
    public byte[] FillColor { get; set; } = [0, 0, 0];
}

/// <summary>
/// Options controlling PDF compression and size optimization.
/// </summary>
public sealed class CompressionOptionsDto
{
    [JsonPropertyName("remove_unused_objects")]
    public bool RemoveUnusedObjects { get; set; } = true;

    [JsonPropertyName("recompress_streams")]
    public bool RecompressStreams { get; set; } = true;

    [JsonPropertyName("strip_metadata")]
    public bool StripMetadata { get; set; } = false;
}

/// <summary>
/// Result statistics detailing the outcome of a compression operation.
/// </summary>
public sealed class CompressionResultDto
{
    [JsonPropertyName("original_size")]
    public ulong OriginalSize { get; set; }

    [JsonPropertyName("compressed_size")]
    public ulong CompressedSize { get; set; }

    [JsonPropertyName("savings_percent")]
    public double SavingsPercent { get; set; }
}

/// <summary>
/// Spatial alignment for stamps, page numbers, and Bates stamps on a PDF page.
/// Matches rbd_core::StampPosition (0 to 5).
/// </summary>
public enum StampPosition
{
    TopLeft = 0,
    TopCenter = 1,
    TopRight = 2,
    BottomLeft = 3,
    BottomCenter = 4,
    BottomRight = 5
}

/// <summary>
/// Configuration options for document watermarks.
/// </summary>
public sealed class WatermarkOptionsDto
{
    public string Text { get; set; } = "CONFIDENTIAL";
    public float FontSize { get; set; } = 48.0f;
    public float RotationDegrees { get; set; } = 45.0f;
    public float Opacity { get; set; } = 0.25f;
    public byte[] ColorRgb { get; set; } = [180, 180, 180];
    public bool OnTop { get; set; } = true;
}

/// <summary>
/// Configuration options for dynamic page numbering.
/// </summary>
public sealed class PageNumberOptionsDto
{
    public string Template { get; set; } = "Sayfa {page} / {total}";
    public StampPosition Position { get; set; } = StampPosition.BottomCenter;
    public float FontSize { get; set; } = 10.0f;
    public float Margin { get; set; } = 36.0f;
}

/// <summary>
/// Configuration options for legal Bates stamping.
/// </summary>
public sealed class BatesOptionsDto
{
    public string Prefix { get; set; } = "RBD-";
    public uint StartNumber { get; set; } = 1;
    public uint PaddingDigits { get; set; } = 6;
    public string Suffix { get; set; } = string.Empty;
    public StampPosition Position { get; set; } = StampPosition.BottomRight;
}

/// <summary>
/// Metadata and state for an interactive AcroForm field.
/// </summary>
public sealed class FormFieldDto
{
    [JsonPropertyName("name")]
    public string Name { get; set; } = string.Empty;

    [JsonPropertyName("field_type")]
    public string FieldType { get; set; } = "Text";

    [JsonPropertyName("value")]
    public string Value { get; set; } = string.Empty;

    [JsonPropertyName("read_only")]
    public bool ReadOnly { get; set; } = false;
}
