using System.Collections.Generic;
using System.Text.Json;
using RbdOfis.Interop.Models;
using Xunit;

namespace RbdOfis.Windows.Tests;

public class SearchHitParsingTests
{
    [Fact]
    public void Verify_SearchHit_Serialization_And_Deserialization()
    {
        var hit = new SearchHitDto
        {
            Page = 3,
            CharOffset = 124,
            Text = "...contains Turkish characters...",
            Rects = new List<PdfRect>
            {
                new PdfRect(72.0f, 144.0f, 120.0f, 156.0f),
                new PdfRect(120.0f, 144.0f, 180.0f, 156.0f)
            }
        };

        string json = JsonSerializer.Serialize(hit);
        Assert.Contains("\"page\":3", json);
        Assert.Contains("\"text\":", json);
        Assert.Contains("\"char_offset\":124", json);
        Assert.Contains("\"x0\":72", json);

        var list = JsonSerializer.Deserialize<List<SearchHitDto>>($"[{json}]");
        Assert.NotNull(list);
        Assert.Single(list);

        var item = list[0];
        Assert.Equal(3u, item.Page);
        Assert.Equal(124, item.CharOffset);
        Assert.Equal("...contains Turkish characters...", item.Text);
        Assert.Equal(2, item.Rects.Count);
        Assert.Equal(72.0f, item.Rects[0].X0);
        Assert.Equal(144.0f, item.Rects[0].Y0);
    }

    [Fact]
    public void Verify_Empty_Or_Invalid_Json_Handling()
    {
        var emptyList = JsonSerializer.Deserialize<List<SearchHitDto>>("[]");
        Assert.NotNull(emptyList);
        Assert.Empty(emptyList);

        Assert.Throws<JsonException>(() => JsonSerializer.Deserialize<List<SearchHitDto>>("invalid json {}"));
    }
}
