using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Threading.Tasks;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using System.Windows.Shapes;
using Microsoft.Web.WebView2.Core;
using Microsoft.Win32;
using RbdOfis.Interop.Localization;
using RbdOfis.Interop.Models;
using RbdOfis.Interop.Native;
using RbdOfis.Interop.Security;
using RbdOfis.Interop.Services;
using RbdOfis.Interop.Viewport;

namespace RbdOfis.Windows;

public partial class MainWindow : Window
{
    private sealed class DocumentTabItem
    {
        public string Title { get; set; } = string.Empty;
        public RbdDocument Document { get; set; } = null!;
        public double Zoom { get; set; } = 1.0;
        public double ScrollX { get; set; }
        public double ScrollY { get; set; }
        public List<SearchHitDto> SearchHits { get; set; } = new();
        public int CurrentSearchIndex { get; set; } = -1;
        public Button TabButton { get; set; } = null!;
        public Border ContainerBorder { get; set; } = null!;
    }

    private enum AnnotationTool
    {
        Select,
        Highlight,
        Underline,
        StrikeOut,
        StickyNote,
        Rectangle,
        FreehandInk,
        FreeText,
        Redact
    }

    private readonly List<(uint PageIndex, PdfRect Rect)> _pendingRedactions = new();
    private readonly ProPdfService _proService = new();
    private readonly DocumentConverterService _converterService = new();
    private readonly OfficeDocumentService _officeService = new();
    private string? _currentOfficeFilePath;

    private enum ViewMode
    {
        Continuous,
        Single,
        TwoPage
    }

    private const double PageWidthPt = 595.0;  // Standard A4 width
    private const double PageHeightPt = 842.0; // Standard A4 height
    private const double PageGapPt = 15.0;

    private readonly RbdEngine _engine;
    private readonly List<DocumentTabItem> _tabs = new();
    private DocumentTabItem? _activeTab;

    private AnnotationTool _currentTool = AnnotationTool.Select;
    private ViewMode _currentViewMode = ViewMode.Continuous;
    private bool _isDarkTheme = true;
    private ulong _annotationIdCounter = 1;
    private readonly Stopwatch _coldOpenStopwatch = Stopwatch.StartNew();

    // Mouse drawing state for annotations
    private bool _isDrawing;
    private Point _drawStartPoint;
    private readonly List<(float X, float Y)> _currentInkPoints = new();
    private FrameworkElement? _tempPreviewElement;

    private WriteableBitmap? _writeableBitmap;
    private readonly string? _initialPdfPath;

    public MainWindow() : this(null)
    {
    }

    public MainWindow(string? initialPdfPath)
    {
        _initialPdfPath = initialPdfPath;
        InitializeComponent();

        try
        {
            _engine = new RbdEngine(NativeRbdApi.Instance);
        }
        catch (Exception ex)
        {
            System.Diagnostics.Debug.WriteLine($"[RBD OFİS] Native engine init failed ({ex.Message}), falling back to Mock.");
            _engine = new RbdEngine(new MockNativeRbdApi
            {
                ConfiguredPageCount = 2000,
                ConfiguredPageWidth = PageWidthPt,
                ConfiguredPageHeight = PageHeightPt
            });
        }

        LocalizationManager.Instance.CultureChanged += (_, _) => UpdateLocalizedUi();
        PreviewKeyDown += OnWindowPreviewKeyDown;
        Loaded += OnWindowLoaded;
    }

    private void OnWindowPreviewKeyDown(object sender, KeyEventArgs e)
    {
        if (Keyboard.Modifiers == ModifierKeys.Control)
        {
            switch (e.Key)
            {
                case Key.O:
                    e.Handled = true;
                    OnOpenFileClicked(this, new RoutedEventArgs());
                    break;
                case Key.S:
                    e.Handled = true;
                    OnSaveFileClicked(this, new RoutedEventArgs());
                    break;
                case Key.Z:
                    e.Handled = true;
                    OnUndoClicked(this, new RoutedEventArgs());
                    break;
                case Key.Y:
                    e.Handled = true;
                    OnRedoClicked(this, new RoutedEventArgs());
                    break;
                case Key.F:
                    e.Handled = true;
                    OnToggleSearchClicked(this, new RoutedEventArgs());
                    break;
            }
        }
    }

    private static string? FindCorpusSamplePdf()
    {
        string baseDir = AppContext.BaseDirectory;
        string? current = baseDir;
        for (int i = 0; i < 8 && current != null; i++)
        {
            string candidate = System.IO.Path.Combine(current, "corpus", "files", "pdf-basic", "pdfium-hello-world.pdf");
            if (File.Exists(candidate)) return System.IO.Path.GetFullPath(candidate);
            current = Directory.GetParent(current)?.FullName;
        }
        return null;
    }

    private async void OnWindowLoaded(object sender, RoutedEventArgs e)
    {
        UpdateLocalizedUi();

        // Open initial default demo document tab
        string? samplePdf = _initialPdfPath;
        if (string.IsNullOrWhiteSpace(samplePdf) || !File.Exists(samplePdf))
        {
            samplePdf = FindCorpusSamplePdf();
        }

        RbdDocument defaultDoc;
        string defaultTitle;
        if (samplePdf != null && File.Exists(samplePdf))
        {
            try
            {
                defaultDoc = _engine.OpenDocument(samplePdf);
                defaultDoc.FilePath = samplePdf;
                defaultTitle = System.IO.Path.GetFileName(samplePdf);
            }
            catch
            {
                defaultDoc = _engine.OpenDocument(new byte[] { 0x25, 0x50, 0x44, 0x46 });
                defaultDoc.FilePath = "Demo_Document_2000p.pdf";
                defaultTitle = "Demo_Document_2000p.pdf";
            }
        }
        else
        {
            defaultDoc = _engine.OpenDocument(new byte[] { 0x25, 0x50, 0x44, 0x46 });
            defaultDoc.FilePath = "Demo_Document_2000p.pdf";
            defaultTitle = "Demo_Document_2000p.pdf";
        }
        OpenNewDocumentTab(defaultDoc, defaultTitle);

        _coldOpenStopwatch.Stop();
        UpdateDiagnostics(0.0);

        await InitializeWebView2Async();
    }

