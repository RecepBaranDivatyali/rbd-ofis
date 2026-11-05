using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using RbdOfis.Interop.Localization;
using Xunit;

namespace RbdOfis.Windows.Tests;

public class LocalizationTests
{
    [Fact]
    public void Verify_Turkish_And_English_Catalogs_Have_Strict_Key_Parity()
    {
        // Rule 5 of AGENTS.md: TR + EN mandatory, translation dictionaries must have 100% key completeness
        var turkishCatalog = LocalizationManager.GetCatalogForCulture("tr-TR");
        var englishCatalog = LocalizationManager.GetCatalogForCulture("en-US");

        var missingInEnglish = new HashSet<string>(turkishCatalog.Keys);
        missingInEnglish.ExceptWith(englishCatalog.Keys);

        var missingInTurkish = new HashSet<string>(englishCatalog.Keys);
        missingInTurkish.ExceptWith(turkishCatalog.Keys);

        Assert.Empty(missingInEnglish);
        Assert.Empty(missingInTurkish);
        Assert.Equal(turkishCatalog.Count, englishCatalog.Count);
        Assert.True(turkishCatalog.Count >= 45, $"Catalog has only {turkishCatalog.Count} keys; expected at least 45 keys.");
    }

    [Fact]
    public void Verify_No_Empty_Or_Whitespace_Translations()
    {
        var turkish = LocalizationManager.GetCatalogForCulture("tr-TR");
        var english = LocalizationManager.GetCatalogForCulture("en-US");

        foreach (var (key, value) in turkish)
        {
            Assert.False(string.IsNullOrWhiteSpace(value), $"Turkish key '{key}' has empty or whitespace value.");
        }

        foreach (var (key, value) in english)
        {
            Assert.False(string.IsNullOrWhiteSpace(value), $"English key '{key}' has empty or whitespace value.");
        }
    }

    [Fact]
    public void Verify_Hardening_Keys_Are_Present_And_Complete()
    {
        string[] requiredHardeningKeys =
        [
            "LanguageLabel",
            "PrevPage",
            "NextPageButtonLabel",
            "TooltipZoomOut",
            "TooltipZoomIn",
            "OfflineFilterTestLabel",
            "TestNavigate",
            "ButtonSmokeTest",
            "StatsDocHeightFormat",
            "SecurityDialogTitle",
            "SecurityReasonPrefix",
            "WebViewErrorPrefix",
            "Phase2FeatureNotice",
            "SmokeTestReportFormat",
            "SmokeTestFailedFormat",
            "SmokeTestDialogTitle"
        ];

        var turkish = LocalizationManager.GetCatalogForCulture("tr-TR");
        var english = LocalizationManager.GetCatalogForCulture("en-US");

        foreach (string key in requiredHardeningKeys)
        {
            Assert.True(turkish.ContainsKey(key), $"Turkish catalog missing hardening key '{key}'.");
            Assert.True(english.ContainsKey(key), $"English catalog missing hardening key '{key}'.");
        }
    }

    [Fact]
    public void Verify_Phase2_Keys_Are_Present_And_Complete()
    {
        string[] requiredPhase2Keys =
        [
            "ToggleThumbnails",
            "ThumbnailsTitle",
            "SearchTitle",
            "SearchPlaceholder",
            "CaseSensitive",
            "MatchCount",
            "ToolSelect",
            "ToolHighlight",
            "ToolUnderline",
            "ToolStrikeOut",
            "ToolStickyNote",
            "ToolRectangle",
            "ToolInk",
            "ToolText",
            "Undo",
            "Redo",
            "SaveAnnotations",
            "PageOrganizerTitle",
            "MergePdfTitle",
            "SplitPdfTitle",
            "ThemeDark",
            "ThemeLight",
            "DiagnosticsFormat",
            "AssociatePdf",
            "PdfAssociatedSuccess",
            "PdfAssociatedFailed"
        ];

        var turkish = LocalizationManager.GetCatalogForCulture("tr-TR");
        var english = LocalizationManager.GetCatalogForCulture("en-US");

        foreach (string key in requiredPhase2Keys)
        {
            Assert.True(turkish.ContainsKey(key), $"Turkish catalog missing Phase 2 key '{key}'.");
            Assert.True(english.ContainsKey(key), $"English catalog missing Phase 2 key '{key}'.");
        }
    }

