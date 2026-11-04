using System;
using System.IO;
using System.Runtime.InteropServices;
using RbdOfis.Interop.Native;
using Xunit;

namespace RbdOfis.Windows.Tests;

public class NativeInteropTests
{
    [Fact]
    public void Verify_ZeroCopy_PinnedBuffer_Rendering_Via_ByteArray()
    {
        var mockApi = new MockNativeRbdApi();
        using var engine = new RbdEngine(mockApi);
        using var doc = engine.OpenDocument(new byte[] { 1, 2, 3, 4 });

        const int tileW = 256;
        const int tileH = 256;
        const int stride = tileW * 4;
        byte[] destinationBuffer = new byte[stride * tileH];

        // All elements are initially 0
        Assert.All(destinationBuffer, b => Assert.Equal(0, b));

        // Call zero-copy pinned buffer render
        doc.RenderTileIntoBuffer(
            pageIndex: 0,
            tileX: 0,
            tileY: 0,
            tileWidth: tileW,
            tileHeight: tileH,
            zoom: 1.0,
            destinationBuffer: destinationBuffer,
            stride: stride,
            format: RbdPixelFormat.Bgra32);

        // Verify the mock native provider wrote directly into the pinned memory address
        Assert.Equal(1, mockApi.RenderCallCount);
        Assert.Equal(stride, mockApi.LastStride);
        Assert.Equal(tileW, mockApi.LastTileWidth);
        Assert.Equal(tileH, mockApi.LastTileHeight);

        // Verify destination buffer received pixels directly
        int nonZeroBytes = 0;
        for (int i = 0; i < destinationBuffer.Length; i++)
        {
            if (destinationBuffer[i] != 0) nonZeroBytes++;
        }

        Assert.True(nonZeroBytes > 0, "Destination buffer remained empty; zero-copy write failed.");
    }

    [Fact]
    public void Verify_ZeroCopy_Direct_Pointer_Rendering_Simulating_WriteableBitmap()
    {
        var mockApi = new MockNativeRbdApi();
        using var engine = new RbdEngine(mockApi);
        using var doc = engine.OpenDocument(new byte[] { 1, 2, 3, 4 });

        const int tileW = 128;
        const int tileH = 128;
        const int stride = tileW * 4;
        int bufferBytes = stride * tileH;

        // Allocate unmanaged memory exactly like WriteableBitmap.BackBuffer
        IntPtr backBuffer = Marshal.AllocHGlobal(bufferBytes);
        try
        {
            // Zero-fill buffer
            unsafe
            {
                byte* ptr = (byte*)backBuffer.ToPointer();
                for (int i = 0; i < bufferBytes; i++) ptr[i] = 0;
            }

            // Zero-copy render directly into native memory
            doc.RenderTileIntoBuffer(
                pageIndex: 5,
                tileX: 1,
                tileY: 2,
                tileWidth: tileW,
                tileHeight: tileH,
                zoom: 2.0,
                backBufferPtr: backBuffer,
                stride: stride,
                format: RbdPixelFormat.Bgra32);

            Assert.Equal(backBuffer, mockApi.LastRenderedBuffer);

            // Verify unmanaged memory contents
            int nonZeroBytes = 0;
            unsafe
            {
                byte* ptr = (byte*)backBuffer.ToPointer();
                for (int i = 0; i < bufferBytes; i++)
                {
                    if (ptr[i] != 0) nonZeroBytes++;
                }
            }

            Assert.True(nonZeroBytes > 0, "Unmanaged backbuffer did not receive rendered pixels.");
        }
        finally
        {
            Marshal.FreeHGlobal(backBuffer);
        }
    }

    [Fact]
    public void Verify_SafeHandles_Deterministic_Disposal()
    {
        var mockApi = new MockNativeRbdApi();
        RbdDocument doc;
        using (var engine = new RbdEngine(mockApi))
        {
            Assert.False(engine.Handle.IsInvalid);
            Assert.False(engine.Handle.IsClosed);

            doc = engine.OpenDocument(new byte[] { 1, 2, 3 });
            Assert.False(doc.Handle.IsInvalid);
            Assert.False(doc.Handle.IsClosed);
            Assert.Equal(2000, doc.PageCount);
        }

        // Dispose document
        doc.Dispose();
        Assert.True(doc.Handle.IsClosed);

        // After disposal, methods must throw ObjectDisposedException
        Assert.Throws<ObjectDisposedException>(() => doc.PageCount);
        Assert.Throws<ObjectDisposedException>(() => doc.GetPageSize(0));
    }

    [Fact]
    public void Verify_SmokeRunner_Report_Succeeds()
    {
        var report = RbdNativeSmokeRunner.Run(forceMock: true);

        Assert.True(report.IsSuccess);
        Assert.True(report.ZeroCopyVerified);
        Assert.True(report.RenderedBytesCount > 0);
        Assert.NotEmpty(report.Details);
    }

    [Fact]
    public void Verify_Live_Native_Rbd_Ffi_Rendering_Real_Pdf()
    {
        // 1. Locate real PDF file from corpus
        string baseDir = AppContext.BaseDirectory;
        string[] candidates =
        [
            Path.Combine(baseDir, "..", "..", "..", "..", "..", "corpus", "files", "pdf-basic", "pdfium-hello-world.pdf"),
            Path.Combine(baseDir, "..", "..", "..", "..", "..", "..", "corpus", "files", "pdf-basic", "pdfium-hello-world.pdf"),
            Path.Combine(Directory.GetCurrentDirectory(), "corpus", "files", "pdf-basic", "pdfium-hello-world.pdf"),
        ];

        string? foundPath = null;
        foreach (var c in candidates)
        {
            string full = Path.GetFullPath(c);
            if (File.Exists(full))
            {
                foundPath = full;
                break;
            }
        }

        Assert.NotNull(foundPath);
        Assert.True(File.Exists(foundPath), $"Real PDF must exist at: {foundPath}");

        // 2. Initialize live native engine dispatching to real rbd_ffi.dll
        using var engine = new RbdEngine(NativeRbdApi.Instance);
        Assert.False(engine.Handle.IsInvalid);

        // 3. Open real PDF
        using var doc = engine.OpenDocument(foundPath);
        Assert.False(doc.Handle.IsInvalid);
        Assert.True(doc.PageCount >= 1, "Page count must be >= 1");

        var (width, height) = doc.GetPageSize(0);
        Assert.True(width > 0.0 && height > 0.0, "Page size must be positive");

        // 4. Render 256x256 tile with Bgra32 into pinned byte array (zero-copy)
        const int tileW = 256;
        const int tileH = 256;
        const int stride = tileW * 4;
        byte[] tileBuffer = new byte[stride * tileH];

        doc.RenderTileIntoBuffer(
            pageIndex: 0,
            tileX: 0,
            tileY: 0,
            tileWidth: tileW,
            tileHeight: tileH,
            zoom: 1.0,
            destinationBuffer: tileBuffer,
            stride: stride,
            format: RbdPixelFormat.Bgra32);

        // 5. Assert rendered bytes contain real non-zero pixels
        int nonZero = 0;
        for (int i = 0; i < tileBuffer.Length; i++)
        {
            if (tileBuffer[i] != 0) nonZero++;
        }

        Assert.True(nonZero > 0, "Live native rendering wrote 0 bytes to buffer");
    }
}
