using System;
using System.Diagnostics;
using System.IO;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using Microsoft.Web.WebView2.Core;
using RbdOfis.Interop.Localization;
using RbdOfis.Interop.Native;
using RbdOfis.Interop.Security;
using RbdOfis.Interop.Viewport;

namespace RbdOfis.Windows;

public partial class MainWindow : Window
{
    private const int TotalPagesDemo = 2000;
    private const double PageWidthPt = 595.0;  // Standard A4 width
    private const double PageHeightPt = 842.0; // Standard A4 height
    private const double PageGapPt = 15.0;

    private readonly RbdEngine _engine;
    private readonly RbdDocument _document;
    private double _zoom = 1.0;
    private WriteableBitmap? _writeableBitmap;

    public MainWindow()
    {
        InitializeComponent();

        // 1. Initialize Interop Engine & Document
        _engine = new RbdEngine(new MockNativeRbdApi
        {
            ConfiguredPageCount = TotalPagesDemo,
            ConfiguredPageWidth = PageWidthPt,
            ConfiguredPageHeight = PageHeightPt
        });
        _document = _engine.OpenDocument(new byte[] { 0x25, 0x50, 0x44, 0x46 }); // "%PDF"

        // 2. Wire localization events
        LocalizationManager.Instance.CultureChanged += (_, _) => UpdateLocalizedUi();
        UpdateLocalizedUi();

        Loaded += OnWindowLoaded;
    }

    private async void OnWindowLoaded(object sender, RoutedEventArgs e)
    {
        // Initial render of tiled canvas
        RenderTiledViewport();

        // Initialize WebView2 offline virtual host
        await InitializeWebView2Async();
    }

    private void UpdateLocalizedUi()
    {
        var lm = LocalizationManager.Instance;
        Title = lm["AppTitle"] + " — " + lm["AppSubtitle"];
        AppTitleText.Text = lm["AppTitle"];
        AppSubtitleText.Text = lm["AppSubtitle"];
        LanguageLabelText.Text = lm["LanguageLabel"];
        PdfTabHeader.Header = lm["TabPdfViewer"];
        OfficeTabHeader.Header = lm["TabOfficeEditor"];
        OpenButton.Content = lm["MenuOpen"];
        PrevPageButton.Content = lm["PrevPage"];
        NextPageButton.Content = lm["NextPageButtonLabel"];
        ZoomOutButton.ToolTip = lm["TooltipZoomOut"];
        ZoomInButton.ToolTip = lm["TooltipZoomIn"];
        ZoomFitWidthButton.Content = lm["ZoomFitWidth"];
        SmokeTestButton.Content = lm["ButtonSmokeTest"];
        OfflineFilterLabelText.Text = lm["OfflineFilterTestLabel"];
        TestUrlButton.Content = lm["TestNavigate"];

        StatsZeroCopyText.Text = lm["StatsZeroCopy"] + " (Direct2D BackBuffer)";
        StatusMessageText.Text = lm["StatusReady"] + " • " + lm["StatusOfflineSecure"];
        SecuritySandboxText.Text = "🔒 " + lm["SecuritySandboxActive"] + " • " + OfflineSecurityPolicy.AllowedOrigin;
        UpdatePageDisplay(1);
    }

    private void UpdatePageDisplay(int currentPage)
    {
        var lm = LocalizationManager.Instance;
        PageStatusText.Text = lm.GetString("PageOf", currentPage, TotalPagesDemo);
        ZoomText.Text = $"{(int)(_zoom * 100)}%";
    }

