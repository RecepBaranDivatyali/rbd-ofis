using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Linq;
using System.Windows;
using System.Windows.Controls;
using Microsoft.Win32;
using RbdOfis.Interop.Localization;
using RbdOfis.Interop.Models;
using RbdOfis.Interop.Services;

namespace RbdOfis.Windows;

/// <summary>
/// Document Converter Wizard Dialog (Phase 5 Track 5C).
/// Supports PDF to Images, Images to PDF, and Markdown to PDF.
/// </summary>
public partial class DocumentConverterDialog : Window
{
    private readonly DocumentConverterService _converterService;

    public string? ResultPdfPath { get; private set; }
    public List<string> GeneratedImagePaths { get; private set; } = [];

    public DocumentConverterDialog(string? currentPdfPath = null, DocumentConverterService? converterService = null)
    {
        InitializeComponent();
        _converterService = converterService ?? new DocumentConverterService();

        UpdateLocalizedUi();

        if (!string.IsNullOrWhiteSpace(currentPdfPath) && File.Exists(currentPdfPath))
        {
            InputPdfTextBox.Text = currentPdfPath;
            string? dir = Path.GetDirectoryName(currentPdfPath);
            if (!string.IsNullOrEmpty(dir))
            {
                OutputDirTextBox.Text = Path.Combine(dir, "exported_images");
                ImagesOutputPdfTextBox.Text = Path.Combine(dir, "images_combined.pdf");
                MarkdownOutputPdfTextBox.Text = Path.Combine(dir, "markdown_doc.pdf");
            }
        }
        else
        {
            string desktop = Environment.GetFolderPath(Environment.SpecialFolder.Desktop);
            OutputDirTextBox.Text = Path.Combine(desktop, "rbd_exported_images");
            ImagesOutputPdfTextBox.Text = Path.Combine(desktop, "images_combined.pdf");
            MarkdownOutputPdfTextBox.Text = Path.Combine(desktop, "markdown_doc.pdf");
        }
    }

    private void UpdateLocalizedUi()
    {
        var lm = LocalizationManager.Instance;

        Title = $"{lm["ConverterTitle"]} — {lm["AppTitle"]}";
        DialogTitleText.Text = $"🔄 {lm["ConverterTitle"]}";

        // Tab Headers
        PdfToImagesTabHeader.Header = $"🖼 {lm["TabPdfToImages"]}";
        ImagesToPdfTabHeader.Header = $"📄 {lm["TabImagesToPdf"]}";
        MarkdownToPdfTabHeader.Header = $"📝 {lm["TabMarkdownToPdf"]}";

        // Tab 1: PDF to Images
        PdfToImagesDescText.Text = lm["TabPdfToImages"];
        InputPdfLabel.Text = lm["InputPdfPrompt"];
        BrowsePdfButton.Content = lm["BrowseButton"];
        OutputDirLabel.Text = lm["OutputDirPrompt"];
        BrowseOutputDirButton.Content = lm["BrowseButton"];
        DpiLabel.Text = lm["DpiLabel"];
        FormatLabel.Text = lm["FormatLabel"];
        PageRangeLabel.Text = lm["PageRangeLabel"];
        PageRangeHint.Text = lm["PageRangeHint"];
        ConvertPdfToImagesButton.Content = $"🔄 {lm["ConvertAction"]}";

        // Tab 2: Images to PDF
        ImagesToPdfDescText.Text = lm["TabImagesToPdf"];
        SelectImagesLabel.Text = lm["SelectImages"];
        AddImagesButton.Content = $"➕ {lm["AddImages"]}";
        RemoveImageButton.Content = $"➖ {lm["RemoveImage"]}";
        ClearImagesButton.Content = $"🗑 {lm["ClearImages"]}";
        PageSizeLabel.Text = lm["PageSizeLabel"];
        PageSizeA4Item.Content = lm["PageSizeA4"];
        PageSizeLetterItem.Content = lm["PageSizeLetter"];
        PageSizeFitItem.Content = lm["PageSizeFitImage"];
        MarginPtLabel.Text = lm["MarginPtLabel"];
        ImagesOutputPdfLabel.Text = lm["OutputPdfPrompt"];
        BrowseImagesOutputPdfButton.Content = lm["BrowseButton"];
        ConvertImagesToPdfButton.Content = $"🔄 {lm["ConvertAction"]}";

        // Tab 3: Markdown to PDF
        MarkdownToPdfDescText.Text = lm["TabMarkdownToPdf"];
        MarkdownInputLabel.Text = lm["MarkdownInputLabel"];
        LoadMdFileButton.Content = $"📂 {lm["LoadMdFile"]}";
        MarkdownOutputPdfLabel.Text = lm["OutputPdfPrompt"];
        BrowseMarkdownOutputPdfButton.Content = lm["BrowseButton"];
        ConvertMarkdownToPdfButton.Content = $"🔄 {lm["ConvertAction"]}";

        CloseDialogButton.Content = lm["CloseButton"];
    }

