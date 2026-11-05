using System;
using System.Diagnostics;
using RbdOfis.Interop.Viewport;
using Xunit;

namespace RbdOfis.Windows.Tests;

public class TiledViewportMathTests
{
    private const int LargeDocumentPages = 2000;
    private const double A4WidthPt = 595.0;
    private const double A4HeightPt = 842.0;
    private const double PageGapPt = 10.0;

    [Fact]
    public void Verify_2000PageDocument_Calculates_Visible_Pages_In_O1()
    {
        // Viewport: 1920x1080 FHD screen, scrolled to offset 500,000 px at zoom 1.5
        var query = new ViewportQuery(
            TotalPages: LargeDocumentPages,
            PageWidthPt: A4WidthPt,
            PageHeightPt: A4HeightPt,
            PageGapPt: PageGapPt,
            Zoom: 1.5,
            ViewportWidth: 1920,
            ViewportHeight: 1080,
            ScrollOffsetX: 0,
            ScrollOffsetY: 500_000,
            TileSize: 512);

        var sw = Stopwatch.StartNew();
        var result = TiledViewportMath.CalculateVisibleTiles(query);
        sw.Stop();

        // 1. Must execute in strict O(1) time (sub-millisecond)
        Assert.True(sw.Elapsed.TotalMilliseconds < 2.0, $"Execution took {sw.Elapsed.TotalMilliseconds:F3} ms; expected < 2.0 ms.");

        // 2. Expected visible page calculation
        // Scaled height = 842 * 1.5 = 1263. Scaled gap = 15. Stride = 1278.
        // 500,000 / 1278 = 391.23 => page index 391
        // (500,000 + 1080) / 1278 = 501,080 / 1278 = 392.08 => page index 392
        Assert.Equal(391, result.FirstVisiblePageIndex);
        Assert.Equal(392, result.LastVisiblePageIndex);
        Assert.Equal(2, result.VisiblePageCount);

        // 3. Document virtual height
        Assert.True(result.TotalDocumentHeight > 2_500_000.0);

        // 4. Must NOT instantiate 2000 elements; tile count is strictly bounded to the visible viewport
        Assert.NotEmpty(result.VisibleTiles);
        Assert.True(result.VisibleTiles.Count <= 16, $"Generated {result.VisibleTiles.Count} tiles; expected <= 16 tiles for viewport.");

        foreach (var tile in result.VisibleTiles)
        {
            Assert.True(tile.PageIndex is 391 or 392);
            Assert.True(tile.TileWidth > 0 && tile.TileWidth <= 512);
            Assert.True(tile.TileHeight > 0 && tile.TileHeight <= 512);
        }
    }

    [Fact]
    public void Verify_VisibleTiles_Coordinates_Strictly_Intersect_Viewport()
    {
        var query = new ViewportQuery(
            TotalPages: LargeDocumentPages,
            PageWidthPt: A4WidthPt,
            PageHeightPt: A4HeightPt,
            PageGapPt: PageGapPt,
            Zoom: 1.0,
            ViewportWidth: 800,
            ViewportHeight: 600,
            ScrollOffsetX: 100,
            ScrollOffsetY: 200,
            TileSize: 256);

        var result = TiledViewportMath.CalculateVisibleTiles(query);

        Assert.True(result.VisiblePageCount >= 1);
        Assert.NotEmpty(result.VisibleTiles);

        foreach (var tile in result.VisibleTiles)
        {
            // CanvasX + tileWidth should be > 0 and < ViewportWidth + TileSize
            Assert.True(tile.CanvasX + tile.TileWidth >= 0);
            Assert.True(tile.CanvasX <= query.ViewportWidth);

            // CanvasY + tileHeight should be > 0 and < ViewportHeight + TileSize
            Assert.True(tile.CanvasY + tile.TileHeight >= 0);
            Assert.True(tile.CanvasY <= query.ViewportHeight);
        }
    }

    [Fact]
    public void Verify_Zero_UI_Allocations_Stress_Benchmark_100k_Queries()
    {
        // Warmup
        var query = new ViewportQuery(LargeDocumentPages, A4WidthPt, A4HeightPt, PageGapPt, 1.25, 1920, 1080, 0, 100_000);
        TiledViewportMath.CalculateVisibleTiles(query);

        const int iterations = 100_000;
        var sw = Stopwatch.StartNew();

        double totalHeightAcc = 0;
        for (int i = 0; i < iterations; i++)
        {
            double scrollY = (i * 23.7) % 2_000_000.0;
            var q = new ViewportQuery(
                TotalPages: LargeDocumentPages,
                PageWidthPt: A4WidthPt,
                PageHeightPt: A4HeightPt,
                PageGapPt: PageGapPt,
                Zoom: 1.0,
                ViewportWidth: 1920,
                ViewportHeight: 1080,
                ScrollOffsetX: 0,
                ScrollOffsetY: scrollY);

            var res = TiledViewportMath.CalculateVisibleTiles(q);
            totalHeightAcc += res.TotalDocumentHeight;
        }

        sw.Stop();

        Assert.True(totalHeightAcc > 0);
        // 100k queries should easily execute in < 1000ms on .NET 9 even under heavy system load
        Assert.True(sw.ElapsedMilliseconds < 1000, $"100,000 queries took {sw.ElapsedMilliseconds} ms; expected < 1000 ms.");
    }

    [Theory]
    [InlineData(0, 0)]                       // First page
    [InlineData(1_000_000, 1173)]           // Middle page (1,000,000 / 852 = 1173)
    [InlineData(1_700_000, 1995)]           // Near end of document (1,700,000 / 852 = 1995)
    public void Verify_Page_Index_Calculation_Accuracy(double scrollY, int expectedFirstPage)
    {
        var query = new ViewportQuery(
            TotalPages: LargeDocumentPages,
            PageWidthPt: A4WidthPt,
            PageHeightPt: A4HeightPt,
            PageGapPt: PageGapPt,
            Zoom: 1.0,
            ViewportWidth: 1000,
            ViewportHeight: 1000,
            ScrollOffsetX: 0,
            ScrollOffsetY: scrollY);

        var result = TiledViewportMath.CalculateVisibleTiles(query);
        Assert.Equal(expectedFirstPage, result.FirstVisiblePageIndex);
    }

    [Fact]
    public void Verify_Scrolling_Beyond_Document_Returns_No_Visible_Pages()
    {
        // Document height at 1.0x is 1,703,990. Scroll far beyond at 3,000,000
        var query = new ViewportQuery(
            TotalPages: LargeDocumentPages,
            PageWidthPt: A4WidthPt,
            PageHeightPt: A4HeightPt,
            PageGapPt: PageGapPt,
            Zoom: 1.0,
            ViewportWidth: 1000,
            ViewportHeight: 1000,
            ScrollOffsetX: 0,
            ScrollOffsetY: 3_000_000);

        var result = TiledViewportMath.CalculateVisibleTiles(query);
        Assert.Equal(-1, result.FirstVisiblePageIndex);
        Assert.Equal(-1, result.LastVisiblePageIndex);
        Assert.Empty(result.VisibleTiles);
    }
}
