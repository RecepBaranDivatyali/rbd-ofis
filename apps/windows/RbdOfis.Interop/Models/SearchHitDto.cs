using System.Collections.Generic;
using System.Text.Json.Serialization;

namespace RbdOfis.Interop.Models;

/// <summary>
/// Result of a search query match inside a PDF document page.
/// Includes visual bounding boxes (rects) for rendering interactive highlight overlays.
/// </summary>
public sealed class SearchHitDto
{
    [JsonPropertyName("page")]
    public uint Page { get; set; }

    [JsonPropertyName("text")]
    public string Text { get; set; } = string.Empty;

    [JsonPropertyName("char_offset")]
    public int CharOffset { get; set; }

    [JsonPropertyName("rects")]
    public List<PdfRect> Rects { get; set; } = new();

    public override string ToString() => $"[Page {Page + 1}] \"{Text}\" ({Rects.Count} rects)";
}