    [Fact]
    public void Verify_Phase4_ProTools_Keys_Are_Present_And_Complete()
    {
        string[] requiredProKeys =
        [
            "ProTools",
            "Redact",
            "ApplyRedaction",
            "RedactPrompt",
            "RedactionSuccess",
            "RedactionNoAreas",
            "Compress",
            "CompressTitle",
            "CompressionSavings",
            "RemoveUnusedObjects",
            "RecompressStreams",
            "StripMetadata",
            "Watermark",
            "WatermarkTitle",
            "WatermarkTextPrompt",
            "WatermarkApplied",
            "PageNumbers",
            "PageNumbersTitle",
            "PageNumberPatternPrompt",
            "PageNumbersApplied",
            "BatesStamp",
            "BatesStampTitle",
            "BatesPrefixPrompt",
            "BatesStartPrompt",
            "BatesDigitsPrompt",
            "BatesApplied",
            "FormFields",
            "FormFieldsTitle",
            "NoFormFieldsFound",
            "FlattenForm",
            "SaveFormFields",
            "FormFieldsSaved"
        ];

        var turkish = LocalizationManager.GetCatalogForCulture("tr-TR");
        var english = LocalizationManager.GetCatalogForCulture("en-US");

        foreach (string key in requiredProKeys)
        {
            Assert.True(turkish.ContainsKey(key), $"Turkish catalog missing Phase 4 Pro key '{key}'.");
            Assert.True(english.ContainsKey(key), $"English catalog missing Phase 4 Pro key '{key}'.");
        }
    }

    [Fact]
    public void Verify_Phase5_Converter_Keys_Are_Present_And_Complete()
    {
        string[] requiredConverterKeys =
        [
            "Converter",
            "ConverterTitle",
            "TabPdfToImages",
            "TabImagesToPdf",
            "TabMarkdownToPdf",
            "SelectPdf",
            "SelectOutputDir",
            "DpiLabel",
            "FormatLabel",
            "PageRangeLabel",
            "PageRangeHint",
            "SelectImages",
            "AddImages",
            "RemoveImage",
            "ClearImages",
            "NoImagesSelected",
            "PageSizeLabel",
            "PageSizeA4",
            "PageSizeLetter",
            "PageSizeFitImage",
            "MarginPtLabel",
            "MarkdownInputLabel",
            "LoadMdFile",
            "ConvertAction",
            "ConversionSuccess",
            "ConversionFailed",
            "OpenConvertedFile",
            "OpenConvertedFolder",
            "SelectMarkdownFile",
            "BrowseButton",
            "InputPdfPrompt",
            "OutputDirPrompt",
            "OutputPdfPrompt"
        ];

        var turkish = LocalizationManager.GetCatalogForCulture("tr-TR");
        var english = LocalizationManager.GetCatalogForCulture("en-US");

        foreach (string key in requiredConverterKeys)
        {
            Assert.True(turkish.ContainsKey(key), $"Turkish catalog missing Phase 5 Converter key '{key}'.");
            Assert.True(english.ContainsKey(key), $"English catalog missing Phase 5 Converter key '{key}'.");
        }
    }

