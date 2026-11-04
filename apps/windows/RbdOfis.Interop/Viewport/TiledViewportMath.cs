using System;
using System.Collections.Generic;

namespace RbdOfis.Interop.Viewport;

/// <summary>
/// Immutable descriptor of a single visible tile within a page.
/// </summary>
public readonly record struct VisibleTile(
    int PageIndex,
    int TileX,
    int TileY,
    int TileWidth,
    int TileHeight,
    double CanvasX,
    double CanvasY);

/// <summary>
/// Parameters for querying visible pages and tiles in the document viewport.
/// </summary>
public sealed record ViewportQuery(
    int TotalPages,
    double PageWidthPt,
    double PageHeightPt,
    double PageGapPt,
    double Zoom,
    double ViewportWidth,
    double ViewportHeight,
    double ScrollOffsetX,
    double ScrollOffsetY,
    int TileSize = 512);

/// <summary>
/// Result of the O(1) viewport calculation.
/// </summary>
public sealed class ViewportCalculationResult
{
    public int FirstVisiblePageIndex { get; init; }
    public int LastVisiblePageIndex { get; init; }
    public int VisiblePageCount => (LastVisiblePageIndex >= FirstVisiblePageIndex && FirstVisiblePageIndex >= 0)
        ? (LastVisiblePageIndex - FirstVisiblePageIndex + 1)
        : 0;
    public double TotalDocumentHeight { get; init; }
    public double TotalDocumentWidth { get; init; }
    public IReadOnlyList<VisibleTile> VisibleTiles { get; init; } = Array.Empty<VisibleTile>();
}

/// <summary>
/// High-performance O(1) viewport mathematics for large documents (e.g. 2000+ pages).
/// Computes visible page indices and tile coordinates strictly using closed-form arithmetic
/// without instantiating UI elements or looping through non-visible pages.
/// </summary>
public static class TiledViewportMath
{
    public static ViewportCalculationResult CalculateVisibleTiles(ViewportQuery query)
    {
        ArgumentNullException.ThrowIfNull(query);

        if (query.TotalPages <= 0 || query.Zoom <= 0.0 || query.ViewportWidth <= 0.0 || query.ViewportHeight <= 0.0)
        {
            return new ViewportCalculationResult
            {
                FirstVisiblePageIndex = -1,
                LastVisiblePageIndex = -1,
                TotalDocumentHeight = 0.0,
                TotalDocumentWidth = 0.0,
                VisibleTiles = Array.Empty<VisibleTile>()
            };
        }

        double scaledPageWidth = query.PageWidthPt * query.Zoom;
        double scaledPageHeight = query.PageHeightPt * query.Zoom;
        double scaledGap = query.PageGapPt * query.Zoom;
        double pageStrideY = scaledPageHeight + scaledGap;
        double totalDocHeight = (query.TotalPages * pageStrideY) - scaledGap;
        double totalDocWidth = scaledPageWidth;

        double topY = Math.Max(0.0, query.ScrollOffsetY);
        double bottomY = Math.Min(totalDocHeight, query.ScrollOffsetY + query.ViewportHeight);

        if (topY >= totalDocHeight || bottomY <= 0.0 || topY >= bottomY)
        {
            return new ViewportCalculationResult
            {
                FirstVisiblePageIndex = -1,
                LastVisiblePageIndex = -1,
                TotalDocumentHeight = totalDocHeight,
                TotalDocumentWidth = totalDocWidth,
                VisibleTiles = Array.Empty<VisibleTile>()
            };
        }

        // Strict O(1) mathematical calculation for visible page bounds:
        int firstPageIndex = (int)(topY / pageStrideY);
        int lastPageIndex = (int)(bottomY / pageStrideY);

        if (firstPageIndex >= query.TotalPages) firstPageIndex = query.TotalPages - 1;
        if (lastPageIndex >= query.TotalPages) lastPageIndex = query.TotalPages - 1;
        if (firstPageIndex < 0) firstPageIndex = 0;
        if (lastPageIndex < firstPageIndex) lastPageIndex = firstPageIndex;

        int tileSize = query.TileSize > 0 ? query.TileSize : 512;
        var visibleTiles = new List<VisibleTile>(16);

        // Loop runs ONLY over visible pages (typically 1 to 3 pages max, strictly O(1) bound)
        for (int p = firstPageIndex; p <= lastPageIndex; p++)
        {
            double pageTop = p * pageStrideY;
            double pageBottom = pageTop + scaledPageHeight;

            // Region of the page intersecting the viewport vertically
            double visPageTop = Math.Max(0.0, topY - pageTop);
            double visPageBottom = Math.Min(scaledPageHeight, bottomY - pageTop);

            if (visPageBottom <= visPageTop)
            {
                // In gap between pages
                continue;
            }

            // Region of the page intersecting the viewport horizontally
            double visPageLeft = Math.Max(0.0, query.ScrollOffsetX);
            double visPageRight = Math.Min(scaledPageWidth, query.ScrollOffsetX + query.ViewportWidth);

            if (visPageRight <= visPageLeft)
            {
                continue;
            }

            int minTileX = (int)(visPageLeft / tileSize);
            int maxTileX = (int)((visPageRight - 0.0001) / tileSize);
            int minTileY = (int)(visPageTop / tileSize);
            int maxTileY = (int)((visPageBottom - 0.0001) / tileSize);

            int maxPossibleTilesX = (int)Math.Ceiling(scaledPageWidth / tileSize) - 1;
            int maxPossibleTilesY = (int)Math.Ceiling(scaledPageHeight / tileSize) - 1;

            if (maxTileX > maxPossibleTilesX) maxTileX = maxPossibleTilesX;
            if (maxTileY > maxPossibleTilesY) maxTileY = maxPossibleTilesY;

            for (int ty = minTileY; ty <= maxTileY; ty++)
            {
                for (int tx = minTileX; tx <= maxTileX; tx++)
                {
                    int tilePixelW = (int)Math.Min(tileSize, Math.Ceiling(scaledPageWidth - (tx * tileSize)));
                    int tilePixelH = (int)Math.Min(tileSize, Math.Ceiling(scaledPageHeight - (ty * tileSize)));

                    double canvasX = (tx * tileSize) - query.ScrollOffsetX;
                    double canvasY = (pageTop + (ty * tileSize)) - query.ScrollOffsetY;

                    visibleTiles.Add(new VisibleTile(
                        PageIndex: p,
                        TileX: tx,
                        TileY: ty,
                        TileWidth: tilePixelW,
                        TileHeight: tilePixelH,
                        CanvasX: canvasX,
                        CanvasY: canvasY));
                }
            }
        }

        return new ViewportCalculationResult
        {
            FirstVisiblePageIndex = firstPageIndex,
            LastVisiblePageIndex = lastPageIndex,
            TotalDocumentHeight = totalDocHeight,
            TotalDocumentWidth = totalDocWidth,
            VisibleTiles = visibleTiles
        };
    }
}
