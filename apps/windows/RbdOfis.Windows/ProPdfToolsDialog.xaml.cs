using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using System.Windows;
using System.Windows.Controls;
using RbdOfis.Interop.Localization;
using RbdOfis.Interop.Models;
using RbdOfis.Interop.Native;
using RbdOfis.Interop.Services;

namespace RbdOfis.Windows;

public enum ProToolTab
{
    Compress = 0,
    Watermark = 1,
    PageNumbers = 2,
    Bates = 3,
    FormFields = 4
}

/// <summary>
/// Professional PDF Tools Dialog (Phase 4 Track 4C).
/// Covers Document Compression, Watermarks, Dynamic Page Numbering, Bates Stamping, and AcroForm Fields.
/// </summary>
public partial class ProPdfToolsDialog : Window
{
    private readonly string _currentPdfPath;
    private readonly ProPdfService _proService;
    private readonly Dictionary<string, TextBox> _formFieldTextBoxes = new();

    public string? ResultFilePath { get; private set; }
    public bool DocumentModified { get; private set; }

    public ProPdfToolsDialog(string currentPdfPath, ProToolTab initialTab = ProToolTab.Compress, ProPdfService? proService = null)
    {
        InitializeComponent();
        _currentPdfPath = currentPdfPath;
        _proService = proService ?? new ProPdfService();

        UpdateLocalizedUi();
        ProTabControl.SelectedIndex = (int)initialTab;

        Loaded += (_, _) => LoadFormFields();
    }

    private void UpdateLocalizedUi()
    {
        var lm = LocalizationManager.Instance;

        Title = $"{lm["ProTools"]} — {lm["AppTitle"]}";
        DialogTitleText.Text = $"⚡ {lm["ProTools"]}";

        // Tab Headers
        CompressTabHeader.Header = $"⚡ {lm["TabCompress"]}";
        WatermarkTabHeader.Header = $"💧 {lm["TabWatermark"]}";
        PageNumbersTabHeader.Header = $"🔢 {lm["TabPageNumbers"]}";
        BatesTabHeader.Header = $"🏷 {lm["TabBates"]}";
        FormFieldsTabHeader.Header = $"📝 {lm["TabFormFields"]}";

        // Compress Tab
        CompressDescText.Text = lm["CompressTitle"];
        RemoveUnusedCheckBox.Content = lm["RemoveUnusedObjects"];
        RecompressStreamsCheckBox.Content = lm["RecompressStreams"];
        StripMetadataCheckBox.Content = lm["StripMetadata"];
        CompressButton.Content = $"⚡ {lm["Compress"]}";

        // Watermark Tab
        WatermarkDescText.Text = lm["WatermarkTitle"];
        WatermarkTextLabel.Text = lm["WatermarkTextPrompt"];
        WatermarkFontSizeLabel.Text = lm["FontSizeLabel"];
        WatermarkRotationLabel.Text = lm["RotationLabel"];
        WatermarkOpacityLabel.Text = lm["OpacityLabel"];
        WatermarkOnTopCheckBox.Content = lm["OnTopLabel"];
        WatermarkButton.Content = $"💧 {lm["Watermark"]}";

        // Page Numbers Tab
        PageNumbersDescText.Text = lm["PageNumbersTitle"];
        PageNumTemplateLabel.Text = lm["PageNumberPatternPrompt"];
        PageNumPositionLabel.Text = lm["StampPosition"];
        PageNumFontSizeLabel.Text = lm["FontSizeLabel"];
        PageNumMarginLabel.Text = lm["MarginLabel"];
        PageNumbersButton.Content = $"🔢 {lm["PageNumbers"]}";

        // Position Combo Items
        if (PageNumPositionCombo.Items.Count >= 6)
        {
            ((ComboBoxItem)PageNumPositionCombo.Items[0]).Content = lm["PositionTopLeft"];
            ((ComboBoxItem)PageNumPositionCombo.Items[1]).Content = lm["PositionTopCenter"];
            ((ComboBoxItem)PageNumPositionCombo.Items[2]).Content = lm["PositionTopRight"];
            ((ComboBoxItem)PageNumPositionCombo.Items[3]).Content = lm["PositionBottomLeft"];
            ((ComboBoxItem)PageNumPositionCombo.Items[4]).Content = lm["PositionBottomCenter"];
            ((ComboBoxItem)PageNumPositionCombo.Items[5]).Content = lm["PositionBottomRight"];
        }

        // Bates Stamp Tab
        BatesDescText.Text = lm["BatesStampTitle"];
        BatesPrefixLabel.Text = lm["BatesPrefixPrompt"];
        BatesSuffixLabel.Text = lm["BatesSuffixPrompt"];
        BatesStartLabel.Text = lm["BatesStartPrompt"];
        BatesDigitsLabel.Text = lm["BatesDigitsPrompt"];
        BatesPositionLabel.Text = lm["StampPosition"];
        BatesButton.Content = $"🏷 {lm["BatesStamp"]}";

        if (BatesPositionCombo.Items.Count >= 6)
        {
            ((ComboBoxItem)BatesPositionCombo.Items[0]).Content = lm["PositionTopLeft"];
            ((ComboBoxItem)BatesPositionCombo.Items[1]).Content = lm["PositionTopCenter"];
            ((ComboBoxItem)BatesPositionCombo.Items[2]).Content = lm["PositionTopRight"];
            ((ComboBoxItem)BatesPositionCombo.Items[3]).Content = lm["PositionBottomLeft"];
            ((ComboBoxItem)BatesPositionCombo.Items[4]).Content = lm["PositionBottomCenter"];
            ((ComboBoxItem)BatesPositionCombo.Items[5]).Content = lm["PositionBottomRight"];
        }

        // Form Fields Tab
        FormFieldsDescText.Text = lm["FormFieldsTitle"];
        FlattenFormCheckBox.Content = lm["FlattenForm"];
        SaveFormFieldsButton.Content = $"💾 {lm["SaveFormFields"]}";

        CloseDialogButton.Content = lm["CloseButton"];
    }