    [Fact]
    public void Verify_MainWindow_CodeBehind_Uses_Localization_For_All_User_Dialogs()
    {
        string? projectRoot = null;
        string currentDir = AppContext.BaseDirectory;

        // Search upward for apps/windows/RbdOfis.Windows/MainWindow.xaml.cs
        for (int i = 0; i < 6; i++)
        {
            string candidate = Path.Combine(currentDir, "apps", "windows", "RbdOfis.Windows", "MainWindow.xaml.cs");
            if (File.Exists(candidate))
            {
                projectRoot = candidate;
                break;
            }
            string candidateDirect = Path.Combine(currentDir, "..", "RbdOfis.Windows", "MainWindow.xaml.cs");
            if (File.Exists(candidateDirect))
            {
                projectRoot = Path.GetFullPath(candidateDirect);
                break;
            }
            string? parent = Directory.GetParent(currentDir)?.FullName;
            if (parent == null) break;
            currentDir = parent;
        }

        if (projectRoot != null && File.Exists(projectRoot))
        {
            string sourceCode = File.ReadAllText(projectRoot);

            // Verify none of the previously rejected hardcoded Turkish strings exist in code-behind
            Assert.DoesNotContain("\"RBD OFİS — Güvenlik Engeli\"", sourceCode);
            Assert.DoesNotContain("\"RBD OFİS — FFI Sıfır-Kopya Smoke Testi\"", sourceCode);
            Assert.DoesNotContain("\"Faz 2'de yerel dosya seçici ile açılacak.\"", sourceCode);
            Assert.DoesNotContain("\"Neden: \"", sourceCode);
            Assert.DoesNotContain("\"WebView2 Offline Host Hatası: \"", sourceCode);
            Assert.DoesNotContain("\"RBD OFİS Notu\"", sourceCode);
            Assert.DoesNotContain("\"Örnek Açıklama Metni\"", sourceCode);
            Assert.DoesNotContain("\"🌙 Koyu Tema\"", sourceCode);
            Assert.DoesNotContain("\"☀️ Açık Tema\"", sourceCode);
            Assert.DoesNotContain("\"Belge_\"", sourceCode);
        }

        string? organizerPath = null;
        currentDir = AppContext.BaseDirectory;
        for (int i = 0; i < 6; i++)
        {
            string candidate = Path.Combine(currentDir, "apps", "windows", "RbdOfis.Windows", "PageOrganizerWindow.xaml.cs");
            if (File.Exists(candidate))
            {
                organizerPath = candidate;
                break;
            }
            string candidateDirect = Path.Combine(currentDir, "..", "RbdOfis.Windows", "PageOrganizerWindow.xaml.cs");
            if (File.Exists(candidateDirect))
            {
                organizerPath = Path.GetFullPath(candidateDirect);
                break;
            }
            string? parent = Directory.GetParent(currentDir)?.FullName;
            if (parent == null) break;
            currentDir = parent;
        }

        if (organizerPath != null && File.Exists(organizerPath))
        {
            string orgCode = File.ReadAllText(organizerPath);
            Assert.DoesNotContain("\"Sayfa düzeni güncellendi.\"", orgCode);
        }
    }

    [Fact]
    public void Verify_Culture_Switching_Updates_Strings_And_Fires_Event()
    {
        var manager = LocalizationManager.Instance;
        bool eventFired = false;
        CultureInfo? observedCulture = null;

        EventHandler<CultureInfo> handler = (_, culture) =>
        {
            eventFired = true;
            observedCulture = culture;
        };

        manager.CultureChanged += handler;
        try
        {
            // Switch to Turkish
            manager.SetCulture("tr-TR");
            Assert.True(eventFired);
            Assert.Equal("tr-TR", observedCulture?.Name);
            Assert.Equal("PDF Görüntüleyici", manager["TabPdfViewer"]);
            Assert.Equal("Dosya", manager["MenuFile"]);
            Assert.Equal("Adrese Git / Test Et", manager["TestNavigate"]);
            Assert.Equal("FFI Sıfır-Kopya Testi", manager["ButtonSmokeTest"]);

            // Switch to English
            eventFired = false;
            manager.SetCulture("en-US");
            Assert.True(eventFired);
            Assert.Equal("en-US", observedCulture?.Name);
            Assert.Equal("PDF Viewer", manager["TabPdfViewer"]);
            Assert.Equal("File", manager["MenuFile"]);
            Assert.Equal("Navigate / Test", manager["TestNavigate"]);
            Assert.Equal("FFI Zero-Copy Test", manager["ButtonSmokeTest"]);
        }
        finally
        {
            manager.CultureChanged -= handler;
        }
    }

    [Fact]
    public void Verify_Missing_Key_Returns_Key_Name_Without_Exception()
    {
        var manager = LocalizationManager.Instance;
        const string nonExistentKey = "NonExistent_Test_Key_12345";
        string result = manager.GetString(nonExistentKey);
        Assert.Equal(nonExistentKey, result);
    }
}