    private void UpdateLocalizedUi()
    {
        var lm = LocalizationManager.Instance;
        Title = $"{lm["AppTitle"]} — {lm["AppSubtitle"]}";
        AppTitleText.Text = lm["AppTitle"];
        AppSubtitleText.Text = lm["AppSubtitle"];
        LanguageLabelText.Text = lm["LanguageLabel"];
        ModePdfRadio.Content = lm["TabPdfViewer"];
        ModeOfficeRadio.Content = lm["TabOfficeEditor"];
        OpenFileButton.Content = $"📁 {lm["MenuOpen"]}";
        SaveFileButton.Content = $"💾 {lm["MenuSave"]}";
        ToggleThumbnailsButton.Content = $"🖼 {lm["ToggleThumbnails"]}";
        PrevPageButton.Content = lm["PrevPage"];
        NextPageButton.Content = lm["NextPageButtonLabel"];
        FitWidthButton.Content = lm["ZoomFitWidth"];
        FitPageButton.Content = lm["FitPage"];
        ToggleSearchButton.Content = $"🔍 {lm["SearchTitle"].Split('(')[0].Trim()}";
        PageOrganizerButton.Content = $"📑 {lm["PageOrganizerTitle"]}";
        ProToolsButton.Content = $"⚡ {lm["ProTools"]}";
        ConverterButton.Content = $"🔄 {lm["Converter"]}";
        SidebarTitleText.Text = lm["ThumbnailsTitle"];
        ThemeToggleButton.Content = _isDarkTheme ? $"🌙 {lm["ThemeDark"]}" : $"☀️ {lm["ThemeLight"]}";
        AssociatePdfButton.Content = $"🔗 {lm["AssociatePdf"]}";

        // Annotation Ribbon
        ToolSelectRadio.Content = $"✋ {lm["ToolSelect"]}";
        ToolHighlightRadio.Content = $"🖊 {lm["ToolHighlight"]}";
        ToolUnderlineRadio.Content = $"U {lm["ToolUnderline"]}";
        ToolStrikeOutRadio.Content = $"S {lm["ToolStrikeOut"]}";
        ToolStickyNoteRadio.Content = $"💬 {lm["ToolStickyNote"]}";
        ToolRectangleRadio.Content = $"▢ {lm["ToolRectangle"]}";
        ToolInkRadio.Content = $"✏ {lm["ToolInk"]}";
        ToolTextRadio.Content = $"T {lm["ToolText"]}";
        ToolRedactRadio.Content = $"⬛ {lm["Redact"]}";
        UndoButton.Content = $"↶ {lm["Undo"].Split('(')[0].Trim()}";
        RedoButton.Content = $"↷ {lm["Redo"].Split('(')[0].Trim()}";
        SaveAnnotationsButton.Content = lm["SaveAnnotations"];
        ApplyRedactionButton.Content = lm["ApplyRedaction"];

        // Status
        StatusMessageText.Text = $"{lm["StatusReady"]} • {lm["StatusOfflineSecure"]}";
        SecuritySandboxText.Text = $"🔒 {lm["SecuritySandboxActive"]} • {OfflineSecurityPolicy.AllowedOrigin}";

        UpdatePageIndicator();
    }

    private void OpenNewDocumentTab(RbdDocument doc, string title)
    {
        var tab = new DocumentTabItem
        {
            Title = title,
            Document = doc,
            Zoom = 1.0
        };

        var container = new Border
        {
            Margin = new Thickness(2, 0, 2, 0),
            Background = new SolidColorBrush(Color.FromRgb(45, 45, 48)),
            BorderBrush = new SolidColorBrush(Color.FromRgb(62, 62, 66)),
            BorderThickness = new Thickness(1),
            CornerRadius = new CornerRadius(3, 3, 0, 0),
            Padding = new Thickness(6, 2, 4, 2)
        };

        var stack = new StackPanel { Orientation = Orientation.Horizontal };
        var tabBtn = new Button
        {
            Content = title,
            Background = Brushes.Transparent,
            Foreground = Brushes.White,
            BorderThickness = new Thickness(0),
            FontWeight = FontWeights.Medium,
            Cursor = Cursors.Hand
        };
        tabBtn.Click += (_, _) => SwitchToTab(tab);

        var closeBtn = new Button
        {
            Content = "✕",
            Width = 18,
            Height = 18,
            Margin = new Thickness(6, 0, 0, 0),
            Background = Brushes.Transparent,
            Foreground = new SolidColorBrush(Color.FromRgb(180, 180, 180)),
            BorderThickness = new Thickness(0),
            Cursor = Cursors.Hand
        };
        closeBtn.Click += (_, _) => CloseDocumentTab(tab);

        stack.Children.Add(tabBtn);
        stack.Children.Add(closeBtn);
        container.Child = stack;

        tab.ContainerBorder = container;
        tab.TabButton = tabBtn;

        _tabs.Add(tab);
        DocumentTabsContainer.Children.Add(container);

        SwitchToTab(tab);
    }

    private void SwitchToTab(DocumentTabItem tab)
    {
        _activeTab = tab;

        // Highlight active tab header visually
        foreach (var t in _tabs)
        {
            if (t == tab)
            {
                t.ContainerBorder.Background = new SolidColorBrush(Color.FromRgb(30, 30, 30));
                t.ContainerBorder.BorderBrush = new SolidColorBrush(Color.FromRgb(0, 122, 255));
                t.TabButton.Foreground = Brushes.White;
            }
            else
            {
                t.ContainerBorder.Background = new SolidColorBrush(Color.FromRgb(45, 45, 48));
                t.ContainerBorder.BorderBrush = new SolidColorBrush(Color.FromRgb(62, 62, 66));
                t.TabButton.Foreground = new SolidColorBrush(Color.FromRgb(180, 180, 180));
            }
        }

        ZoomSlider.Value = tab.Zoom * 100.0;
        ZoomValueText.Text = $"{(int)(tab.Zoom * 100)}%";

        RebuildThumbnails();
        RenderTiledViewport();
    }

    private void CloseDocumentTab(DocumentTabItem tab)
    {
        if (_tabs.Count <= 1) return; // Keep at least one tab open

        _tabs.Remove(tab);
        DocumentTabsContainer.Children.Remove(tab.ContainerBorder);
        tab.Document.Dispose();

        if (_activeTab == tab)
        {
            SwitchToTab(_tabs.Last());
        }
    }

    private void RebuildThumbnails()
    {
        ThumbnailsContainer.Children.Clear();
        if (_activeTab == null) return;

        int pageCount = Math.Min(_activeTab.Document.PageCount, 40); // Generate first 40 thumbnails for speed
        var lm = LocalizationManager.Instance;

        for (int p = 0; p < pageCount; p++)
        {
            int pageIndex = p;
            int pageNum = p + 1;

            var card = new Border
            {
                Width = 140,
                Height = 175,
                Margin = new Thickness(0, 4, 0, 6),
                Background = new SolidColorBrush(Color.FromRgb(37, 37, 38)),
                BorderBrush = new SolidColorBrush(Color.FromRgb(60, 60, 60)),
                BorderThickness = new Thickness(1),
                CornerRadius = new CornerRadius(3),
                Cursor = Cursors.Hand
            };

            var stack = new StackPanel { Margin = new Thickness(4) };

            var thumbBox = new Border
            {
                Width = 120,
                Height = 140,
                Background = new SolidColorBrush(Color.FromRgb(245, 245, 245)),
                BorderBrush = new SolidColorBrush(Color.FromRgb(200, 200, 200)),
                BorderThickness = new Thickness(1)
            };

            var thumbImage = new Image
            {
                Width = 120,
                Height = 140,
                Stretch = Stretch.Uniform
            };

            var pageText = new TextBlock
            {
                Text = $"{pageNum}",
                FontSize = 22,
                FontWeight = FontWeights.Bold,
                Foreground = new SolidColorBrush(Color.FromRgb(120, 120, 120)),
                HorizontalAlignment = HorizontalAlignment.Center,
                VerticalAlignment = VerticalAlignment.Center
            };

            var thumbGrid = new Grid();
            thumbGrid.Children.Add(pageText);
            thumbGrid.Children.Add(thumbImage);
            thumbBox.Child = thumbGrid;

            var currentDoc = _activeTab.Document;
            _ = Task.Run(() =>
            {
                try
                {
                    const int tw = 120;
                    const int th = 160;
                    const int stride = tw * 4;
                    byte[] buffer = new byte[stride * th];
                    double zoom = tw / PageWidthPt;

                    currentDoc.RenderTileIntoBuffer(
                        pageIndex: pageIndex,
                        tileX: 0,
                        tileY: 0,
                        tileWidth: tw,
                        tileHeight: th,
                        zoom: zoom,
                        destinationBuffer: buffer,
                        stride: stride,
                        format: RbdPixelFormat.Bgra32);

                    Dispatcher.InvokeAsync(() =>
                    {
                        var bmp = new WriteableBitmap(tw, th, 96, 96, PixelFormats.Bgra32, null);
                        bmp.WritePixels(new Int32Rect(0, 0, tw, th), buffer, stride, 0);
                        bmp.Freeze();
                        thumbImage.Source = bmp;
                    });
                }
                catch
                {
                    // Keep text block placeholder on exception
                }
            });

            var label = new TextBlock
            {
                Text = $"{lm["Page"]} {pageNum}",
                FontSize = 11,
                Foreground = new SolidColorBrush(Color.FromRgb(210, 210, 210)),
                HorizontalAlignment = HorizontalAlignment.Center,
                Margin = new Thickness(0, 4, 0, 0)
            };

            stack.Children.Add(thumbBox);
            stack.Children.Add(label);
            card.Child = stack;

            card.MouseLeftButtonDown += (_, _) =>
            {
                ScrollToPage(pageIndex);
            };

            ThumbnailsContainer.Children.Add(card);
        }
    }

