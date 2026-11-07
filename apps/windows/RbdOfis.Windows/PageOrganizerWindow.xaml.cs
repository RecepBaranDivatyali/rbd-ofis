using System;
using System.Collections.Generic;
using System.Linq;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Imaging;
using Microsoft.Win32;
using RbdOfis.Interop.Localization;
using RbdOfis.Interop.Native;
using RbdOfis.Interop.Services;

namespace RbdOfis.Windows;

public partial class PageOrganizerWindow : Window
{
    private sealed class PageCardItem
    {
        public int OriginalIndex { get; set; }
        public int RotationDegrees { get; set; }
        public bool IsSelected { get; set; }
        public RbdOfis.Interop.Models.PdfRect? CropBox { get; set; }
        public Border? CardElement { get; set; }
    }

    private readonly RbdDocument _document;
    private readonly PageOperationService _pageOps;
    private readonly List<PageCardItem> _pages = new();
    private PageCardItem? _selectedItem;

    public string? ResultFilePath { get; private set; }

    public PageOrganizerWindow(RbdDocument document)
    {
        InitializeComponent();
        _document = document ?? throw new ArgumentNullException(nameof(document));
        _pageOps = new PageOperationService();

        UpdateLocalizedStrings();
        InitializePages();
    }

    private void UpdateLocalizedStrings()
    {
        var lm = LocalizationManager.Instance;
        Title = lm["PageOrganizerTitle"] + " — " + lm["AppTitle"];
        RotateCwButton.Content = lm["RotateClockwise"];
        RotateCcwButton.Content = lm["RotateCounterClockwise"];
        DeleteButton.Content = lm["DeletePage"];
        MoveLeftButton.Content = lm["MoveLeft"];
        MoveRightButton.Content = lm["MoveRight"];
        CropButton.Content = lm["CropPage"];
        InsertBlankButton.Content = lm["InsertBlankPage"];
        MergePdfButton.Content = lm["MergePdfTitle"];
        SplitPdfButton.Content = lm["SplitPdfTitle"];
        CloseButton.Content = lm["MenuClose"];
        ApplyButton.Content = lm["MenuSave"];
    }

    private void InitializePages()
    {
        _pages.Clear();
        int count = _document.PageCount; // Preserve all pages without truncation
        for (int i = 0; i < count; i++)
        {
            _pages.Add(new PageCardItem
            {
                OriginalIndex = i,
                RotationDegrees = 0,
                IsSelected = false
            });
        }
        RebuildCardGrid();
    }

    private void RebuildCardGrid()
    {
        PagesWrapPanel.Children.Clear();
        var lm = LocalizationManager.Instance;

        for (int i = 0; i < _pages.Count; i++)
        {
            var item = _pages[i];
            int displayNum = i + 1;

            var card = new Border
            {
                Width = 140,
                Height = 190,
                Margin = new Thickness(10),
                Background = new SolidColorBrush(Color.FromRgb(37, 37, 38)),
                BorderBrush = item == _selectedItem ? new SolidColorBrush(Color.FromRgb(0, 122, 255)) : new SolidColorBrush(Color.FromRgb(62, 62, 66)),
                BorderThickness = item == _selectedItem ? new Thickness(2) : new Thickness(1),
                CornerRadius = new CornerRadius(4),
                Cursor = System.Windows.Input.Cursors.Hand,
                Tag = item
            };

            var stack = new StackPanel { Margin = new Thickness(6) };

            // Page thumbnail placeholder
            var thumbBorder = new Border
            {
                Width = 120,
                Height = 140,
                Background = new SolidColorBrush(Color.FromRgb(240, 240, 240)),
                BorderBrush = new SolidColorBrush(Color.FromRgb(180, 180, 180)),
                BorderThickness = new Thickness(1)
            };

            var thumbText = new TextBlock
            {
                Text = $"{displayNum}",
                FontSize = 24,
                FontWeight = FontWeights.Bold,
                Foreground = new SolidColorBrush(Color.FromRgb(100, 100, 100)),
                HorizontalAlignment = HorizontalAlignment.Center,
                VerticalAlignment = VerticalAlignment.Center
            };
            thumbBorder.Child = thumbText;

            // Apply visual rotation transform
            if (item.RotationDegrees != 0)
            {
                thumbBorder.LayoutTransform = new RotateTransform(item.RotationDegrees);
            }

            var label = new TextBlock
            {
                Text = $"{lm["Page"]} {displayNum} ({item.RotationDegrees}°)",
                FontSize = 12,
                Foreground = new SolidColorBrush(Color.FromRgb(220, 220, 220)),
                HorizontalAlignment = HorizontalAlignment.Center,
                Margin = new Thickness(0, 6, 0, 0)
            };

            stack.Children.Add(thumbBorder);
            stack.Children.Add(label);
            card.Child = stack;

            card.MouseLeftButtonDown += (s, _) =>
            {
                if (s is Border b && b.Tag is PageCardItem clickedItem)
                {
                    _selectedItem = clickedItem;
                    RebuildCardGrid();
                }
            };

            item.CardElement = card;
            PagesWrapPanel.Children.Add(card);
        }

        StatusLabel.Text = $"{lm["PageOf"].Split('{')[0].Trim()}: {_pages.Count}";
    }

