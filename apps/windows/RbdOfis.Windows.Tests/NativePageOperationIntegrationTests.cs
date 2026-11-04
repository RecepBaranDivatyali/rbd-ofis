using System;
using System.IO;
using RbdOfis.Interop.Models;
using RbdOfis.Interop.Native;
using RbdOfis.Interop.Services;
using Xunit;

namespace RbdOfis.Windows.Tests;

public class NativePageOperationIntegrationTests : IDisposable
{
    private readonly string _tempDir;
    private readonly PageOperationService _pageOps;
    private readonly string _sample1Page;
    private readonly string _sample2Pages;
    private readonly string _sampleBlank;

    public NativePageOperationIntegrationTests()
    {
        _tempDir = Path.Combine(Path.GetTempPath(), $"rbd_int_{Guid.NewGuid():N}");
        Directory.CreateDirectory(_tempDir);
        _pageOps = new PageOperationService(NativeRbdApi.Instance);

        string corpusDir = FindCorpusDirectory();
        _sample1Page = Path.Combine(corpusDir, "pdfium-hello-world.pdf");
        _sample2Pages = Path.Combine(corpusDir, "pdfium-hello-world-2-pages.pdf");
        _sampleBlank = Path.Combine(corpusDir, "pdfium-about-blank.pdf");

        Assert.True(File.Exists(_sample1Page), $"Corpus sample 1 not found: {_sample1Page}");
        Assert.True(File.Exists(_sample2Pages), $"Corpus sample 2 not found: {_sample2Pages}");
        Assert.True(File.Exists(_sampleBlank), $"Corpus sample 3 not found: {_sampleBlank}");
    }

    public void Dispose()
    {
        try
        {
            if (Directory.Exists(_tempDir))
            {
                Directory.Delete(_tempDir, recursive: true);
            }
        }
        catch { }
    }

    private static string FindCorpusDirectory()
    {
        string current = AppContext.BaseDirectory;
        for (int i = 0; i < 8 && current != null; i++)
        {
            string candidate = Path.Combine(current, "corpus", "files", "pdf-basic");
            if (Directory.Exists(candidate)) return Path.GetFullPath(candidate);
            current = Directory.GetParent(current)?.FullName!;
        }
        throw new DirectoryNotFoundException("Could not locate corpus/files/pdf-basic directory.");
    }

    [Fact]
    public void Verify_Live_Rotate_Page_Operation()
    {
        string outPath = Path.Combine(_tempDir, "rotated.pdf");
        _pageOps.Rotate(_sample1Page, 90, outPath);

        Assert.True(File.Exists(outPath));
        Assert.True(new FileInfo(outPath).Length > 0);

        using var engine = new RbdEngine(NativeRbdApi.Instance);
        using var doc = engine.OpenDocument(outPath);
        Assert.Equal(1, doc.PageCount);
    }

    [Fact]
    public void Verify_Live_Split_Page_Operation()
    {
        string outPath = Path.Combine(_tempDir, "split.pdf");
        _pageOps.Split(_sample2Pages, 1, 1, outPath);

        Assert.True(File.Exists(outPath));
        Assert.True(new FileInfo(outPath).Length > 0);

        using var engine = new RbdEngine(NativeRbdApi.Instance);
        using var doc = engine.OpenDocument(outPath);
        Assert.Equal(1, doc.PageCount);
    }

    [Fact]
    public void Verify_Live_Merge_Page_Operation()
    {
        string outPath = Path.Combine(_tempDir, "merged.pdf");
        _pageOps.Merge(new[] { _sample1Page, _sampleBlank }, outPath);

        Assert.True(File.Exists(outPath));
        Assert.True(new FileInfo(outPath).Length > 0);

        using var engine = new RbdEngine(NativeRbdApi.Instance);
        using var doc = engine.OpenDocument(outPath);
        Assert.Equal(2, doc.PageCount);
    }

    [Fact]
    public void Verify_Live_Delete_Pages_Operation()
    {
        string outPath = Path.Combine(_tempDir, "deleted.pdf");
        _pageOps.DeletePages(_sample2Pages, new[] { 2 }, outPath);

        Assert.True(File.Exists(outPath));
        Assert.True(new FileInfo(outPath).Length > 0);

        using var engine = new RbdEngine(NativeRbdApi.Instance);
        using var doc = engine.OpenDocument(outPath);
        Assert.Equal(1, doc.PageCount);
    }

    [Fact]
    public void Verify_Live_Reorder_Pages_Operation()
    {
        string outPath = Path.Combine(_tempDir, "reordered.pdf");
        _pageOps.ReorderPages(_sample2Pages, new uint[] { 2, 1 }, outPath);

        Assert.True(File.Exists(outPath));
        Assert.True(new FileInfo(outPath).Length > 0);

        using var engine = new RbdEngine(NativeRbdApi.Instance);
        using var doc = engine.OpenDocument(outPath);
        Assert.Equal(2, doc.PageCount);
    }

    [Fact]
    public void Verify_Live_Insert_Blank_Page_Operation()
    {
        string outPath = Path.Combine(_tempDir, "inserted_blank.pdf");
        _pageOps.InsertBlankPage(_sample1Page, 1, 595.0f, 842.0f, outPath);

        Assert.True(File.Exists(outPath));
        Assert.True(new FileInfo(outPath).Length > 0);

        using var engine = new RbdEngine(NativeRbdApi.Instance);
        using var doc = engine.OpenDocument(outPath);
        Assert.Equal(2, doc.PageCount);
    }

    [Fact]
    public void Verify_Live_Crop_Page_Operation()
    {
        string outPath = Path.Combine(_tempDir, "cropped.pdf");
        _pageOps.CropPage(_sample1Page, 1, new PdfRect(50, 50, 300, 400), outPath);

        Assert.True(File.Exists(outPath));
        Assert.True(new FileInfo(outPath).Length > 0);

        using var engine = new RbdEngine(NativeRbdApi.Instance);
        using var doc = engine.OpenDocument(outPath);
        Assert.Equal(1, doc.PageCount);
    }

    [Fact]
    public void Verify_Live_Annotation_Save_To_Path()
    {
        using var engine = new RbdEngine(NativeRbdApi.Instance);
        using var doc = engine.OpenDocument(_sample1Page);

        var anno = AnnotationDto.CreateHighlight(999, 0, new PdfRect(10, 10, 150, 30), [255, 235, 59, 128]);
        doc.AddAnnotation(anno);

        string outPath = Path.Combine(_tempDir, "saved_annotations.pdf");
        doc.SaveAnnotations(_sample1Page, outPath);

        Assert.True(File.Exists(outPath));
        Assert.True(new FileInfo(outPath).Length > 0);

        using var verifiedDoc = engine.OpenDocument(outPath);
        Assert.Equal(1, verifiedDoc.PageCount);
    }
}