    private void RenderTiledViewport()
    {
        if (_document == null || PdfScrollViewer == null) return;

        double viewportW = Math.Max(PdfScrollViewer.ViewportWidth, 800);
        double viewportH = Math.Max(PdfScrollViewer.ViewportHeight, 600);
        double scrollX = PdfScrollViewer.HorizontalOffset;
        double scrollY = PdfScrollViewer.VerticalOffset;

        var query = new ViewportQuery(
            TotalPages: TotalPagesDemo,
            PageWidthPt: PageWidthPt,
            PageHeightPt: PageHeightPt,
            PageGapPt: PageGapPt,
            Zoom: _zoom,
            ViewportWidth: viewportW,
            ViewportHeight: viewportH,
            ScrollOffsetX: scrollX,
            ScrollOffsetY: scrollY,
            TileSize: 512);

        var sw = Stopwatch.StartNew();
        var result = TiledViewportMath.CalculateVisibleTiles(query);

        // Adjust virtual document canvas height so the scroll viewer has correct scroll bounds
        VirtualDocumentCanvas.Height = Math.Max(result.TotalDocumentHeight, viewportH);
        VirtualDocumentCanvas.Width = Math.Max(result.TotalDocumentWidth, viewportW);

        // Ensure WriteableBitmap matches viewport dimensions
        int bmpW = (int)Math.Ceiling(viewportW);
        int bmpH = (int)Math.Ceiling(viewportH);

        if (_writeableBitmap == null || _writeableBitmap.PixelWidth != bmpW || _writeableBitmap.PixelHeight != bmpH)
        {
            _writeableBitmap = new WriteableBitmap(bmpW, bmpH, 96, 96, PixelFormats.Bgra32, null);
            CanvasImage.Source = _writeableBitmap;
        }

        // Zero-copy direct pointer rendering into WriteableBitmap.BackBuffer
        _writeableBitmap.Lock();
        try
        {
            IntPtr backBuffer = _writeableBitmap.BackBuffer;
            int stride = _writeableBitmap.BackBufferStride;

            // Clear viewport background to dark gray
            unsafe
            {
                byte* basePtr = (byte*)backBuffer.ToPointer();
                for (int y = 0; y < bmpH; y++)
                {
                    uint* row = (uint*)(basePtr + y * stride);
                    for (int x = 0; x < bmpW; x++)
                    {
                        row[x] = 0xFF252526; // dark gray canvas
                    }
                }
            }

            // Render visible tiles directly into WriteableBitmap unmanaged backbuffer pointer
            foreach (var tile in result.VisibleTiles)
            {
                int destX = Math.Max(0, (int)tile.CanvasX);
                int destY = Math.Max(0, (int)tile.CanvasY);
                int drawW = Math.Min(tile.TileWidth, bmpW - destX);
                int drawH = Math.Min(tile.TileHeight, bmpH - destY);

                if (drawW <= 0 || drawH <= 0) continue;

                unsafe
                {
                    byte* subBuffer = (byte*)backBuffer.ToPointer() + (destY * stride) + (destX * 4);
                    _document.RenderTileIntoBuffer(
                        pageIndex: tile.PageIndex,
                        tileX: tile.TileX,
                        tileY: tile.TileY,
                        tileWidth: drawW,
                        tileHeight: drawH,
                        zoom: _zoom,
                        backBufferPtr: (IntPtr)subBuffer,
                        stride: stride,
                        format: RbdPixelFormat.Bgra32);
                }

                _writeableBitmap.AddDirtyRect(new Int32Rect(destX, destY, drawW, drawH));
            }
        }
        finally
        {
            _writeableBitmap.Unlock();
        }

        sw.Stop();

        // Update diagnostic counters
        int displayPage = result.FirstVisiblePageIndex >= 0 ? result.FirstVisiblePageIndex + 1 : 1;
        UpdatePageDisplay(displayPage);

        var lm = LocalizationManager.Instance;
        StatsTileCountText.Text = lm.GetString("StatsTileCount", result.VisibleTiles.Count);
        StatsRenderTimeText.Text = lm.GetString("StatsRenderTime", sw.Elapsed.TotalMilliseconds);
        StatsDocHeightText.Text = lm.GetString("StatsDocHeightFormat", result.TotalDocumentHeight, TotalPagesDemo);
    }

    private async Task InitializeWebView2Async()
    {
        try
        {
            await OfficeWebView.EnsureCoreWebView2Async();

            // Harden WebView2 runtime per ADR-0003/0004 offline security requirements
            OfficeWebView.CoreWebView2.Settings.IsGeneralAutofillEnabled = false;
            OfficeWebView.CoreWebView2.Settings.IsPasswordAutosaveEnabled = false;
            OfficeWebView.CoreWebView2.Settings.AreHostObjectsAllowed = false;

            string assetsFolder = Path.Combine(AppContext.BaseDirectory, "Assets", "OfficeHost");
            if (!Directory.Exists(assetsFolder))
            {
                assetsFolder = AppContext.BaseDirectory;
            }

            // Map https://rbd-office.local/ strictly to local assets directory (ADR-0004)
            OfficeWebView.CoreWebView2.SetVirtualHostNameToFolderMapping(
                OfflineSecurityPolicy.AllowedHost,
                assetsFolder,
                CoreWebView2HostResourceAccessKind.Allow);

            // 1. Navigation filter: intercepts and cancels unauthorized top-level navigations
            OfficeWebView.CoreWebView2.NavigationStarting += (_, args) =>
            {
                if (!OfflineSecurityPolicy.IsRequestAllowed(args.Uri, out string reason))
                {
                    args.Cancel = true;
                    HandleBlockedRequest(args.Uri, reason);
                }
            };

            // 2. Subresource filter: intercepts fetch(), XHR, scripts, styles, images, and fonts
            OfficeWebView.CoreWebView2.AddWebResourceRequestedFilter("*", CoreWebView2WebResourceContext.All);
            OfficeWebView.CoreWebView2.WebResourceRequested += (_, args) =>
            {
                if (!OfflineSecurityPolicy.IsRequestAllowed(args.Request.Uri, out string reason))
                {
                    // Block unauthorized subresource with 403 Forbidden
                    var forbiddenResponse = OfficeWebView.CoreWebView2.Environment.CreateWebResourceResponse(
                        null, 403, "Forbidden", "Content-Type: text/plain; charset=utf-8");
                    args.Response = forbiddenResponse;
                    HandleBlockedRequest(args.Request.Uri, reason);
                }
            };

            // Navigate to offline virtual-host
            OfficeWebView.CoreWebView2.Navigate("https://rbd-office.local/index.html");
        }
        catch (Exception ex)
        {
            StatusMessageText.Text = LocalizationManager.Instance.GetString("WebViewErrorPrefix", ex.Message);
        }
    }

