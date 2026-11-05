using System.Text.Json.Serialization;

namespace RbdOfis.Interop.Models;

/// <summary>
/// Office document metadata inspection result (DOCX, XLSX, PPTX).
/// </summary>
public sealed class OfficeDocumentInfoDto
{
    [JsonPropertyName("doc_type")]
    public string DocType { get; set; } = string.Empty;

    [JsonPropertyName("title")]
    public string Title { get; set; } = string.Empty;

    [JsonPropertyName("word_count")]
    public int WordCount { get; set; }

    [JsonPropertyName("item_count")]
    public int ItemCount { get; set; }

    [JsonPropertyName("summary")]
    public string Summary { get; set; } = string.Empty;
}
