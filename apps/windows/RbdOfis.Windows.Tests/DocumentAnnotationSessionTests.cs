using System;
using System.IO;
using RbdOfis.Interop.Models;
using RbdOfis.Interop.Native;
using Xunit;

namespace RbdOfis.Windows.Tests;

public class DocumentAnnotationSessionTests
{
    [Fact]
    public void Verify_Document_Annotation_Lifecycle_And_Undo_Redo()
    {
        var mockApi = new MockNativeRbdApi();
        using var engine = new RbdEngine(mockApi);
        using var doc = engine.OpenDocument(new byte[] { 1, 2, 3 });

        // Initially no annotations and cannot undo/redo
        var initial = doc.GetAnnotations(0);
        Assert.Empty(initial);
        Assert.False(doc.CanUndo);
        Assert.False(doc.CanRedo);

        // 1. Add Highlight annotation
        var highlight = AnnotationDto.CreateHighlight(1001, 0, new PdfRect(10, 10, 100, 25), [255, 235, 59, 128]);
        highlight.Contents = "First Note";
        doc.AddAnnotation(highlight);

        var list1 = doc.GetAnnotations(0);
        Assert.Single(list1);
        Assert.Equal(highlight.Id, list1[0].Id);
        Assert.True(doc.CanUndo);
        Assert.False(doc.CanRedo);

        // 2. Add Rectangle annotation
        var rect = AnnotationDto.CreateRectangle(1002, 0, new PdfRect(50, 50, 150, 150), [0, 255, 0, 255]);
        doc.AddAnnotation(rect);

        var list2 = doc.GetAnnotations(0);
        Assert.Equal(2, list2.Count);

        // 3. Undo rectangle addition
        bool undone = doc.Undo();
        Assert.True(undone);
        var afterUndo = doc.GetAnnotations(0);
        Assert.Single(afterUndo);
        Assert.Equal(highlight.Id, afterUndo[0].Id);
        Assert.True(doc.CanRedo);

        // 4. Redo rectangle addition
        bool redone = doc.Redo();
        Assert.True(redone);
        var afterRedo = doc.GetAnnotations(0);
        Assert.Equal(2, afterRedo.Count);

        // 5. Remove annotation
        doc.RemoveAnnotation(rect.Id);
        var listAfterRemove = doc.GetAnnotations(0);
        Assert.Single(listAfterRemove);
        Assert.Equal(highlight.Id, listAfterRemove[0].Id);

        // 6. Save annotations to output path
        string tempIn = Path.GetTempFileName();
        string tempOut = Path.Combine(Path.GetTempPath(), $"saved_anno_{Guid.NewGuid():N}.pdf");
        try
        {
            File.WriteAllText(tempIn, "%PDF-1.4 mock");
            doc.SaveAnnotations(tempIn, tempOut);
            Assert.True(File.Exists(tempOut));
        }
        finally
        {
            if (File.Exists(tempIn)) File.Delete(tempIn);
            if (File.Exists(tempOut)) File.Delete(tempOut);
        }
    }

    [Fact]
    public void Verify_Document_Text_Extraction_And_Search()
    {
        var mockApi = new MockNativeRbdApi();
        using var engine = new RbdEngine(mockApi);
        using var doc = engine.OpenDocument(new byte[] { 1, 2, 3 });

        string pageText = doc.ExtractText(0);
        Assert.Contains("RBD OFİS Document Page 1", pageText);

        var hits = doc.Search("Mock", caseSensitive: false);
        Assert.NotEmpty(hits);
        Assert.All(hits, h => Assert.Contains("Mock", h.Text));
    }
}