    private void HandleBlockedRequest(string uri, string reason)
    {
        var lm = LocalizationManager.Instance;
        string blockedMsg = lm.GetString("SecurityBlockedExternalUrl", uri);
        StatusMessageText.Text = "⚠️ " + blockedMsg;
        string detailMsg = blockedMsg + "\n\n" + lm.GetString("SecurityReasonPrefix", reason);
        MessageBox.Show(detailMsg, lm["SecurityDialogTitle"], MessageBoxButton.OK, MessageBoxImage.Warning);
    }

    private void OnPdfScrollChanged(object sender, ScrollChangedEventArgs e)
    {
        if (e.VerticalChange != 0 || e.HorizontalChange != 0 || e.ViewportHeightChange != 0 || e.ViewportWidthChange != 0)
        {
            RenderTiledViewport();
        }
    }

    private void OnZoomInClicked(object sender, RoutedEventArgs e)
    {
        if (_zoom < 5.0)
        {
            _zoom = Math.Min(5.0, _zoom + 0.25);
            RenderTiledViewport();
        }
    }

    private void OnZoomOutClicked(object sender, RoutedEventArgs e)
    {
        if (_zoom > 0.25)
        {
            _zoom = Math.Max(0.25, _zoom - 0.25);
            RenderTiledViewport();
        }
    }

    private void OnZoomFitWidthClicked(object sender, RoutedEventArgs e)
    {
        if (PdfScrollViewer != null && PdfScrollViewer.ViewportWidth > 0)
        {
            _zoom = Math.Clamp(PdfScrollViewer.ViewportWidth / (PageWidthPt + PageGapPt * 2), 0.5, 3.0);
            RenderTiledViewport();
        }
    }

    private void OnPrevPageClicked(object sender, RoutedEventArgs e)
    {
        double pageStride = (PageHeightPt + PageGapPt) * _zoom;
        PdfScrollViewer.ScrollToVerticalOffset(Math.Max(0, PdfScrollViewer.VerticalOffset - pageStride));
    }

    private void OnNextPageClicked(object sender, RoutedEventArgs e)
    {
        double pageStride = (PageHeightPt + PageGapPt) * _zoom;
        PdfScrollViewer.ScrollToVerticalOffset(PdfScrollViewer.VerticalOffset + pageStride);
    }

    private void OnOpenFileClicked(object sender, RoutedEventArgs e)
    {
        var lm = LocalizationManager.Instance;
        MessageBox.Show(lm["MenuOpen"] + " — " + lm["Phase2FeatureNotice"], lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Information);
    }

    private void OnRunSmokeTestClicked(object sender, RoutedEventArgs e)
    {
        var lm = LocalizationManager.Instance;
        var report = RbdNativeSmokeRunner.Run();
        string message = report.IsSuccess
            ? lm.GetString("SmokeTestReportFormat", report.Details, report.LatencyMicroseconds, report.RenderedBytesCount)
            : lm.GetString("SmokeTestFailedFormat", report.Details);

        MessageBox.Show(message, lm["SmokeTestDialogTitle"], MessageBoxButton.OK, report.IsSuccess ? MessageBoxImage.Information : MessageBoxImage.Error);
    }

    private void OnTestNavigateClicked(object sender, RoutedEventArgs e)
    {
        string targetUrl = UrlTestTextBox.Text.Trim();
        if (OfficeWebView?.CoreWebView2 != null)
        {
            OfficeWebView.CoreWebView2.Navigate(targetUrl);
        }
        else
        {
            bool allowed = OfflineSecurityPolicy.IsRequestAllowed(targetUrl, out string reason);
            if (!allowed)
            {
                HandleBlockedRequest(targetUrl, reason);
            }
        }
    }

    private void OnLanguageSelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (LanguageComboBox.SelectedItem is ComboBoxItem item && item.Tag is string tag)
        {
            LocalizationManager.Instance.SetCulture(tag);
        }
    }

    protected override void OnClosed(EventArgs e)
    {
        _document?.Dispose();
        _engine?.Dispose();
        base.OnClosed(e);
    }
}
