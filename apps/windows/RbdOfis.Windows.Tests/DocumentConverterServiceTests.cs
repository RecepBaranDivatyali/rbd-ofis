using System;
using System.Collections.Generic;
using System.IO;
using RbdOfis.Interop;
using RbdOfis.Interop.Models;
using RbdOfis.Interop.Native;
using RbdOfis.Interop.Services;
using Xunit;

namespace RbdOfis.Windows.Tests;

public sealed class DocumentConverterServiceTests : IDisposable
{
    private readonly string _tempDir;
    private readonly string _dummyPdf;
    private readonly string _dummyImg;
    private readonly string? _corpusSample1Page;

    public DocumentConverterServiceTests()
    {
        _tempDir = Path.Combine(Path.GetTempPath(), $"rbd_conv_test_{Guid.NewGuid():N}");
        Directory.CreateDirectory(_tempDir);

        _dummyPdf = Path.Combine(_tempDir, "input.pdf");
        File.WriteAllText(_dummyPdf, "%PDF-1.7\n1 0 obj\n<< /Type /Catalog >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF");

        _dummyImg = Path.Combine(_tempDir, "sample.png");
        // Simple 1x1 valid PNG byte payload
        File.WriteAllBytes(_dummyImg, [
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
            0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
            0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
            0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41,
            0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
            0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D,
            0xB0, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
            0x44, 0xAE, 0x42, 0x60, 0x82
        ]);

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
        var service = new DocumentConverterService(new MockNativeRbdApi());
        string outDir = Path.Combine(_tempDir, "img_out");

        Assert.Throws<ArgumentException>(() => service.ConvertPdfToImages("", outDir));
        Assert.Throws<FileNotFoundException>(() => service.ConvertPdfToImages("missing_doc.pdf", outDir));
        Assert.Throws<ArgumentException>(() => service.ConvertPdfToImages(_dummyPdf, ""));
        Assert.Throws<ArgumentException>(() => service.ConvertPdfToImages(_dummyPdf, outDir, new PdfToImageOptionsDto { Dpi = 0 }));

        Assert.Throws<ArgumentException>(() => service.ConvertImagesToPdf([], Path.Combine(_tempDir, "out.pdf")));
        Assert.Throws<FileNotFoundException>(() => service.ConvertImagesToPdf(["missing_image.png"], Path.Combine(_tempDir, "out.pdf")));
        Assert.Throws<ArgumentException>(() => service.ConvertImagesToPdf([_dummyImg], ""));
        Assert.Throws<ArgumentException>(() => service.ConvertImagesToPdf([_dummyImg], Path.Combine(_tempDir, "out.pdf"), new ImageToPdfOptionsDto { MarginPt = -1.0f }));

        Assert.Throws<ArgumentNullException>(() => service.ConvertMarkdownToPdf(null!, Path.Combine(_tempDir, "out.pdf")));
        Assert.Throws<ArgumentException>(() => service.ConvertMarkdownToPdf("# Heading", ""));
    }

    [Fact]
    public void Verify_Mock_Converter_Execution()
    {
        var service = new DocumentConverterService(new MockNativeRbdApi());
        string outDir = Path.Combine(_tempDir, "mock_img_out");

        // 1. Mock PDF to Images
        var images = service.ConvertPdfToImages(_dummyPdf, outDir, new PdfToImageOptionsDto
        {
            Dpi = 150,
            Format = ImageFormatEnum.Png,
            PageRange = [0, 1]
        });

        Assert.Equal(2, images.Count);
        Assert.True(File.Exists(images[0]));
        Assert.True(File.Exists(images[1]));

        // 2. Mock Images to PDF
        string outPdf = Path.Combine(_tempDir, "mock_combined.pdf");
        service.ConvertImagesToPdf(images, outPdf, new ImageToPdfOptionsDto
        {
            PageSize = PageSizeEnum.A4,
            MarginPt = 20.0f
        });
        Assert.True(File.Exists(outPdf));
        Assert.True(new FileInfo(outPdf).Length > 0);

        // 3. Mock Markdown to PDF
        string mdPdf = Path.Combine(_tempDir, "mock_md.pdf");
        service.ConvertMarkdownToPdf("# Mock Title\n\nMock body content.", mdPdf);
        Assert.True(File.Exists(mdPdf));
        Assert.True(new FileInfo(mdPdf).Length > 0);
    }