    private void OnRotateCwClicked(object sender, RoutedEventArgs e)
    {
        if (_selectedItem != null)
        {
            _selectedItem.RotationDegrees = (_selectedItem.RotationDegrees + 90) % 360;
            RebuildCardGrid();
        }
    }

    private void OnRotateCcwClicked(object sender, RoutedEventArgs e)
    {
        if (_selectedItem != null)
        {
            _selectedItem.RotationDegrees = (_selectedItem.RotationDegrees + 270) % 360;
            RebuildCardGrid();
        }
    }

    private void OnDeletePageClicked(object sender, RoutedEventArgs e)
    {
        if (_selectedItem != null && _pages.Count > 1)
        {
            _pages.Remove(_selectedItem);
            _selectedItem = _pages.FirstOrDefault();
            RebuildCardGrid();
        }
    }

    private void OnMoveLeftClicked(object sender, RoutedEventArgs e)
    {
        if (_selectedItem != null)
        {
            int idx = _pages.IndexOf(_selectedItem);
            if (idx > 0)
            {
                _pages.RemoveAt(idx);
                _pages.Insert(idx - 1, _selectedItem);
                RebuildCardGrid();
            }
        }
    }

    private void OnMoveRightClicked(object sender, RoutedEventArgs e)
    {
        if (_selectedItem != null)
        {
            int idx = _pages.IndexOf(_selectedItem);
            if (idx >= 0 && idx < _pages.Count - 1)
            {
                _pages.RemoveAt(idx);
                _pages.Insert(idx + 1, _selectedItem);
                RebuildCardGrid();
            }
        }
    }

    private void OnCropClicked(object sender, RoutedEventArgs e)
    {
        if (_selectedItem != null)
        {
            // Toggle crop box (insetting 50pt from edges on A4 standard 595x842)
            if (_selectedItem.CropBox != null)
            {
                _selectedItem.CropBox = null;
            }
            else
            {
                _selectedItem.CropBox = new RbdOfis.Interop.Models.PdfRect(50, 50, 545, 792);
            }
            RebuildCardGrid();
        }
    }

    private void OnInsertBlankClicked(object sender, RoutedEventArgs e)
    {
        int insertIdx = _selectedItem != null ? _pages.IndexOf(_selectedItem) + 1 : _pages.Count;
        var blankItem = new PageCardItem
        {
            OriginalIndex = -1,
            RotationDegrees = 0,
            IsSelected = false
        };
        _pages.Insert(insertIdx, blankItem);
        _selectedItem = blankItem;
        RebuildCardGrid();
    }

    private void OnMergePdfClicked(object sender, RoutedEventArgs e)
    {
        var lm = LocalizationManager.Instance;
        var dlg = new OpenFileDialog
        {
            Title = lm["MergePdfTitle"],
            Filter = "PDF Files (*.pdf)|*.pdf",
            Multiselect = true
        };

        if (dlg.ShowDialog() == true && dlg.FileNames.Length >= 2)
        {
            var saveDlg = new SaveFileDialog
            {
                Title = lm["MergeAction"],
                Filter = "PDF Files (*.pdf)|*.pdf",
                FileName = "Merged.pdf"
            };

            if (saveDlg.ShowDialog() == true)
            {
                try
                {
                    _pageOps.Merge(dlg.FileNames, saveDlg.FileName);
                    MessageBox.Show(lm.GetString("OperationSuccess", saveDlg.FileName), lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Information);
                }
                catch (Exception ex)
                {
                    MessageBox.Show(lm.GetString("OperationFailed", ex.Message), lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Error);
                }
            }
        }
    }