    private void ScrollToPage(int pageIndex)
    {
        if (_activeTab == null || PdfScrollViewer == null) return;
        double pageStride = (PageHeightPt + PageGapPt) * _activeTab.Zoom;
        double targetScroll = pageIndex * pageStride;
        PdfScrollViewer.ScrollToVerticalOffset(targetScroll);
    }

    private void RenderTiledViewport()
    {
        if (_activeTab == null || PdfScrollViewer == null) return;

        double viewportW = Math.Max(PdfScrollViewer.ViewportWidth, 800);
        double viewportH = Math.Max(PdfScrollViewer.ViewportHeight, 600);
        double scrollX = PdfScrollViewer.HorizontalOffset;
        double scrollY = PdfScrollViewer.VerticalOffset;
        double zoom = _activeTab.Zoom;

        var query = new ViewportQuery(
            TotalPages: _activeTab.Document.PageCount,
            PageWidthPt: PageWidthPt,
            PageHeightPt: PageHeightPt,
            PageGapPt: PageGapPt,
            Zoom: zoom,
            ViewportWidth: viewportW,
            ViewportHeight: viewportH,
            ScrollOffsetX: scrollX,
            ScrollOffsetY: scrollY,
            TileSize: 512);

        var sw = Stopwatch.StartNew();
        var result = TiledViewportMath.CalculateVisibleTiles(query);

        VirtualDocumentCanvas.Height = Math.Max(result.TotalDocumentHeight, viewportH);
        VirtualDocumentCanvas.Width = Math.Max(result.TotalDocumentWidth, viewportW);
        OverlayCanvas.Width = VirtualDocumentCanvas.Width;
        OverlayCanvas.Height = VirtualDocumentCanvas.Height;

        int bmpW = (int)Math.Ceiling(viewportW);
        int bmpH = (int)Math.Ceiling(viewportH);

        if (_writeableBitmap == null || _writeableBitmap.PixelWidth != bmpW || _writeableBitmap.PixelHeight != bmpH)
        {
            _writeableBitmap = new WriteableBitmap(bmpW, bmpH, 96, 96, PixelFormats.Bgra32, null);
            CanvasImage.Source = _writeableBitmap;
        }

        // Direct2D / WriteableBitmap zero-copy render
        _writeableBitmap.Lock();
        try
        {
            IntPtr backBuffer = _writeableBitmap.BackBuffer;
            int stride = _writeableBitmap.BackBufferStride;

            unsafe
            {
                byte* basePtr = (byte*)backBuffer.ToPointer();
                for (int y = 0; y < bmpH; y++)
                {
                    uint* row = (uint*)(basePtr + y * stride);
                    for (int x = 0; x < bmpW; x++)
                    {
                        row[x] = _isDarkTheme ? 0xFF181818 : 0xFFE0E0E0;
                    }
                }
            }

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
                    _activeTab.Document.RenderTileIntoBuffer(
                        pageIndex: tile.PageIndex,
                        tileX: tile.TileX,
                        tileY: tile.TileY,
                        tileWidth: drawW,
                        tileHeight: drawH,
                        zoom: zoom,
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

        // Render visual annotations and search highlights on overlay canvas
        RenderOverlays(result.FirstVisiblePageIndex, result.LastVisiblePageIndex);

        UpdatePageIndicator(result.FirstVisiblePageIndex);
        UpdateDiagnostics(sw.Elapsed.TotalMilliseconds);
    }

    private void RenderOverlays(int firstPage, int lastPage)
    {
        OverlayCanvas.Children.Clear();
        if (_activeTab == null || firstPage < 0 || lastPage < 0) return;

        double zoom = _activeTab.Zoom;
        double pageStride = (PageHeightPt + PageGapPt) * zoom;

        // 1. Render Document Annotations
        for (int p = firstPage; p <= lastPage; p++)
        {
            double pageTopY = p * pageStride;
            var (_, actualH) = _activeTab.Document.GetPageSize(p);
            double pageHeightPt = actualH > 0 ? actualH : PageHeightPt;
            var annots = _activeTab.Document.GetAnnotations((uint)p);

            foreach (var annot in annots)
            {
                double left = annot.Rect.X0 * zoom;
                double top = pageTopY + (pageHeightPt - annot.Rect.Y1) * zoom;
                double w = annot.Rect.Width * zoom;
                double h = annot.Rect.Height * zoom;

                var rectElem = new System.Windows.Shapes.Rectangle
                {
                    Width = Math.Max(w, 4),
                    Height = Math.Max(h, 4),
                    Fill = new SolidColorBrush(Color.FromArgb(
                        annot.ColorRgba[3],
                        annot.ColorRgba[0],
                        annot.ColorRgba[1],
                        annot.ColorRgba[2])),
                    Stroke = annot.KindName == "Rectangle" ? Brushes.Red : null,
                    StrokeThickness = annot.KindName == "Rectangle" ? 2 : 0
                };

                Canvas.SetLeft(rectElem, left);
                Canvas.SetTop(rectElem, top);
                OverlayCanvas.Children.Add(rectElem);

                // Sticky Note or FreeText label
                if (!string.IsNullOrEmpty(annot.Contents))
                {
                    var textBlock = new TextBlock
                    {
                        Text = annot.Contents,
                        FontSize = 11,
                        Background = new SolidColorBrush(Color.FromArgb(200, 255, 255, 200)),
                        Foreground = Brushes.Black,
                        Padding = new Thickness(3, 1, 3, 1)
                    };
                    Canvas.SetLeft(textBlock, left + 4);
                    Canvas.SetTop(textBlock, top + 4);
                    OverlayCanvas.Children.Add(textBlock);
                }
            }
        }

        // 2. Render Search Highlight Overlays (Yellow & Gold)
        if (_activeTab.SearchHits.Count > 0)
        {
            for (int i = 0; i < _activeTab.SearchHits.Count; i++)
            {
                var hit = _activeTab.SearchHits[i];
                if (hit.Page >= (uint)firstPage && hit.Page <= (uint)lastPage)
                {
                    double pageTopY = hit.Page * pageStride;
                    var (_, actualH) = _activeTab.Document.GetPageSize((int)hit.Page);
                    double pageHeightPt = actualH > 0 ? actualH : PageHeightPt;
                    bool isActiveMatch = (i == _activeTab.CurrentSearchIndex);

                    foreach (var rect in hit.Rects)
                    {
                        double left = rect.X0 * zoom;
                        double top = pageTopY + (pageHeightPt - rect.Y1) * zoom;
                        double w = rect.Width * zoom;
                        double h = rect.Height * zoom;

                        var highlightRect = new System.Windows.Shapes.Rectangle
                        {
                            Width = Math.Max(w, 8),
                            Height = Math.Max(h, 8),
                            Fill = isActiveMatch
                                ? new SolidColorBrush(Color.FromArgb(200, 255, 152, 0))   // Orange/Amber for active
                                : new SolidColorBrush(Color.FromArgb(140, 255, 235, 59)), // Yellow for other matches
                            Stroke = isActiveMatch ? Brushes.DarkOrange : null,
                            StrokeThickness = isActiveMatch ? 1.5 : 0
                        };

                        Canvas.SetLeft(highlightRect, left);
                        Canvas.SetTop(highlightRect, top);
                        OverlayCanvas.Children.Add(highlightRect);
                    }
                }
            }
        }

        // 3. Render Pending Redaction blackout boxes (Phase 4 Track 4C)
        foreach (var (page, r) in _pendingRedactions)
        {
            if (page >= (uint)firstPage && page <= (uint)lastPage)
            {
                double pageTopY = page * pageStride;
                var (_, actualH) = _activeTab.Document.GetPageSize((int)page);
                double pageHeightPt = actualH > 0 ? actualH : PageHeightPt;

                double left = r.X0 * zoom;
                double top = pageTopY + (pageHeightPt - r.Y1) * zoom;
                double w = r.Width * zoom;
                double h = r.Height * zoom;

                var redactVisual = new System.Windows.Shapes.Rectangle
                {
                    Width = Math.Max(w, 4),
                    Height = Math.Max(h, 4),
                    Fill = new SolidColorBrush(Color.FromArgb(200, 20, 20, 20)),
                    Stroke = Brushes.Red,
                    StrokeThickness = 2
                };
                Canvas.SetLeft(redactVisual, left);
                Canvas.SetTop(redactVisual, top);
                OverlayCanvas.Children.Add(redactVisual);
            }
        }
    }

    private void UpdatePageIndicator(int firstPageIndex = -1)
    {
        if (_activeTab == null) return;
        int current = firstPageIndex >= 0 ? firstPageIndex + 1 : 1;
        PageJumpTextBox.Text = current.ToString();
        PageTotalLabel.Text = $"/ {_activeTab.Document.PageCount}";
    }

    private void UpdateDiagnostics(double renderMs)
    {
        long memoryBytes = GC.GetTotalMemory(forceFullCollection: false);
        double memoryMb = memoryBytes / (1024.0 * 1024.0);
        double coldOpenMs = _coldOpenStopwatch.Elapsed.TotalMilliseconds;

        DiagnosticsText.Text = LocalizationManager.Instance.GetString(
            "DiagnosticsFormat", coldOpenMs, renderMs, memoryMb);
    }

    // --- Mouse Canvas Annotation Interactions ---
    private void OnOverlayCanvasMouseDown(object sender, MouseButtonEventArgs e)
    {
        if (_activeTab == null || _currentTool == AnnotationTool.Select) return;

        _isDrawing = true;
        _drawStartPoint = e.GetPosition(OverlayCanvas);
        _currentInkPoints.Clear();
        _currentInkPoints.Add(((float)_drawStartPoint.X, (float)_drawStartPoint.Y));

        double zoom = _activeTab.Zoom;
        double pageStride = (PageHeightPt + PageGapPt) * zoom;
        uint pageIndex = (uint)Math.Clamp((int)(_drawStartPoint.Y / pageStride), 0, _activeTab.Document.PageCount - 1);
        double pageTopY = pageIndex * pageStride;
        var (_, actualH) = _activeTab.Document.GetPageSize((int)pageIndex);
        double pageHeightPt = actualH > 0 ? actualH : PageHeightPt;

        if (_currentTool == AnnotationTool.StickyNote)
        {
            _isDrawing = false;
            var lm = LocalizationManager.Instance;
            string noteText = lm["DefaultStickyNoteText"];
            double relTop = _drawStartPoint.Y - pageTopY;
            double relBottom = relTop + 40;
            float y0 = (float)(pageHeightPt - (relBottom / zoom));
            float y1 = (float)(pageHeightPt - (relTop / zoom));
            float x0 = (float)(_drawStartPoint.X / zoom);
            float x1 = (float)((_drawStartPoint.X + 80) / zoom);
            var rect = new PdfRect(x0, y0, x1, y1);

            var annot = AnnotationDto.CreateStickyNote(++_annotationIdCounter, pageIndex, rect, noteText);
            _activeTab.Document.AddAnnotation(annot);
            RenderTiledViewport();
            return;
        }

        if (_currentTool == AnnotationTool.FreeText)
        {
            _isDrawing = false;
            var lm = LocalizationManager.Instance;
            string text = lm["DefaultFreeText"];
            double relTop = _drawStartPoint.Y - pageTopY;
            double relBottom = relTop + 25;
            float y0 = (float)(pageHeightPt - (relBottom / zoom));
            float y1 = (float)(pageHeightPt - (relTop / zoom));
            float x0 = (float)(_drawStartPoint.X / zoom);
            float x1 = (float)((_drawStartPoint.X + 120) / zoom);
            var rect = new PdfRect(x0, y0, x1, y1);

            var annot = AnnotationDto.CreateFreeText(++_annotationIdCounter, pageIndex, rect, text);
            _activeTab.Document.AddAnnotation(annot);
            RenderTiledViewport();
            return;
        }

        // Live preview element for Rectangle, Highlight, Ink, Redact
        if (_currentTool is AnnotationTool.Rectangle or AnnotationTool.Highlight or AnnotationTool.Underline or AnnotationTool.StrikeOut or AnnotationTool.Redact)
        {
            var previewRect = new System.Windows.Shapes.Rectangle
            {
                Stroke = _currentTool == AnnotationTool.Redact ? Brushes.Red : (_currentTool == AnnotationTool.Rectangle ? Brushes.Red : Brushes.Gold),
                StrokeThickness = _currentTool == AnnotationTool.Redact ? 2.0 : 1.5,
                Fill = _currentTool == AnnotationTool.Redact
                    ? new SolidColorBrush(Color.FromArgb(160, 200, 0, 0))
                    : (_currentTool == AnnotationTool.Highlight ? new SolidColorBrush(Color.FromArgb(80, 255, 235, 59)) : null)
            };
            _tempPreviewElement = previewRect;
            OverlayCanvas.Children.Add(previewRect);
        }
    }

    private void OnOverlayCanvasMouseMove(object sender, MouseEventArgs e)
    {
        if (!_isDrawing || _activeTab == null) return;
        Point current = e.GetPosition(OverlayCanvas);

        if (_currentTool == AnnotationTool.FreehandInk)
        {
            _currentInkPoints.Add(((float)current.X, (float)current.Y));
            // Draw temporary live polyline
            if (_tempPreviewElement is not Polyline polyline)
            {
                polyline = new Polyline
                {
                    Stroke = Brushes.Black,
                    StrokeThickness = 2.0
                };
                _tempPreviewElement = polyline;
                OverlayCanvas.Children.Add(polyline);
            }
            polyline.Points.Add(current);
            return;
        }

        if (_tempPreviewElement is System.Windows.Shapes.Rectangle rect)
        {
            double x = Math.Min(_drawStartPoint.X, current.X);
            double y = Math.Min(_drawStartPoint.Y, current.Y);
            double w = Math.Abs(current.X - _drawStartPoint.X);
            double h = Math.Abs(current.Y - _drawStartPoint.Y);

            Canvas.SetLeft(rect, x);
            Canvas.SetTop(rect, y);
            rect.Width = Math.Max(w, 2);
            rect.Height = Math.Max(h, 2);
        }
    }

    private void OnOverlayCanvasMouseUp(object sender, MouseButtonEventArgs e)
    {
        if (!_isDrawing || _activeTab == null) return;
        _isDrawing = false;

        Point endPoint = e.GetPosition(OverlayCanvas);
        if (_tempPreviewElement != null)
        {
            OverlayCanvas.Children.Remove(_tempPreviewElement);
            _tempPreviewElement = null;
        }

        double zoom = _activeTab.Zoom;
        double pageStride = (PageHeightPt + PageGapPt) * zoom;
        uint pageIndex = (uint)Math.Clamp((int)(_drawStartPoint.Y / pageStride), 0, _activeTab.Document.PageCount - 1);
        double pageTopY = pageIndex * pageStride;
        var (_, actualH) = _activeTab.Document.GetPageSize((int)pageIndex);
        double pageHeightPt = actualH > 0 ? actualH : PageHeightPt;

        double minCanvasY = Math.Min(_drawStartPoint.Y, endPoint.Y);
        double maxCanvasY = Math.Max(_drawStartPoint.Y, endPoint.Y);
        double relTop = minCanvasY - pageTopY;
        double relBottom = maxCanvasY - pageTopY;

        double x0 = Math.Min(_drawStartPoint.X, endPoint.X) / zoom;
        double x1 = Math.Max(_drawStartPoint.X, endPoint.X) / zoom;
        double y0 = pageHeightPt - (relBottom / zoom);
        double y1 = pageHeightPt - (relTop / zoom);

        var box = new PdfRect((float)x0, (float)y0, (float)x1, (float)y1);
        if (_currentTool == AnnotationTool.Redact)
        {
            if (box.Width > 5 && box.Height > 5)
            {
                _pendingRedactions.Add((pageIndex, box));
                RenderTiledViewport();
            }
            return;
        }

        ulong id = ++_annotationIdCounter;

        List<(float X, float Y)> pdfInkPoints = new();
        if (_currentTool == AnnotationTool.FreehandInk && _currentInkPoints.Count > 0)
        {
            foreach (var (cx, cy) in _currentInkPoints)
            {
                float px = (float)(cx / zoom);
                float py = (float)(pageHeightPt - ((cy - pageTopY) / zoom));
                pdfInkPoints.Add((px, py));
            }
        }

        AnnotationDto? newAnnot = _currentTool switch
        {
            AnnotationTool.Highlight => AnnotationDto.CreateHighlight(id, pageIndex, box),
            AnnotationTool.Rectangle => AnnotationDto.CreateRectangle(id, pageIndex, box),
            AnnotationTool.Underline => AnnotationDto.CreateUnderline(id, pageIndex, box),
            AnnotationTool.StrikeOut => AnnotationDto.CreateStrikeOut(id, pageIndex, box),
            AnnotationTool.FreehandInk => AnnotationDto.CreateFreehandInk(id, pageIndex, box, pdfInkPoints),
            _ => null
        };

        if (newAnnot != null)
        {
            _activeTab.Document.AddAnnotation(newAnnot);
            RenderTiledViewport();
        }
    }

    // --- Search UI Methods ---
    private void OnToggleSearchClicked(object sender, RoutedEventArgs e)
    {
        SearchBarBorder.Visibility = SearchBarBorder.Visibility == Visibility.Visible ? Visibility.Collapsed : Visibility.Visible;
        if (SearchBarBorder.Visibility == Visibility.Visible)
        {
            SearchQueryTextBox.Focus();
            SearchQueryTextBox.SelectAll();
        }
    }

    private void OnCloseSearchClicked(object sender, RoutedEventArgs e)
    {
        SearchBarBorder.Visibility = Visibility.Collapsed;
        if (_activeTab != null)
        {
            _activeTab.SearchHits.Clear();
            _activeTab.CurrentSearchIndex = -1;
            RenderTiledViewport();
        }
    }

    private void OnSearchQueryKeyDown(object sender, KeyEventArgs e)
    {
        if (e.Key == Key.Enter)
        {
            ExecuteSearch();
        }
    }

    private void ExecuteSearch()
    {
        if (_activeTab == null) return;
        string query = SearchQueryTextBox.Text.Trim();
        if (string.IsNullOrEmpty(query)) return;

        bool caseSensitive = CaseSensitiveCheckBox.IsChecked == true;
        var hits = _activeTab.Document.Search(query, caseSensitive);
        _activeTab.SearchHits = hits.ToList();
        _activeTab.CurrentSearchIndex = hits.Count > 0 ? 0 : -1;

        UpdateSearchCountDisplay();

        if (_activeTab.SearchHits.Count > 0)
        {
            JumpToCurrentSearchHit();
        }
        else
        {
            RenderTiledViewport();
        }
    }

    private void OnSearchNextClicked(object sender, RoutedEventArgs e)
    {
        if (_activeTab == null || _activeTab.SearchHits.Count == 0) return;
        _activeTab.CurrentSearchIndex = (_activeTab.CurrentSearchIndex + 1) % _activeTab.SearchHits.Count;
        UpdateSearchCountDisplay();
        JumpToCurrentSearchHit();
    }

    private void OnSearchPrevClicked(object sender, RoutedEventArgs e)
    {
        if (_activeTab == null || _activeTab.SearchHits.Count == 0) return;
        _activeTab.CurrentSearchIndex = (_activeTab.CurrentSearchIndex - 1 + _activeTab.SearchHits.Count) % _activeTab.SearchHits.Count;
        UpdateSearchCountDisplay();
        JumpToCurrentSearchHit();
    }

    private void JumpToCurrentSearchHit()
    {
        if (_activeTab == null || _activeTab.CurrentSearchIndex < 0 || _activeTab.CurrentSearchIndex >= _activeTab.SearchHits.Count) return;
        var hit = _activeTab.SearchHits[_activeTab.CurrentSearchIndex];
        ScrollToPage((int)hit.Page);
        RenderTiledViewport();
    }

    private void UpdateSearchCountDisplay()
    {
        if (_activeTab == null || _activeTab.SearchHits.Count == 0)
        {
            SearchCountLabel.Text = "0 / 0";
        }
        else
        {
            SearchCountLabel.Text = $"{_activeTab.CurrentSearchIndex + 1} / {_activeTab.SearchHits.Count}";
        }
    }

    // --- Toolbar & Action Handlers ---
    private async void OnOpenFileClicked(object sender, RoutedEventArgs e)
    {
        var dlg = new OpenFileDialog
        {
            Filter = LocalizationManager.Instance["FilterSupportedFiles"],
            Multiselect = true
        };

        if (dlg.ShowDialog() == true)
        {
            foreach (string file in dlg.FileNames)
            {
                try
                {
                    if (OfficeDocumentService.IsSupportedOfficeFile(file))
                    {
                        await OpenOfficeDocumentAsync(file);
                    }
                    else
                    {
                        var doc = _engine.OpenDocument(file);
                        string name = System.IO.Path.GetFileName(file);
                        OpenNewDocumentTab(doc, name);
                    }
                }
                catch (Exception ex)
                {
                    MessageBox.Show(ex.Message, LocalizationManager.Instance["ErrorOpeningDocument"], MessageBoxButton.OK, MessageBoxImage.Error);
                }
            }
        }
    }

    private void OnSaveFileClicked(object sender, RoutedEventArgs e)
    {
        if (_activeTab == null) return;
        var lm = LocalizationManager.Instance;
        var dlg = new SaveFileDialog
        {
            Filter = "PDF Files (*.pdf)|*.pdf",
            FileName = _activeTab.Title
        };

        if (dlg.ShowDialog() == true)
        {
            try
            {
                string input = !string.IsNullOrEmpty(_activeTab.Document.FilePath) ? _activeTab.Document.FilePath : dlg.FileName;
                _activeTab.Document.SaveAnnotations(input, dlg.FileName);
                MessageBox.Show(lm["AnnotationsSavedNotice"], lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Information);
            }
            catch (Exception ex)
            {
                MessageBox.Show(ex.Message, lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Error);
            }
        }
    }

    private void OnSaveAnnotationsClicked(object sender, RoutedEventArgs e)
    {
        OnSaveFileClicked(sender, e);
    }

    private void OnUndoClicked(object sender, RoutedEventArgs e)
    {
        if (_activeTab != null && _activeTab.Document.Undo())
        {
            RenderTiledViewport();
        }
    }

    private void OnRedoClicked(object sender, RoutedEventArgs e)
    {
        if (_activeTab != null && _activeTab.Document.Redo())
        {
            RenderTiledViewport();
        }
    }

    private void OnToolSelected(object sender, RoutedEventArgs e)
    {
        if (sender == ToolSelectRadio) _currentTool = AnnotationTool.Select;
        else if (sender == ToolHighlightRadio) _currentTool = AnnotationTool.Highlight;
        else if (sender == ToolUnderlineRadio) _currentTool = AnnotationTool.Underline;
        else if (sender == ToolStrikeOutRadio) _currentTool = AnnotationTool.StrikeOut;
        else if (sender == ToolStickyNoteRadio) _currentTool = AnnotationTool.StickyNote;
        else if (sender == ToolRectangleRadio) _currentTool = AnnotationTool.Rectangle;
        else if (sender == ToolInkRadio) _currentTool = AnnotationTool.FreehandInk;
        else if (sender == ToolTextRadio) _currentTool = AnnotationTool.FreeText;
        else if (sender == ToolRedactRadio) _currentTool = AnnotationTool.Redact;

        if (ApplyRedactionButton != null)
        {
            ApplyRedactionButton.Visibility = _currentTool == AnnotationTool.Redact ? Visibility.Visible : Visibility.Collapsed;
        }
    }

    private void OnToggleThumbnailsClicked(object sender, RoutedEventArgs e)
    {
        SidebarColumn.Width = SidebarColumn.Width.Value > 0 ? new GridLength(0) : new GridLength(220);
    }

    private void OnCloseSidebarClicked(object sender, RoutedEventArgs e)
    {
        SidebarColumn.Width = new GridLength(0);
    }

    private void OnPageOrganizerClicked(object sender, RoutedEventArgs e)
    {
        if (_activeTab == null) return;
        var organizer = new PageOrganizerWindow(_activeTab.Document)
        {
            Owner = this
        };
        if (organizer.ShowDialog() == true && !string.IsNullOrEmpty(organizer.ResultFilePath))
        {
            try
            {
                var newDoc = _engine.OpenDocument(organizer.ResultFilePath);
                _activeTab.Document.Dispose();
                _activeTab.Document = newDoc;
                _activeTab.Title = System.IO.Path.GetFileName(organizer.ResultFilePath);
                _activeTab.TabButton.Content = _activeTab.Title;
                RebuildThumbnails();
                RenderTiledViewport();
            }
            catch (Exception ex)
            {
                var lm = LocalizationManager.Instance;
                MessageBox.Show(lm.GetString("OperationFailed", ex.Message), lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Error);
            }
        }
    }

    private void OnProToolsClicked(object sender, RoutedEventArgs e)
    {
        if (_activeTab == null || string.IsNullOrWhiteSpace(_activeTab.Document.FilePath) || !File.Exists(_activeTab.Document.FilePath))
        {
            return;
        }

        var dialog = new ProPdfToolsDialog(_activeTab.Document.FilePath, ProToolTab.Compress, _proService)
        {
            Owner = this
        };

        if (dialog.ShowDialog() == true && dialog.DocumentModified && !string.IsNullOrWhiteSpace(dialog.ResultFilePath))
        {
            ReloadActiveTabDocument(dialog.ResultFilePath);
        }
    }

    private void OnConverterClicked(object sender, RoutedEventArgs e)
    {
        string? currentPdf = (_activeTab != null && !string.IsNullOrWhiteSpace(_activeTab.Document.FilePath) && File.Exists(_activeTab.Document.FilePath))
            ? _activeTab.Document.FilePath
            : null;

        var dialog = new DocumentConverterDialog(currentPdf, _converterService)
        {
            Owner = this
        };

        if (dialog.ShowDialog() == true && !string.IsNullOrWhiteSpace(dialog.ResultPdfPath) && File.Exists(dialog.ResultPdfPath))
        {
            try
            {
                var doc = _engine.OpenDocument(dialog.ResultPdfPath);
                doc.FilePath = dialog.ResultPdfPath;
                OpenNewDocumentTab(doc, System.IO.Path.GetFileName(dialog.ResultPdfPath));
            }
            catch (Exception ex)
            {
                var lm = LocalizationManager.Instance;
                MessageBox.Show(lm.GetString("OperationFailed", ex.Message), lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Error);
            }
        }
    }

    private void OnApplyRedactionClicked(object sender, RoutedEventArgs e)
    {
        var lm = LocalizationManager.Instance;
        if (_pendingRedactions.Count == 0)
        {
            MessageBox.Show(lm["RedactionNoAreas"], lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Information);
            return;
        }

        if (_activeTab == null || string.IsNullOrWhiteSpace(_activeTab.Document.FilePath) || !File.Exists(_activeTab.Document.FilePath))
        {
            return;
        }

        var confirm = MessageBox.Show(lm["RedactPrompt"], lm["Redact"], MessageBoxButton.YesNo, MessageBoxImage.Warning);
        if (confirm != MessageBoxResult.Yes) return;

        try
        {
            var grouped = new Dictionary<uint, List<PdfRect>>();
            foreach (var (p, r) in _pendingRedactions)
            {
                if (!grouped.ContainsKey(p)) grouped[p] = new List<PdfRect>();
                grouped[p].Add(r);
            }

            var redactions = new List<PageRedactionDto>();
            foreach (var (page, rects) in grouped)
            {
                redactions.Add(new PageRedactionDto
                {
                    PageIndex = page,
                    Rects = rects,
                    FillColor = [0, 0, 0]
                });
            }

            string tempOut = System.IO.Path.Combine(System.IO.Path.GetTempPath(), $"rbd_redact_{Guid.NewGuid():N}.pdf");
            _proService.Redact(_activeTab.Document.FilePath, tempOut, redactions);

            ReloadActiveTabDocument(tempOut);
            MessageBox.Show(lm["RedactionSuccess"], lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Information);
        }
        catch (Exception ex)
        {
            MessageBox.Show(lm.GetString("OperationFailed", ex.Message), lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Error);
        }
    }

    private void ReloadActiveTabDocument(string newPdfPath)
    {
        if (_activeTab == null || !File.Exists(newPdfPath)) return;

        var newDoc = _engine.OpenDocument(newPdfPath);
        newDoc.FilePath = newPdfPath;

        _activeTab.Document.Dispose();
        _activeTab.Document = newDoc;
        _activeTab.Title = System.IO.Path.GetFileName(newPdfPath);
        _activeTab.TabButton.Content = _activeTab.Title;

        _pendingRedactions.Clear();
        RebuildThumbnails();
        RenderTiledViewport();
    }

    private void OnAssociatePdfClicked(object sender, RoutedEventArgs e)
    {
        var lm = LocalizationManager.Instance;
        string exePath = Environment.ProcessPath ?? Environment.GetCommandLineArgs()[0];
        bool success = FileAssociationService.RegisterPdfAssociation(exePath);
        if (success)
        {
            MessageBox.Show(lm["PdfAssociatedSuccess"], lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Information);
        }
        else
        {
            MessageBox.Show(lm["PdfAssociatedFailed"], lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Warning);
        }
    }

    private void OnThemeToggleClicked(object sender, RoutedEventArgs e)
    {
        _isDarkTheme = !_isDarkTheme;
        var lm = LocalizationManager.Instance;
        ThemeToggleButton.Content = _isDarkTheme ? $"🌙 {lm["ThemeDark"]}" : $"☀️ {lm["ThemeLight"]}";

        var res = Application.Current.Resources;
        if (_isDarkTheme)
        {
            res["AppBackgroundBrush"] = new SolidColorBrush(Color.FromRgb(30, 30, 30));
            res["AppForegroundBrush"] = new SolidColorBrush(Color.FromRgb(212, 212, 212));
            res["CanvasBackgroundBrush"] = new SolidColorBrush(Color.FromRgb(24, 24, 24));
            res["ControlBackgroundBrush"] = new SolidColorBrush(Color.FromRgb(62, 62, 66));
            res["ToolbarBackgroundBrush"] = new SolidColorBrush(Color.FromRgb(45, 45, 48));
            res["SidebarBackgroundBrush"] = new SolidColorBrush(Color.FromRgb(37, 37, 38));
        }
        else
        {
            res["AppBackgroundBrush"] = new SolidColorBrush(Color.FromRgb(243, 243, 243));
            res["AppForegroundBrush"] = new SolidColorBrush(Color.FromRgb(30, 30, 30));
            res["CanvasBackgroundBrush"] = new SolidColorBrush(Color.FromRgb(200, 200, 200));
            res["ControlBackgroundBrush"] = new SolidColorBrush(Color.FromRgb(225, 225, 225));
            res["ToolbarBackgroundBrush"] = new SolidColorBrush(Color.FromRgb(235, 235, 235));
            res["SidebarBackgroundBrush"] = new SolidColorBrush(Color.FromRgb(230, 230, 230));
        }

        RenderTiledViewport();
    }

    private void OnViewModeSelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (ViewModeComboBox.SelectedItem is ComboBoxItem item && item.Tag is string tag)
        {
            _currentViewMode = tag switch
            {
                "Single" => ViewMode.Single,
                "TwoPage" => ViewMode.TwoPage,
                _ => ViewMode.Continuous
            };
            RenderTiledViewport();
        }
    }

    private void OnNewTabClicked(object sender, RoutedEventArgs e)
    {
        var doc = _engine.OpenDocument(new byte[] { 0x25, 0x50, 0x44, 0x46 });
        int num = _tabs.Count + 1;
        var lm = LocalizationManager.Instance;
        OpenNewDocumentTab(doc, $"{lm["DocumentPrefix"]}_{num}.pdf");
    }

    private void OnPrevPageClicked(object sender, RoutedEventArgs e)
    {
        if (_activeTab == null || PdfScrollViewer == null) return;
        double pageStride = (PageHeightPt + PageGapPt) * _activeTab.Zoom;
        PdfScrollViewer.ScrollToVerticalOffset(Math.Max(0, PdfScrollViewer.VerticalOffset - pageStride));
    }

    private void OnNextPageClicked(object sender, RoutedEventArgs e)
    {
        if (_activeTab == null || PdfScrollViewer == null) return;
        double pageStride = (PageHeightPt + PageGapPt) * _activeTab.Zoom;
        PdfScrollViewer.ScrollToVerticalOffset(PdfScrollViewer.VerticalOffset + pageStride);
    }

    private void OnPageJumpKeyDown(object sender, KeyEventArgs e)
    {
        if (e.Key == Key.Enter && int.TryParse(PageJumpTextBox.Text.Trim(), out int pageNum))
        {
            ScrollToPage(pageNum - 1);
        }
    }

    private void OnZoomInClicked(object sender, RoutedEventArgs e)
    {
        if (_activeTab != null && _activeTab.Zoom < 5.0)
        {
            _activeTab.Zoom = Math.Min(5.0, _activeTab.Zoom + 0.25);
            ZoomSlider.Value = _activeTab.Zoom * 100.0;
            RenderTiledViewport();
        }
    }

    private void OnZoomOutClicked(object sender, RoutedEventArgs e)
    {
        if (_activeTab != null && _activeTab.Zoom > 0.25)
        {
            _activeTab.Zoom = Math.Max(0.25, _activeTab.Zoom - 0.25);
            ZoomSlider.Value = _activeTab.Zoom * 100.0;
            RenderTiledViewport();
        }
    }

    private void OnZoomSliderValueChanged(object sender, RoutedPropertyChangedEventArgs<double> e)
    {
        if (_activeTab != null && Math.Abs(_activeTab.Zoom - (e.NewValue / 100.0)) > 0.01)
        {
            _activeTab.Zoom = e.NewValue / 100.0;
            ZoomValueText.Text = $"{(int)e.NewValue}%";
            RenderTiledViewport();
        }
    }

    private void OnFitWidthClicked(object sender, RoutedEventArgs e)
    {
        if (_activeTab != null && PdfScrollViewer != null && PdfScrollViewer.ViewportWidth > 0)
        {
            _activeTab.Zoom = Math.Clamp(PdfScrollViewer.ViewportWidth / (PageWidthPt + PageGapPt * 2), 0.25, 5.0);
            ZoomSlider.Value = _activeTab.Zoom * 100.0;
            RenderTiledViewport();
        }
    }

    private void OnFitPageClicked(object sender, RoutedEventArgs e)
    {
        if (_activeTab != null && PdfScrollViewer != null && PdfScrollViewer.ViewportHeight > 0)
        {
            _activeTab.Zoom = Math.Clamp(PdfScrollViewer.ViewportHeight / (PageHeightPt + PageGapPt * 2), 0.25, 5.0);
            ZoomSlider.Value = _activeTab.Zoom * 100.0;
            RenderTiledViewport();
        }
    }

    private void OnPdfScrollViewerPreviewMouseWheel(object sender, MouseWheelEventArgs e)
    {
        if (Keyboard.Modifiers == ModifierKeys.Control && _activeTab != null)
        {
            e.Handled = true;
            double delta = e.Delta > 0 ? 0.15 : -0.15;
            _activeTab.Zoom = Math.Clamp(_activeTab.Zoom + delta, 0.25, 5.0);
            ZoomSlider.Value = _activeTab.Zoom * 100.0;
            RenderTiledViewport();
        }
    }

    private void OnPdfScrollChanged(object sender, ScrollChangedEventArgs e)
    {
        if (e.VerticalChange != 0 || e.HorizontalChange != 0 || e.ViewportHeightChange != 0 || e.ViewportWidthChange != 0)
        {
            RenderTiledViewport();
        }
    }

    private void OnLanguageSelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (LanguageComboBox.SelectedItem is ComboBoxItem item && item.Tag is string tag)
        {
            LocalizationManager.Instance.SetCulture(tag);
        }
    }