    private void OnCompressClicked(object sender, RoutedEventArgs e)
    {
        var lm = LocalizationManager.Instance;
        string tempOut = Path.Combine(Path.GetTempPath(), $"rbd_comp_{Guid.NewGuid():N}.pdf");
        try
        {
            var options = new CompressionOptionsDto
            {
                RemoveUnusedObjects = RemoveUnusedCheckBox.IsChecked == true,
                RecompressStreams = RecompressStreamsCheckBox.IsChecked == true,
                StripMetadata = StripMetadataCheckBox.IsChecked == true
            };

            var result = _proService.Compress(_currentPdfPath, tempOut, options);
            ResultFilePath = tempOut;
            DocumentModified = true;

            double origKb = result.OriginalSize / 1024.0;
            double compKb = result.CompressedSize / 1024.0;
            string msg = string.Format(CultureInfo.CurrentCulture, lm["CompressionSavings"], origKb, compKb, result.SavingsPercent);
            CompressResultText.Text = msg;
            CompressResultBorder.Visibility = Visibility.Visible;
        }
        catch (Exception ex)
        {
            if (File.Exists(tempOut))
            {
                try { File.Delete(tempOut); } catch { }
            }
            string msg = ex is RbdNativeException nativeEx ? lm.GetFriendlyErrorMessage(nativeEx) : ex.Message;
            MessageBox.Show(lm.GetString("OperationFailed", msg), lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Error);
        }
    }

    private void OnWatermarkClicked(object sender, RoutedEventArgs e)
    {
        var lm = LocalizationManager.Instance;
        if (string.IsNullOrWhiteSpace(WatermarkTextBox.Text))
        {
            MessageBox.Show(lm["WatermarkTextEmptyWarning"], lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Warning);
            return;
        }

        string tempOut = Path.Combine(Path.GetTempPath(), $"rbd_wm_{Guid.NewGuid():N}.pdf");
        try
        {
            float.TryParse(WatermarkFontSizeBox.Text, NumberStyles.Float, CultureInfo.InvariantCulture, out float fontSize);
            float.TryParse(WatermarkRotationBox.Text, NumberStyles.Float, CultureInfo.InvariantCulture, out float rot);

            string opacityText = (WatermarkOpacityBox.Text ?? "").Trim().Replace(',', '.');
            float.TryParse(opacityText, NumberStyles.Float, CultureInfo.InvariantCulture, out float opacity);

            var options = new WatermarkOptionsDto
            {
                Text = WatermarkTextBox.Text,
                FontSize = fontSize > 0 ? fontSize : 48.0f,
                RotationDegrees = rot,
                Opacity = opacity > 0 ? opacity : 0.25f,
                ColorRgb = [180, 180, 180],
                OnTop = WatermarkOnTopCheckBox.IsChecked == true
            };

            _proService.ApplyWatermark(_currentPdfPath, tempOut, options);
            ResultFilePath = tempOut;
            DocumentModified = true;

            MessageBox.Show(lm["WatermarkApplied"], lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Information);
            DialogResult = true;
            Close();
        }
        catch (Exception ex)
        {
            if (File.Exists(tempOut))
            {
                try { File.Delete(tempOut); } catch { }
            }
            string msg = ex is RbdNativeException nativeEx ? lm.GetFriendlyErrorMessage(nativeEx) : ex.Message;
            MessageBox.Show(lm.GetString("OperationFailed", msg), lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Error);
        }
    }

