using System;
using System.IO;
using RbdOfis.Interop.Services;
using Xunit;

namespace RbdOfis.Windows.Tests;

public class OfficeDocumentServiceTests : IDisposable
{
    private readonly string _tempDir;
    private readonly OfficeDocumentService _service;

    public OfficeDocumentServiceTests()
    {
        _tempDir = Path.Combine(Path.GetTempPath(), $"rbd_office_test_{Guid.NewGuid():N}");
        Directory.CreateDirectory(_tempDir);
        _service = new OfficeDocumentService();
    }

    public void Dispose()
    {
        if (Directory.Exists(_tempDir))
        {
            try { Directory.Delete(_tempDir, true); } catch { }
        }
    }

    [Fact]
    public void IsSupportedOfficeFile_Recognizes_Office_Extensions()
    {
        Assert.True(OfficeDocumentService.IsSupportedOfficeFile("test.docx"));
        Assert.True(OfficeDocumentService.IsSupportedOfficeFile("SHEET.XLSX"));
        Assert.True(OfficeDocumentService.IsSupportedOfficeFile("presentation.pptx"));
        Assert.True(OfficeDocumentService.IsSupportedOfficeFile("legacy.doc"));
        Assert.True(OfficeDocumentService.IsSupportedOfficeFile("legacy.xls"));
        Assert.True(OfficeDocumentService.IsSupportedOfficeFile("legacy.ppt"));

        Assert.False(OfficeDocumentService.IsSupportedOfficeFile("document.pdf"));
        Assert.False(OfficeDocumentService.IsSupportedOfficeFile("image.png"));
        Assert.False(OfficeDocumentService.IsSupportedOfficeFile("notes.txt"));
        Assert.False(OfficeDocumentService.IsSupportedOfficeFile(""));
    }

    [Fact]
    public void CreateDocx_And_ExtractText_RoundTrip()
    {
        string docxPath = Path.Combine(_tempDir, "sample.docx");
        string sampleText = "RBD OFİS Çevrimdışı Düzenleyici\nİkinci paragraf: çşğüöı Türkçe metin.\nÜçüncü satır.";

        _service.CreateDocx(sampleText, docxPath);
        Assert.True(File.Exists(docxPath));

        string extracted = _service.ExtractText(docxPath);
        Assert.Contains("RBD OFİS Çevrimdışı Düzenleyici", extracted);
        Assert.Contains("Türkçe metin", extracted);
        Assert.Contains("çşğüöı", extracted);
    }

    [Fact]
    public void Inspect_Returns_Accurate_Metadata()
    {
        string docxPath = Path.Combine(_tempDir, "metadata_test.docx");
        string sampleText = "Birinci paragraf.\nİkinci paragraf burada yer alıyor.";

        _service.CreateDocx(sampleText, docxPath);
        var info = _service.Inspect(docxPath);

        Assert.Equal("DOCX", info.DocType);
        Assert.Equal("metadata_test", info.Title);
        Assert.Equal(2, info.ItemCount);
        Assert.True(info.WordCount >= 6);
        Assert.Contains("Birinci", info.Summary);
    }

    [Fact]
    public void ExtractText_Throws_On_Missing_File()
    {
        string missing = Path.Combine(_tempDir, "non_existent.docx");
        Assert.Throws<FileNotFoundException>(() => _service.ExtractText(missing));
    }

    [Fact]
    public void CreateDocx_Throws_On_Null_Or_Empty_Path()
    {
        Assert.Throws<ArgumentException>(() => _service.CreateDocx("test", ""));
    }
}