    // --- Tab 1: PDF to Images ---

    private void OnBrowsePdfClicked(object sender, RoutedEventArgs e)
    {
        var dlg = new OpenFileDialog
        {
            Filter = "PDF Files (*.pdf)|*.pdf|All Files (*.*)|*.*",
            Title = LocalizationManager.Instance["SelectPdf"]
        };
        if (dlg.ShowDialog() == true)
        {
            InputPdfTextBox.Text = dlg.FileName;
            string? dir = Path.GetDirectoryName(dlg.FileName);
            if (!string.IsNullOrEmpty(dir))
            {
                OutputDirTextBox.Text = Path.Combine(dir, "exported_images");
            }
        }
    }

    private void OnBrowseOutputDirClicked(object sender, RoutedEventArgs e)
    {
        var dlg = new OpenFolderDialog
        {
            Title = LocalizationManager.Instance["SelectOutputDir"]
        };
        if (dlg.ShowDialog() == true)
        {
            OutputDirTextBox.Text = dlg.FolderName;
        }
    }

    private void OnConvertPdfToImagesClicked(object sender, RoutedEventArgs e)
    {
        var lm = LocalizationManager.Instance;
        string inPdf = InputPdfTextBox.Text.Trim();
        string outDir = OutputDirTextBox.Text.Trim();

        if (string.IsNullOrEmpty(inPdf) || !File.Exists(inPdf))
        {
            MessageBox.Show(lm.GetString("OperationFailed", lm["InputPdfPrompt"]), lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Warning);
            return;
        }
        if (string.IsNullOrEmpty(outDir))
        {
            MessageBox.Show(lm.GetString("OperationFailed", lm["OutputDirPrompt"]), lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Warning);
            return;
        }

        uint dpi = 150;
        if (DpiComboBox.SelectedItem is ComboBoxItem dpiItem && uint.TryParse(dpiItem.Tag?.ToString(), out uint parsedDpi))
        {
            dpi = parsedDpi;
        }

        ImageFormatEnum format = ImageFormatEnum.Png;
        if (FormatComboBox.SelectedItem is ComboBoxItem formatItem && int.TryParse(formatItem.Tag?.ToString(), out int parsedFormat))
        {
            format = (ImageFormatEnum)parsedFormat;
        }

        List<uint>? pageRange = ParsePageRange(PageRangeTextBox.Text);

        try
        {
            var options = new PdfToImageOptionsDto
            {
                Dpi = dpi,
                Format = format,
                PageRange = pageRange
            };

            var paths = _converterService.ConvertPdfToImages(inPdf, outDir, options);
            GeneratedImagePaths = paths;

            PdfToImagesResultBorder.Visibility = Visibility.Visible;
            PdfToImagesResultText.Text = $"{lm["ConversionSuccess"]}\n({paths.Count} files -> {outDir})";

            var promptResult = MessageBox.Show(lm["OpenConvertedFolder"], lm["AppTitle"], MessageBoxButton.YesNo, MessageBoxImage.Information);
            if (promptResult == MessageBoxResult.Yes)
            {
                Process.Start(new ProcessStartInfo
                {
                    FileName = outDir,
                    UseShellExecute = true
                });
            }
        }
        catch (Exception ex)
        {
            PdfToImagesResultBorder.Visibility = Visibility.Visible;
            PdfToImagesResultText.Text = lm.GetString("ConversionFailed", ex.Message);
            MessageBox.Show(lm.GetString("ConversionFailed", ex.Message), lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Error);
        }
    }