    private void OnModeRadioChanged(object sender, RoutedEventArgs e)
    {
        if (PdfWorkspaceGrid == null || OfficeWorkspaceGrid == null) return;
        if (ModePdfRadio.IsChecked == true)
        {
            PdfWorkspaceGrid.Visibility = Visibility.Visible;
            OfficeWorkspaceGrid.Visibility = Visibility.Collapsed;
        }
        else
        {
            PdfWorkspaceGrid.Visibility = Visibility.Collapsed;
            OfficeWorkspaceGrid.Visibility = Visibility.Visible;
        }
    }

    private async Task InitializeWebView2Async()
    {
        try
        {
            await OfficeWebView.EnsureCoreWebView2Async();

            OfficeWebView.CoreWebView2.Settings.IsGeneralAutofillEnabled = false;
            OfficeWebView.CoreWebView2.Settings.IsPasswordAutosaveEnabled = false;
            OfficeWebView.CoreWebView2.Settings.AreHostObjectsAllowed = false;

            string assetsFolder = System.IO.Path.Combine(AppContext.BaseDirectory, "Assets", "OfficeHost");
            if (!Directory.Exists(assetsFolder))
            {
                assetsFolder = AppContext.BaseDirectory;
            }

            OfficeWebView.CoreWebView2.SetVirtualHostNameToFolderMapping(
                OfflineSecurityPolicy.AllowedHost,
                assetsFolder,
                CoreWebView2HostResourceAccessKind.Allow);

            OfficeWebView.CoreWebView2.NavigationStarting += (_, args) =>
            {
                if (!OfflineSecurityPolicy.IsRequestAllowed(args.Uri, out string reason))
                {
                    args.Cancel = true;
                    string msg = LocalizationManager.Instance.GetString("SecurityBlockedExternalUrl", args.Uri);
                    MessageBox.Show($"{msg}\n\n{reason}", LocalizationManager.Instance["SecurityDialogTitle"], MessageBoxButton.OK, MessageBoxImage.Warning);
                }
            };

            OfficeWebView.CoreWebView2.AddWebResourceRequestedFilter("*", CoreWebView2WebResourceContext.All);
            OfficeWebView.CoreWebView2.WebResourceRequested += (_, args) =>
            {
                if (!OfflineSecurityPolicy.IsRequestAllowed(args.Request.Uri, out string reason))
                {
                    args.Response = OfficeWebView.CoreWebView2.Environment.CreateWebResourceResponse(null, 403, "Forbidden", "Content-Type: text/plain; charset=utf-8");
                }
            };

            OfficeWebView.CoreWebView2.WebMessageReceived += (_, args) =>
            {
                try
                {
                    string json = args.WebMessageAsJson;
                    using var doc = System.Text.Json.JsonDocument.Parse(json);
                    var root = doc.RootElement;
                    if (root.TryGetProperty("action", out var actionProp))
                    {
                        string action = actionProp.GetString() ?? "";
                        if (action == "saveDocx")
                        {
                            string content = root.TryGetProperty("content", out var cProp) ? cProp.GetString() ?? "" : "";
                            string outPath = _currentOfficeFilePath ?? System.IO.Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.MyDocuments), "Belge.docx");
                            _officeService.CreateDocx(content, outPath);
                            StatusMessageText.Text = LocalizationManager.Instance.GetString("OfficeSaveDocxSuccess", System.IO.Path.GetFileName(outPath));
                        }
                        else if (action == "exportPdf")
                        {
                            string content = root.TryGetProperty("content", out var cProp) ? cProp.GetString() ?? "" : "";
                            string outPath = System.IO.Path.ChangeExtension(_currentOfficeFilePath ?? System.IO.Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.MyDocuments), "Belge.docx"), ".pdf");
                            _converterService.ConvertMarkdownToPdf(content, outPath);
                            StatusMessageText.Text = LocalizationManager.Instance.GetString("OfficeExportPdfSuccess", System.IO.Path.GetFileName(outPath));
                        }
                    }
                }
                catch (Exception ex)
                {
                    StatusMessageText.Text = ex.Message;
                }
            };

            OfficeWebView.CoreWebView2.Navigate("https://rbd-office.local/index.html");
        }
        catch (Exception ex)
        {
            StatusMessageText.Text = LocalizationManager.Instance.GetString("WebViewErrorPrefix", ex.Message);
        }
    }

    /// <summary>
    /// Opens an Office document (DOCX, XLSX, PPTX) in the offline WebView workspace.
    /// </summary>
    public async Task OpenOfficeDocumentAsync(string filePath)
    {
        try
        {
            _currentOfficeFilePath = filePath;
            var info = _officeService.Inspect(filePath);
            string rawText = _officeService.ExtractText(filePath);
            string title = System.IO.Path.GetFileName(filePath);

            await InitializeWebView2Async();
            ModeOfficeRadio.IsChecked = true;

            string titleJson = System.Text.Json.JsonSerializer.Serialize(title);
            string docTypeJson = System.Text.Json.JsonSerializer.Serialize(info.DocType);
            string contentJson = System.Text.Json.JsonSerializer.Serialize(rawText);

            string js = $"window.loadOfficeDocument({titleJson}, {docTypeJson}, {contentJson}, {info.WordCount}, {info.ItemCount});";
            await OfficeWebView.CoreWebView2.ExecuteScriptAsync(js);

            string unitName = info.DocType == "XLSX" ? LocalizationManager.Instance["OfficeRows"] : LocalizationManager.Instance["OfficeParagraphs"];
            string summary = LocalizationManager.Instance.GetString("OfficeInspectSummary", info.DocType, info.WordCount, info.ItemCount, unitName);
            StatusMessageText.Text = LocalizationManager.Instance.GetString("OfficeDocumentOpened", $"{title} ({summary})");
        }
        catch (Exception ex)
        {
            MessageBox.Show(ex.Message, LocalizationManager.Instance["ErrorOpeningDocument"], MessageBoxButton.OK, MessageBoxImage.Error);
        }
    }

    protected override void OnClosed(EventArgs e)
    {
        foreach (var t in _tabs)
        {
            t.Document.Dispose();
        }
        _engine?.Dispose();
        base.OnClosed(e);
    }
}
