using System;
using System.Collections.Generic;
using System.IO;
using RbdOfis.Interop.Models;
using RbdOfis.Interop.Native;
using RbdOfis.Interop.Services;
using Xunit;

namespace RbdOfis.Windows.Tests;

public sealed class ProPdfServiceTests : IDisposable
{
    private readonly string _tempDir;
    private readonly string _dummyPdf;
    private readonly string? _corpusSample1Page;

    public ProPdfServiceTests()
    {
        _tempDir = Path.Combine(Path.GetTempPath(), $"rbd_pro_test_{Guid.NewGuid():N}");
        Directory.CreateDirectory(_tempDir);

        _dummyPdf = Path.Combine(_tempDir, "input.pdf");
        File.WriteAllText(_dummyPdf, "%PDF-1.7\n1 0 obj\n<< /Type /Catalog >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF");

        string current = AppContext.BaseDirectory;
        for (int i = 0; i < 6; i++)
        {
            string candidate = Path.Combine(current, "corpus", "files", "pdf-basic", "sample-1page.pdf");
            if (File.Exists(candidate))
            {
                _corpusSample1Page = Path.GetFullPath(candidate);
                break;
            }
            string? parent = Directory.GetParent(current)?.FullName;
            if (parent == null) break;
            current = parent;
        }
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

    [Fact]
    public void Verify_Service_Validates_Null_And_Missing_Files()
    {
        var service = new ProPdfService(new MockNativeRbdApi());

        Assert.Throws<ArgumentException>(() => service.Redact("", "out.pdf", [new PageRedactionDto()]));
        Assert.Throws<ArgumentException>(() => service.Compress(_dummyPdf, "", new CompressionOptionsDto()));
        Assert.Throws<FileNotFoundException>(() => service.Compress("missing_file.pdf", "out.pdf"));
        Assert.Throws<ArgumentException>(() => service.ApplyWatermark(_dummyPdf, "out.pdf", new WatermarkOptionsDto { Text = "" }));
        Assert.Throws<ArgumentException>(() => service.ApplyPageNumbers(_dummyPdf, "out.pdf", new PageNumberOptionsDto { Template = "" }));
        Assert.Throws<FileNotFoundException>(() => service.ReadFormFields("missing_file.pdf"));
    }

    [Fact]
    public void Verify_Mock_Compress_Returns_Valid_Metrics()
    {
        var service = new ProPdfService(new MockNativeRbdApi());
        string outPath = Path.Combine(_tempDir, "comp.pdf");

        var result = service.Compress(_dummyPdf, outPath, new CompressionOptionsDto());

        Assert.True(File.Exists(outPath));
        Assert.True(result.OriginalSize > 0);
        Assert.True(result.CompressedSize > 0);
        Assert.True(result.SavingsPercent > 0);
    }

    [Fact]
    public void Verify_Mock_Watermark_Execution()
    {
        var service = new ProPdfService(new MockNativeRbdApi());
        string outPath = Path.Combine(_tempDir, "watermarked.pdf");

        service.ApplyWatermark(_dummyPdf, outPath, new WatermarkOptionsDto
        {
            Text = "TOP SECRET",
            FontSize = 36.0f,
            RotationDegrees = 30.0f,
            Opacity = 0.5f
        });

        Assert.True(File.Exists(outPath));
        Assert.True(new FileInfo(outPath).Length > 0);
    }

    [Fact]
    public void Verify_Mock_PageNumbers_And_Bates_Execution()
    {
        var service = new ProPdfService(new MockNativeRbdApi());

        string outNum = Path.Combine(_tempDir, "numbered.pdf");
        service.ApplyPageNumbers(_dummyPdf, outNum, new PageNumberOptionsDto
        {
            Template = "Page {page} / {total}",
            Position = StampPosition.BottomCenter
        });
        Assert.True(File.Exists(outNum));

        string outBates = Path.Combine(_tempDir, "bates.pdf");
        service.ApplyBatesStamp(_dummyPdf, outBates, new BatesOptionsDto
        {
            Prefix = "TEST-",
            StartNumber = 100,
            PaddingDigits = 5,
            Position = StampPosition.BottomRight
        });
        Assert.True(File.Exists(outBates));
    }

    [Fact]
    public void Verify_Mock_FormFields_Read_And_Fill()
    {
        var service = new ProPdfService(new MockNativeRbdApi());

        var fields = service.ReadFormFields(_dummyPdf);
        Assert.NotEmpty(fields);
        Assert.Equal("sample_field", fields[0].Name);

        string outForm = Path.Combine(_tempDir, "filled.pdf");
        service.FillFormFields(_dummyPdf, outForm, new Dictionary<string, string> { ["sample_field"] = "Custom Value" }, flatten: true);
        Assert.True(File.Exists(outForm));
    }

    [Fact]
    public void Verify_Live_ProPdfService_With_Corpus_Pdf()
    {
        if (string.IsNullOrEmpty(_corpusSample1Page) || !File.Exists(_corpusSample1Page))
        {
            return;
        }

        var liveService = new ProPdfService(NativeRbdApi.Instance);
        using var engine = new RbdEngine(NativeRbdApi.Instance);

        // 1. Live Compression
        string outComp = Path.Combine(_tempDir, "live_comp.pdf");
        var compResult = liveService.Compress(_corpusSample1Page, outComp, new CompressionOptionsDto
        {
            RemoveUnusedObjects = true,
            RecompressStreams = true,
            StripMetadata = false
        });
        Assert.True(File.Exists(outComp));
        Assert.True(compResult.OriginalSize > 0);
        using (var docComp = engine.OpenDocument(outComp))
        {
            Assert.True(docComp.PageCount >= 1);
        }

        // 2. Live Watermark
        string outWm = Path.Combine(_tempDir, "live_wm.pdf");
        liveService.ApplyWatermark(_corpusSample1Page, outWm, new WatermarkOptionsDto
        {
            Text = "LIVE WATERMARK",
            FontSize = 36.0f,
            RotationDegrees = 45.0f,
            Opacity = 0.3f,
            ColorRgb = [200, 200, 200],
            OnTop = true
        });
        Assert.True(File.Exists(outWm));
        using (var docWm = engine.OpenDocument(outWm))
        {
            Assert.True(docWm.PageCount >= 1);
        }

        // 3. Live Page Numbers
        string outNum = Path.Combine(_tempDir, "live_num.pdf");
        liveService.ApplyPageNumbers(_corpusSample1Page, outNum, new PageNumberOptionsDto
        {
            Template = "Sayfa {page} / {total}",
            Position = StampPosition.BottomCenter,
            FontSize = 10.0f,
            Margin = 36.0f
        });
        Assert.True(File.Exists(outNum));
        using (var docNum = engine.OpenDocument(outNum))
        {
            Assert.True(docNum.PageCount >= 1);
        }

        // 4. Live Bates Stamp
        string outBates = Path.Combine(_tempDir, "live_bates.pdf");
        liveService.ApplyBatesStamp(_corpusSample1Page, outBates, new BatesOptionsDto
        {
            Prefix = "LEGAL-",
            StartNumber = 1,
            PaddingDigits = 6,
            Suffix = "",
            Position = StampPosition.BottomRight
        });
        Assert.True(File.Exists(outBates));
        using (var docBates = engine.OpenDocument(outBates))
        {
            Assert.True(docBates.PageCount >= 1);
        }

        // 5. Live Redaction
        string outRedact = Path.Combine(_tempDir, "live_redact.pdf");
        var redactDto = new PageRedactionDto
        {
            PageIndex = 0,
            Rects = [new PdfRect(50, 650, 300, 750)],
            FillColor = [0, 0, 0]
        };
        liveService.Redact(_corpusSample1Page, outRedact, [redactDto]);
        Assert.True(File.Exists(outRedact));
        using (var docRedact = engine.OpenDocument(outRedact))
        {
            Assert.True(docRedact.PageCount >= 1);
        }
    }
}