    private void OnSplitPdfClicked(object sender, RoutedEventArgs e)
    {
        var lm = LocalizationManager.Instance;
        var saveDlg = new SaveFileDialog
        {
            Title = lm["SplitAction"],
            Filter = "PDF Files (*.pdf)|*.pdf",
            FileName = "Split_Part.pdf"
        };

        if (saveDlg.ShowDialog() == true)
        {
            try
            {
                string input = !string.IsNullOrEmpty(_document.FilePath) ? _document.FilePath : saveDlg.FileName;
                _pageOps.Split(input, 1, Math.Min(1u, (uint)_pages.Count), saveDlg.FileName);
                MessageBox.Show(lm.GetString("OperationSuccess", saveDlg.FileName), lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Information);
            }
            catch (Exception ex)
            {
                MessageBox.Show(lm.GetString("OperationFailed", ex.Message), lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Error);
            }
        }
    }

    private void OnApplyClicked(object sender, RoutedEventArgs e)
    {
        var lm = LocalizationManager.Instance;
        try
        {
            string sourceFile = _document.FilePath;
            if (string.IsNullOrEmpty(sourceFile) || !System.IO.File.Exists(sourceFile))
            {
                var saveDlg = new SaveFileDialog
                {
                    Title = lm["MenuSaveAs"],
                    Filter = "PDF Files (*.pdf)|*.pdf",
                    FileName = "Document_Organized.pdf"
                };
                if (saveDlg.ShowDialog() != true) return;
                sourceFile = saveDlg.FileName;
                _document.SaveAnnotations(sourceFile);
            }

            string workPath = System.IO.Path.Combine(System.IO.Path.GetTempPath(), $"org_{Guid.NewGuid():N}.pdf");
            System.IO.File.Copy(sourceFile, workPath, overwrite: true);

            // 1. Reorder & Delete
            var nonBlank = _pages.Where(p => p.OriginalIndex >= 0).ToList();
            if (nonBlank.Count > 0)
            {
                uint[] newOrder = nonBlank.Select(p => (uint)(p.OriginalIndex + 1)).ToArray();
                string reorderOut = System.IO.Path.Combine(System.IO.Path.GetTempPath(), $"reorder_{Guid.NewGuid():N}.pdf");
                _pageOps.ReorderPages(workPath, newOrder, reorderOut);
                workPath = reorderOut;
            }

            // 2. Rotations (Rotate only relevant pages, not the entire document)
            var pageRotations = new Dictionary<int, int>();
            for (int i = 0; i < nonBlank.Count; i++)
            {
                if (nonBlank[i].RotationDegrees % 360 != 0)
                {
                    pageRotations[i + 1] = nonBlank[i].RotationDegrees;
                }
            }

            if (pageRotations.Count > 0)
            {
                string rotateOut = System.IO.Path.Combine(System.IO.Path.GetTempPath(), $"rotate_{Guid.NewGuid():N}.pdf");
                _pageOps.RotatePages(workPath, pageRotations, nonBlank.Count, rotateOut);
                workPath = rotateOut;
            }

            // 3. Crops
            for (int i = 0; i < _pages.Count; i++)
            {
                if (_pages[i].CropBox != null)
                {
                    string cropOut = System.IO.Path.Combine(System.IO.Path.GetTempPath(), $"crop_{Guid.NewGuid():N}.pdf");
                    _pageOps.CropPage(workPath, (uint)(i + 1), _pages[i].CropBox!, cropOut);
                    workPath = cropOut;
                }
            }

            // 4. Blank page insertions
            for (int i = 0; i < _pages.Count; i++)
            {
                if (_pages[i].OriginalIndex < 0)
                {
                    string blankOut = System.IO.Path.Combine(System.IO.Path.GetTempPath(), $"blank_{Guid.NewGuid():N}.pdf");
                    _pageOps.InsertBlankPage(workPath, (uint)i, 595.0f, 842.0f, blankOut);
                    workPath = blankOut;
                }
            }

            ResultFilePath = workPath;
            MessageBox.Show(lm.GetString("OperationSuccess", lm["PageLayoutUpdated"]), lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Information);
            DialogResult = true;
            Close();
        }
        catch (Exception ex)
        {
            MessageBox.Show(lm.GetString("OperationFailed", ex.Message), lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Error);
        }
    }

    private void OnCloseClicked(object sender, RoutedEventArgs e)
    {
        Close();
    }
}
