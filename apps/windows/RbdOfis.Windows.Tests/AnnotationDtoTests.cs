using System.Collections.Generic;
using System.Text.Json;
using RbdOfis.Interop.Models;
using Xunit;

namespace RbdOfis.Windows.Tests;

public class AnnotationDtoTests
{
    [Fact]
    public void Verify_CreateHighlight_Sets_Properties_And_Serializes_To_Json()
    {
        var rect = new PdfRect(10, 20, 150, 40);
        var anno = AnnotationDto.CreateHighlight(101, 0, rect, [255, 235, 59, 128]);

        Assert.Equal(101ul, anno.Id);
        Assert.Equal("Highlight", anno.KindName);
        Assert.Equal(0u, anno.PageIndex);
        Assert.Equal(rect, anno.Rect);
        Assert.Equal(255, anno.ColorRgba[0]);
        Assert.Equal(235, anno.ColorRgba[1]);

        string json = JsonSerializer.Serialize(anno);
        Assert.Contains("\"kind\":\"Highlight\"", json);
        Assert.Contains("\"x0\":10", json);

        var restored = JsonSerializer.Deserialize<AnnotationDto>(json);
        Assert.NotNull(restored);
        Assert.Equal(anno.Id, restored.Id);
        Assert.Equal(anno.KindName, restored.KindName);
        Assert.Equal(anno.Rect, restored.Rect);
    }

    [Fact]
    public void Verify_CreateStickyNote_Positions_Correctly()
    {
        var rect = new PdfRect(200.5f, 300.5f, 224.5f, 324.5f);
        var note = AnnotationDto.CreateStickyNote(102, 2, rect, "Important reviewer note");

        Assert.Equal(102ul, note.Id);
        Assert.Equal("StickyNote", note.KindName);
        Assert.Equal(2u, note.PageIndex);
        Assert.Equal(200.5f, note.Rect.X0);
        Assert.Equal(300.5f, note.Rect.Y0);
        Assert.Equal("Important reviewer note", note.Contents);
    }

    [Fact]
    public void Verify_CreateFreehandInk_Preserves_Points_Array()
    {
        var points = new List<(float X, float Y)>
        {
            (10.0f, 10.0f),
            (20.5f, 25.5f),
            (40.0f, 50.0f)
        };
        var bbox = new PdfRect(10, 10, 40, 50);

        var ink = AnnotationDto.CreateFreehandInk(103, 1, bbox, points, 3.0f, [0, 0, 0, 255]);

        Assert.Equal(103ul, ink.Id);
        Assert.Equal("FreehandInk", ink.KindName);
        Assert.Equal(1u, ink.PageIndex);
        Assert.Equal(3, ink.Points.Count);
        Assert.Equal(10.0, ink.Rect.X0);
        Assert.Equal(50.0, ink.Rect.Y1);

        string json = JsonSerializer.Serialize(ink);
        var deserialized = JsonSerializer.Deserialize<AnnotationDto>(json);
        Assert.NotNull(deserialized);
        Assert.Equal(ink.Id, deserialized.Id);
        Assert.Equal("FreehandInk", deserialized.KindName);
    }

    [Fact]
    public void Verify_List_Serialization_Roundtrip()
    {
        var list = new List<AnnotationDto>
        {
            AnnotationDto.CreateHighlight(1, 0, new PdfRect(5, 5, 50, 20)),
            AnnotationDto.CreateRectangle(2, 0, new PdfRect(100, 100, 200, 200), [0, 255, 0, 255]),
            AnnotationDto.CreateUnderline(3, 1, new PdfRect(10, 50, 120, 60)),
            AnnotationDto.CreateStrikeOut(4, 1, new PdfRect(10, 70, 120, 80)),
            AnnotationDto.CreateFreeText(5, 2, new PdfRect(50, 50, 150, 80), "Draft Comment")
        };

        string json = JsonSerializer.Serialize(list);
        var restored = JsonSerializer.Deserialize<List<AnnotationDto>>(json);

        Assert.NotNull(restored);
        Assert.Equal(list.Count, restored.Count);
        for (int i = 0; i < list.Count; i++)
        {
            Assert.Equal(list[i].Id, restored[i].Id);
            Assert.Equal(list[i].KindName, restored[i].KindName);
            Assert.Equal(list[i].PageIndex, restored[i].PageIndex);
        }
    }
}