    private void OnPageNumbersClicked(object sender, RoutedEventArgs e)
    {
        var lm = LocalizationManager.Instance;
        if (string.IsNullOrWhiteSpace(PageNumTemplateBox.Text))
        {
            MessageBox.Show(lm["PageNumberTemplateEmptyWarning"], lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Warning);
            return;
        }

        string tempOut = Path.Combine(Path.GetTempPath(), $"rbd_num_{Guid.NewGuid():N}.pdf");
        try
        {
            float.TryParse(PageNumFontSizeBox.Text, NumberStyles.Float, CultureInfo.InvariantCulture, out float fontSize);
            float.TryParse(PageNumMarginBox.Text, NumberStyles.Float, CultureInfo.InvariantCulture, out float margin);

            int pos = 4;
            if (PageNumPositionCombo.SelectedItem is ComboBoxItem item && int.TryParse(item.Tag?.ToString(), out int parsedPos))
            {
                pos = parsedPos;
            }

            var options = new PageNumberOptionsDto
            {
                Template = PageNumTemplateBox.Text,
                Position = (StampPosition)pos,
                FontSize = fontSize > 0 ? fontSize : 10.0f,
                Margin = margin > 0 ? margin : 36.0f
            };

            _proService.ApplyPageNumbers(_currentPdfPath, tempOut, options);
            ResultFilePath = tempOut;
            DocumentModified = true;

            MessageBox.Show(lm["PageNumbersApplied"], lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Information);
            DialogResult = true;
            Close();
        }
        catch (Exception ex)
        {
            if (File.Exists(tempOut))
            {
                try { File.Delete(tempOut); } catch { }
            }
            string msg = ex is RbdNativeException nativeEx ? lm.GetFriendlyErrorMessage(nativeEx) : ex.Message;
            MessageBox.Show(lm.GetString("OperationFailed", msg), lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Error);
        }
    }

    private void OnBatesClicked(object sender, RoutedEventArgs e)
    {
        var lm = LocalizationManager.Instance;
        string tempOut = Path.Combine(Path.GetTempPath(), $"rbd_bates_{Guid.NewGuid():N}.pdf");
        try
        {
            uint.TryParse(BatesStartBox.Text, out uint start);
            uint.TryParse(BatesDigitsBox.Text, out uint digits);

            int pos = 5;
            if (BatesPositionCombo.SelectedItem is ComboBoxItem item && int.TryParse(item.Tag?.ToString(), out int parsedPos))
            {
                pos = parsedPos;
            }

            var options = new BatesOptionsDto
            {
                Prefix = BatesPrefixBox.Text,
                Suffix = BatesSuffixBox.Text,
                StartNumber = start > 0 ? start : 1,
                PaddingDigits = digits > 0 ? digits : 6,
                Position = (StampPosition)pos
            };

            _proService.ApplyBatesStamp(_currentPdfPath, tempOut, options);
            ResultFilePath = tempOut;
            DocumentModified = true;

            MessageBox.Show(lm["BatesApplied"], lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Information);
            DialogResult = true;
            Close();
        }
        catch (Exception ex)
        {
            if (File.Exists(tempOut))
            {
                try { File.Delete(tempOut); } catch { }
            }
            string msg = ex is RbdNativeException nativeEx ? lm.GetFriendlyErrorMessage(nativeEx) : ex.Message;
            MessageBox.Show(lm.GetString("OperationFailed", msg), lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Error);
        }
    }

