using System;
using System.Collections.Generic;
using System.Text.Json;
using System.Text.Json.Serialization;
using System.Threading.Tasks;
using RbdOfis.Interop.Native;

namespace RbdOfis.Interop.Services;

public record KeywordScoreDto(
    [property: JsonPropertyName("word")] string Word,
    [property: JsonPropertyName("count")] int Count,
    [property: JsonPropertyName("score")] float Score
);

public record DocumentSummaryDto(
    [property: JsonPropertyName("summary_text")] string SummaryText,
    [property: JsonPropertyName("key_sentences")] List<string> KeySentences,
    [property: JsonPropertyName("top_keywords")] List<KeywordScoreDto> TopKeywords,
    [property: JsonPropertyName("total_sentences")] int TotalSentences,
    [property: JsonPropertyName("word_count")] int WordCount,
    [property: JsonPropertyName("reading_time_minutes")] float ReadingTimeMinutes
);

public record DocumentAnswerDto(
    [property: JsonPropertyName("query")] string Query,
    [property: JsonPropertyName("answer_text")] string AnswerText,
    [property: JsonPropertyName("context_excerpt")] string ContextExcerpt,
    [property: JsonPropertyName("page_number")] uint PageNumber,
    [property: JsonPropertyName("confidence")] float Confidence,
    [property: JsonPropertyName("found")] bool Found
);

public record ReflowParagraphDto(
    [property: JsonPropertyName("id")] uint Id,
    [property: JsonPropertyName("page_index")] uint PageIndex,
    [property: JsonPropertyName("bbox")] float[] Bbox,
    [property: JsonPropertyName("text")] string Text,
    [property: JsonPropertyName("font_size")] float FontSize,
    [property: JsonPropertyName("line_count")] int LineCount
);

/// <summary>
/// Managed service providing 100% offline document AI summarization, Q&amp;A, and fluid paragraph reflow editing.
/// </summary>
public sealed class DocumentAiService
{
    private readonly INativeRbdApi _nativeApi;

    public DocumentAiService(INativeRbdApi? nativeApi = null)
    {
        _nativeApi = nativeApi ?? NativeRbdApi.Instance;
    }

    public Task<DocumentSummaryDto> SummarizeDocumentAsync(SafeDocumentHandle doc, uint maxSentences = 5)
    {
        return Task.Run(() =>
        {
            int status = _nativeApi.SummarizeDocument(doc, maxSentences, out string json);
            if (status != 0)
            {
                throw new InvalidOperationException($"Document summarization failed with native error code {status}");
            }

            return JsonSerializer.Deserialize<DocumentSummaryDto>(json)
                ?? new DocumentSummaryDto(string.Empty, new(), new(), 0, 0, 0f);
        });
    }

    public Task<DocumentAnswerDto> AskDocumentAsync(SafeDocumentHandle doc, string query)
    {
        return Task.Run(() =>
        {
            if (string.IsNullOrWhiteSpace(query))
            {
                return new DocumentAnswerDto(string.Empty, "Sorgu boş olamaz.", string.Empty, 1, 0f, false);
            }

            int status = _nativeApi.AskDocument(doc, query, out string json);
            if (status != 0)
            {
                throw new InvalidOperationException($"Document question answering failed with native error code {status}");
            }

            return JsonSerializer.Deserialize<DocumentAnswerDto>(json)
                ?? new DocumentAnswerDto(query, "Yanıt alınamadı.", string.Empty, 1, 0f, false);
        });
    }

    public Task<List<ReflowParagraphDto>> GetReflowParagraphsAsync(SafeDocumentHandle doc, uint pageIndex)
    {
        return Task.Run(() =>
        {
            int status = _nativeApi.ReflowPageText(doc, pageIndex, out string json);
            if (status != 0)
            {
                throw new InvalidOperationException($"Reflow paragraph extraction failed with native error code {status}");
            }

            return JsonSerializer.Deserialize<List<ReflowParagraphDto>>(json)
                ?? new List<ReflowParagraphDto>();
        });
    }

    public Task ReplaceParagraphTextAsync(
        SafeDocumentHandle doc,
        string inputPath,
        uint pageIndex,
        float x0,
        float y0,
        float x1,
        float y1,
        string newText,
        float fontSize,
        string outputPath)
    {
        return Task.Run(() =>
        {
            int status = _nativeApi.ReplaceParagraphText(
                doc,
                inputPath,
                pageIndex,
                x0,
                y0,
                x1,
                y1,
                newText,
                fontSize,
                outputPath);

            if (status != 0)
            {
                throw new InvalidOperationException($"Paragraph text replacement failed with native error code {status}");
            }
        });
    }
}