    // --- Tab 2: Images to PDF ---

    private void OnAddImagesClicked(object sender, RoutedEventArgs e)
    {
        var dlg = new OpenFileDialog
        {
            Filter = "Images (*.png;*.jpg;*.jpeg;*.bmp;*.webp)|*.png;*.jpg;*.jpeg;*.bmp;*.webp|All Files (*.*)|*.*",
            Multiselect = true,
            Title = LocalizationManager.Instance["SelectImages"]
        };
        if (dlg.ShowDialog() == true)
        {
            foreach (var file in dlg.FileNames)
            {
                if (!ImagesListBox.Items.Contains(file))
                {
                    ImagesListBox.Items.Add(file);
                }
            }
        }
    }

    private void OnRemoveImageClicked(object sender, RoutedEventArgs e)
    {
        if (ImagesListBox.SelectedItem != null)
        {
            ImagesListBox.Items.Remove(ImagesListBox.SelectedItem);
        }
    }

    private void OnClearImagesClicked(object sender, RoutedEventArgs e)
    {
        ImagesListBox.Items.Clear();
    }

    private void OnBrowseImagesOutputPdfClicked(object sender, RoutedEventArgs e)
    {
        var dlg = new SaveFileDialog
        {
            Filter = "PDF Files (*.pdf)|*.pdf|All Files (*.*)|*.*",
            DefaultExt = ".pdf",
            Title = LocalizationManager.Instance["OutputPdfPrompt"]
        };
        if (dlg.ShowDialog() == true)
        {
            ImagesOutputPdfTextBox.Text = dlg.FileName;
        }
    }

    private void OnConvertImagesToPdfClicked(object sender, RoutedEventArgs e)
    {
        var lm = LocalizationManager.Instance;
        var images = ImagesListBox.Items.Cast<string>().ToList();

        if (images.Count == 0)
        {
            MessageBox.Show(lm["NoImagesSelected"], lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Warning);
            return;
        }

        string outPdf = ImagesOutputPdfTextBox.Text.Trim();
        if (string.IsNullOrEmpty(outPdf))
        {
            OnBrowseImagesOutputPdfClicked(sender, e);
            outPdf = ImagesOutputPdfTextBox.Text.Trim();
            if (string.IsNullOrEmpty(outPdf)) return;
        }

        PageSizeEnum pageSize = PageSizeEnum.A4;
        if (PageSizeComboBox.SelectedItem is ComboBoxItem sizeItem && int.TryParse(sizeItem.Tag?.ToString(), out int parsedSize))
        {
            pageSize = (PageSizeEnum)parsedSize;
        }

        float marginPt = 20.0f;
        if (float.TryParse(MarginPtTextBox.Text.Trim(), NumberStyles.Float, CultureInfo.InvariantCulture, out float parsedMargin))
        {
            marginPt = Math.Max(0.0f, parsedMargin);
        }

        try
        {
            var options = new ImageToPdfOptionsDto
            {
                PageSize = pageSize,
                MarginPt = marginPt
            };

            _converterService.ConvertImagesToPdf(images, outPdf, options);
            ResultPdfPath = outPdf;

            ImagesToPdfResultBorder.Visibility = Visibility.Visible;
            ImagesToPdfResultText.Text = $"{lm["ConversionSuccess"]}\n({outPdf})";

            var promptResult = MessageBox.Show(lm["OpenConvertedFile"], lm["AppTitle"], MessageBoxButton.YesNo, MessageBoxImage.Information);
            if (promptResult == MessageBoxResult.Yes)
            {
                DialogResult = true;
                Close();
            }
        }
        catch (Exception ex)
        {
            ImagesToPdfResultBorder.Visibility = Visibility.Visible;
            ImagesToPdfResultText.Text = lm.GetString("ConversionFailed", ex.Message);
            MessageBox.Show(lm.GetString("ConversionFailed", ex.Message), lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Error);
        }
    }

