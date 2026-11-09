using System;
using System.IO;
using System.Threading.Tasks;
using RbdOfis.Interop.Native;
using RbdOfis.Interop.Services;
using Xunit;

namespace RbdOfis.Windows.Tests;

public class DocumentAiServiceTests
{
    private readonly MockNativeRbdApi _mockApi = new();
    private readonly DocumentAiService _service;

    public DocumentAiServiceTests()
    {
        _service = new DocumentAiService(_mockApi);
    }

    [Fact]
    public async Task SummarizeDocument_Returns_Valid_Summary()
    {
        using var handle = new SafeDocumentHandle(new IntPtr(123), ownsHandle: false);
        var summary = await _service.SummarizeDocumentAsync(handle, 5);
        Assert.NotNull(summary);
        Assert.False(string.IsNullOrWhiteSpace(summary.SummaryText));
        Assert.NotEmpty(summary.KeySentences);
        Assert.True(summary.WordCount > 0);
        Assert.True(summary.ReadingTimeMinutes > 0);
    }

    [Fact]
    public async Task AskDocument_Returns_Answer_With_Citation()
    {
        using var handle = new SafeDocumentHandle(new IntPtr(123), ownsHandle: false);
        var answer = await _service.AskDocumentAsync(handle, "Proje lisansı nedir?");
        Assert.NotNull(answer);
        Assert.True(answer.Found);
        Assert.False(string.IsNullOrWhiteSpace(answer.AnswerText));
        Assert.True(answer.PageNumber >= 1);
        Assert.True(answer.Confidence > 0.5f);
    }

    [Fact]
    public async Task AskDocument_EmptyQuery_Returns_EmptyAnswer()
    {
        using var handle = new SafeDocumentHandle(new IntPtr(123), ownsHandle: false);
        var answer = await _service.AskDocumentAsync(handle, "   ");
        Assert.NotNull(answer);
        Assert.False(answer.Found);
    }

    [Fact]
    public async Task GetReflowParagraphs_Returns_Paragraph_Candidates()
    {
        using var handle = new SafeDocumentHandle(new IntPtr(123), ownsHandle: false);
        var paragraphs = await _service.GetReflowParagraphsAsync(handle, 0);
        Assert.NotNull(paragraphs);
        Assert.NotEmpty(paragraphs);
        Assert.Equal(0u, paragraphs[0].PageIndex);
        Assert.Equal(4, paragraphs[0].Bbox.Length);
        Assert.False(string.IsNullOrWhiteSpace(paragraphs[0].Text));
    }

    [Fact]
    public async Task ReplaceParagraphText_Creates_Output()
    {
        using var handle = new SafeDocumentHandle(new IntPtr(123), ownsHandle: false);
        string tmpIn = Path.GetTempFileName();
        string tmpOut = Path.GetTempFileName();
        try
        {
            File.WriteAllText(tmpIn, "%PDF-1.7 initial content");
            await _service.ReplaceParagraphTextAsync(
                handle,
                tmpIn,
                0,
                72f, 600f, 400f, 650f,
                "Yeni reflow edilmiş metin",
                12f,
                tmpOut);

            Assert.True(File.Exists(tmpOut));
            Assert.True(new FileInfo(tmpOut).Length > 0);
        }
        finally
        {
            if (File.Exists(tmpIn)) File.Delete(tmpIn);
            if (File.Exists(tmpOut)) File.Delete(tmpOut);
        }
    }
}