    private void LoadFormFields()
    {
        var lm = LocalizationManager.Instance;
        FormFieldsContainer.Children.Clear();
        _formFieldTextBoxes.Clear();

        if (string.IsNullOrWhiteSpace(_currentPdfPath) || !File.Exists(_currentPdfPath))
        {
            return;
        }

        try
        {
            var fields = _proService.ReadFormFields(_currentPdfPath);
            if (fields.Count == 0)
            {
                var emptyLabel = new TextBlock
                {
                    Text = lm["NoFormFieldsFound"],
                    Foreground = System.Windows.Media.Brushes.Gray,
                    Margin = new Thickness(0, 10, 0, 0),
                    FontStyle = FontStyles.Italic
                };
                FormFieldsContainer.Children.Add(emptyLabel);
                SaveFormFieldsButton.IsEnabled = false;
                return;
            }

            SaveFormFieldsButton.IsEnabled = true;
            foreach (var f in fields)
            {
                var card = new Border
                {
                    Background = new System.Windows.Media.SolidColorBrush(System.Windows.Media.Color.FromRgb(37, 37, 38)),
                    BorderBrush = new System.Windows.Media.SolidColorBrush(System.Windows.Media.Color.FromRgb(62, 62, 66)),
                    BorderThickness = new Thickness(1),
                    CornerRadius = new CornerRadius(3),
                    Padding = new Thickness(10, 6, 10, 6),
                    Margin = new Thickness(0, 0, 0, 8)
                };

                var panel = new StackPanel();
                var headerPanel = new DockPanel { LastChildFill = false };

                var nameBlock = new TextBlock
                {
                    Text = $"{f.Name} ({f.FieldType})",
                    FontWeight = FontWeights.SemiBold,
                    Foreground = System.Windows.Media.Brushes.White
                };
                DockPanel.SetDock(nameBlock, Dock.Left);
                headerPanel.Children.Add(nameBlock);

                if (f.ReadOnly)
                {
                    var roBlock = new TextBlock
                    {
                        Text = "[Read-Only]",
                        Foreground = System.Windows.Media.Brushes.Orange,
                        FontSize = 11
                    };
                    DockPanel.SetDock(roBlock, Dock.Right);
                    headerPanel.Children.Add(roBlock);
                }

                panel.Children.Add(headerPanel);

                var valBox = new TextBox
                {
                    Text = f.Value,
                    Background = new System.Windows.Media.SolidColorBrush(System.Windows.Media.Color.FromRgb(30, 30, 30)),
                    Foreground = System.Windows.Media.Brushes.White,
                    BorderBrush = new System.Windows.Media.SolidColorBrush(System.Windows.Media.Color.FromRgb(62, 62, 66)),
                    Padding = new Thickness(6, 4, 6, 4),
                    Margin = new Thickness(0, 4, 0, 0),
                    IsEnabled = !f.ReadOnly
                };

                _formFieldTextBoxes[f.Name] = valBox;
                panel.Children.Add(valBox);
                card.Child = panel;

                FormFieldsContainer.Children.Add(card);
            }
        }
        catch (Exception ex)
        {
            var errBlock = new TextBlock
            {
                Text = lm.GetString("OperationFailed", ex.Message),
                Foreground = System.Windows.Media.Brushes.Tomato
            };
            FormFieldsContainer.Children.Add(errBlock);
        }
    }

    private void OnSaveFormFieldsClicked(object sender, RoutedEventArgs e)
    {
        var lm = LocalizationManager.Instance;
        string tempOut = Path.Combine(Path.GetTempPath(), $"rbd_form_{Guid.NewGuid():N}.pdf");
        try
        {
            var values = new Dictionary<string, string>();
            foreach (var (fieldName, textBox) in _formFieldTextBoxes)
            {
                values[fieldName] = textBox.Text;
            }

            bool flatten = FlattenFormCheckBox.IsChecked == true;
            _proService.FillFormFields(_currentPdfPath, tempOut, values, flatten);
            ResultFilePath = tempOut;
            DocumentModified = true;

            MessageBox.Show(lm["FormFieldsSaved"], lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Information);
            DialogResult = true;
            Close();
        }
        catch (Exception ex)
        {
            if (File.Exists(tempOut))
            {
                try { File.Delete(tempOut); } catch { }
            }
            string msg = ex is RbdNativeException nativeEx ? lm.GetFriendlyErrorMessage(nativeEx) : ex.Message;
            MessageBox.Show(lm.GetString("OperationFailed", msg), lm["AppTitle"], MessageBoxButton.OK, MessageBoxImage.Error);
        }
    }

    private void OnCloseDialogClicked(object sender, RoutedEventArgs e)
    {
        Close();
    }
}