    // --- Tab 3: Markdown to PDF ---

    private void OnLoadMdFileClicked(object sender, RoutedEventArgs e)
    {
        var dlg = new OpenFileDialog
        {
            Filter = "Markdown Files (*.md;*.markdown;*.txt)|*.md;*.markdown;*.txt|All Files (*.*)|*.*",
            Title = LocalizationManager.Instance["SelectMarkdownFile"]
        };
        if (dlg.ShowDialog() == true)
        {
            try
            {
                string content = File.ReadAllText(dlg.FileName);
                MarkdownTextBox.Text = content;
                string? dir = Path.GetDirectoryName(dlg.FileName);
                string fileNameNoExt = Path.GetFileNameWithoutExtension(dlg.FileName);
                if (!string.IsNullOrEmpty(dir))
                {
                    MarkdownOutputPdfTextBox.Text = Path.Combine(dir, $"{fileNameNoExt}.pdf");
                }
            }
            catch (Exception ex)
            {
                MessageBox.Show(ex.Message, LocalizationManager.Instance["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Error);
            }
        }
    }

    private void OnBrowseMarkdownOutputPdfClicked(object sender, RoutedEventArgs e)
    {
        var dlg = new SaveFileDialog
        {
            Filter = "PDF Files (*.pdf)|*.pdf|All Files (*.*)|*.*",
            DefaultExt = ".pdf",
            Title = LocalizationManager.Instance["OutputPdfPrompt"]
        };
        if (dlg.ShowDialog() == true)
        {
            MarkdownOutputPdfTextBox.Text = dlg.FileName;
        }
    }

    private void OnConvertMarkdownToPdfClicked(object sender, RoutedEventArgs e)
    {
        var lm = LocalizationManager.Instance;
        string mdText = MarkdownTextBox.Text;
        if (string.IsNullOrWhiteSpace(mdText))
        {
            MessageBox.Show(lm.GetString("OperationFailed", lm["MarkdownInputLabel"]), lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Warning);
            return;
        }

        string outPdf = MarkdownOutputPdfTextBox.Text.Trim();
        if (string.IsNullOrEmpty(outPdf))
        {
            OnBrowseMarkdownOutputPdfClicked(sender, e);
            outPdf = MarkdownOutputPdfTextBox.Text.Trim();
            if (string.IsNullOrEmpty(outPdf)) return;
        }

        try
        {
            _converterService.ConvertMarkdownToPdf(mdText, outPdf);
            ResultPdfPath = outPdf;

            MarkdownToPdfResultBorder.Visibility = Visibility.Visible;
            MarkdownToPdfResultText.Text = $"{lm["ConversionSuccess"]}\n({outPdf})";

            var promptResult = MessageBox.Show(lm["OpenConvertedFile"], lm["AppTitle"], MessageBoxButton.YesNo, MessageBoxImage.Information);
            if (promptResult == MessageBoxResult.Yes)
            {
                DialogResult = true;
                Close();
            }
        }
        catch (Exception ex)
        {
            MarkdownToPdfResultBorder.Visibility = Visibility.Visible;
            MarkdownToPdfResultText.Text = lm.GetString("ConversionFailed", ex.Message);
            MessageBox.Show(lm.GetString("ConversionFailed", ex.Message), lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Error);
        }
    }

    private void OnCloseClicked(object sender, RoutedEventArgs e)
    {
        Close();
    }

    /// <summary>
    /// Parses human-friendly page numbers (1-based, e.g. "1, 3, 5-8") into 0-based page indices.
    /// Returns null if empty/whitespace.
    /// </summary>
    public static List<uint>? ParsePageRange(string? input) => DocumentConverterService.ParsePageRange(input);
}