    [Fact]
    public void Verify_Page_Range_Parser()
    {
        // Null or empty
        Assert.Null(DocumentConverterService.ParsePageRange(null));
        Assert.Null(DocumentConverterService.ParsePageRange(""));
        Assert.Null(DocumentConverterService.ParsePageRange("   "));

        // Single pages (1-based to 0-based conversion)
        var single = DocumentConverterService.ParsePageRange("1");
        Assert.NotNull(single);
        Assert.Equal([0u], single);

        // Multiple discrete pages and range
        var mixed = DocumentConverterService.ParsePageRange("1, 3, 5-7");
        Assert.NotNull(mixed);
        Assert.Equal([0u, 2u, 4u, 5u, 6u], mixed);

        // Semicolon separator and double-dot range
        var dots = DocumentConverterService.ParsePageRange("2..4; 8");
        Assert.NotNull(dots);
        Assert.Equal([1u, 2u, 3u, 7u], dots);

        // Duplicate handling
        var dups = DocumentConverterService.ParsePageRange("1, 1, 2, 2");
        Assert.NotNull(dups);
        Assert.Equal([0u, 1u], dups);
    }

    [Fact]
    public void Verify_Live_Native_Conversion_End_To_End()
    {
        if (_corpusSample1Page == null || !File.Exists(_corpusSample1Page))
        {
            // Skip live test if corpus file unavailable
            return;
        }

        RbdEngine engine;
        try
        {
            engine = new RbdEngine();
        }
        catch
        {
            // Skip if native engine DLL cannot be loaded in current environment
            return;
        }

        using (engine)
        {
            var liveService = new DocumentConverterService(NativeRbdApi.Instance);

            // 1. PDF -> Images
            string imgOutDir = Path.Combine(_tempDir, "live_images");
            var generatedImages = liveService.ConvertPdfToImages(_corpusSample1Page, imgOutDir, new PdfToImageOptionsDto
            {
                Dpi = 150,
                Format = ImageFormatEnum.Png,
                PageRange = [0]
            });

            Assert.NotEmpty(generatedImages);
            Assert.True(File.Exists(generatedImages[0]));
            Assert.True(new FileInfo(generatedImages[0]).Length > 0);

            // 2. Images -> PDF
            string outCombinedPdf = Path.Combine(_tempDir, "live_from_images.pdf");
            liveService.ConvertImagesToPdf(generatedImages, outCombinedPdf, new ImageToPdfOptionsDto
            {
                PageSize = PageSizeEnum.A4,
                MarginPt = 18.0f
            });

            Assert.True(File.Exists(outCombinedPdf));
            Assert.True(new FileInfo(outCombinedPdf).Length > 0);
            using (var docFromImg = engine.OpenDocument(outCombinedPdf))
            {
                Assert.True(docFromImg.PageCount >= 1);
            }

            // 3. Markdown -> PDF
            string outMdPdf = Path.Combine(_tempDir, "live_from_md.pdf");
            string markdown = "# RBD OFİS\n\nBu bir test belgesidir.\n\n- Madde 1\n- Madde 2";
            liveService.ConvertMarkdownToPdf(markdown, outMdPdf);

            Assert.True(File.Exists(outMdPdf));
            Assert.True(new FileInfo(outMdPdf).Length > 0);
            using (var docFromMd = engine.OpenDocument(outMdPdf))
            {
                Assert.True(docFromMd.PageCount >= 1);
            }
        }
    }
}
