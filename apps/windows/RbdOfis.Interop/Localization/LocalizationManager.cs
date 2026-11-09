using System;
using System.Collections.Generic;
using System.Globalization;
using RbdOfis.Interop.Native;

namespace RbdOfis.Interop.Localization;

/// <summary>
/// Singleton localization manager providing bilingual support (Turkish / English).
/// In accordance with AGENTS.md Rule 5: User-visible strings are loaded from translation dictionaries.
/// </summary>
public sealed class LocalizationManager
{
    private static readonly Lazy<LocalizationManager> _lazy = new(() => new LocalizationManager());
    public static LocalizationManager Instance => _lazy.Value;

    private CultureInfo _currentCulture;
    private IReadOnlyDictionary<string, string> _activeCatalog;

    public static readonly CultureInfo TurkishCulture = new("tr-TR");
    public static readonly CultureInfo EnglishCulture = new("en-US");

    public event EventHandler<CultureInfo>? CultureChanged;

    public CultureInfo CurrentCulture => _currentCulture;

    public IReadOnlyList<CultureInfo> SupportedCultures { get; } =
    [
        TurkishCulture,
        EnglishCulture
    ];

    private LocalizationManager()
    {
        // Default to system UI culture if it starts with 'tr', otherwise fallback to Turkish or English
        string currentName = CultureInfo.CurrentUICulture.Name;
        if (currentName.StartsWith("tr", StringComparison.OrdinalIgnoreCase))
        {
            _currentCulture = TurkishCulture;
            _activeCatalog = StringCatalogs.Turkish;
        }
        else
        {
            _currentCulture = EnglishCulture;
            _activeCatalog = StringCatalogs.English;
        }
    }

    /// <summary>
    /// Explicitly sets the active culture (tr-TR or en-US).
    /// </summary>
    public void SetCulture(CultureInfo culture)
    {
        ArgumentNullException.ThrowIfNull(culture);

        if (culture.Name.StartsWith("tr", StringComparison.OrdinalIgnoreCase))
        {
            _currentCulture = TurkishCulture;
            _activeCatalog = StringCatalogs.Turkish;
        }
        else
        {
            _currentCulture = EnglishCulture;
            _activeCatalog = StringCatalogs.English;
        }

        CultureChanged?.Invoke(this, _currentCulture);
    }

    /// <summary>
    /// Sets the active culture by name code (e.g. "tr-TR", "tr", "en-US", "en").
    /// </summary>
    public void SetCulture(string cultureName)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(cultureName);
        SetCulture(new CultureInfo(cultureName));
    }

    /// <summary>
    /// Gets the localized string for the specified key.
    /// Falls back to English catalog if missing in active catalog, or returns the key itself.
    /// </summary>
    public string GetString(string key)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(key);

        if (_activeCatalog.TryGetValue(key, out string? value))
        {
            return value;
        }

        // Fallback to English
        if (StringCatalogs.English.TryGetValue(key, out string? fallback))
        {
            return fallback;
        }

        return key;
    }

    /// <summary>
    /// Formats a localized string with parameters.
    /// </summary>
    public string GetString(string key, params object[] args)
    {
        string pattern = GetString(key);
        try
        {
            return string.Format(_currentCulture, pattern, args);
        }
        catch (FormatException)
        {
            return pattern;
        }
    }

    public string this[string key] => GetString(key);

    /// <summary>
    /// Returns the complete dictionary of keys and values for a given culture.
    /// Used for completeness validation tests.
    /// </summary>
    public static IReadOnlyDictionary<string, string> GetCatalogForCulture(string cultureName)
    {
        if (cultureName.StartsWith("tr", StringComparison.OrdinalIgnoreCase))
        {
            return StringCatalogs.Turkish;
        }
        if (cultureName.StartsWith("en", StringComparison.OrdinalIgnoreCase))
        {
            return StringCatalogs.English;
        }

        throw new ArgumentException($"Unsupported culture: {cultureName}", nameof(cultureName));
    }

    /// <summary>
    /// Gets all unique translation keys defined across catalogs.
    /// </summary>
    public static IReadOnlySet<string> GetAllKnownKeys()
    {
        var keys = new HashSet<string>(StringCatalogs.Turkish.Keys);
        foreach (string k in StringCatalogs.English.Keys)
        {
            keys.Add(k);
        }
        return keys;
    }

    /// <summary>
    /// Maps an exception to a user-friendly localized message.
    /// Specifically translates RbdNativeException status codes into descriptive messages.
    /// </summary>
    public string GetFriendlyErrorMessage(Exception ex)
    {
        ArgumentNullException.ThrowIfNull(ex);

        if (ex is RbdNativeException nativeEx)
        {
            return GetFriendlyErrorMessage(nativeEx.StatusCode);
        }

        return ex.Message;
    }

    /// <summary>
    /// Maps an RbdStatusCode to a user-friendly localized message.
    /// </summary>
    public string GetFriendlyErrorMessage(RbdStatusCode statusCode)
    {
        return statusCode switch
        {
            RbdStatusCode.ErrorPasswordRequired => GetString("ErrorPasswordRequired"),
            RbdStatusCode.ErrorCorruptPdf => GetString("ErrorCorruptPdf"),
            RbdStatusCode.ErrorFileIo => GetString("ErrorFileIo"),
            RbdStatusCode.ErrorInvalidArgument => GetString("ErrorInvalidArgument"),
            RbdStatusCode.ErrorPageOutOfRange => GetString("ErrorPageOutOfRange"),
            RbdStatusCode.ErrorEngineFailure => GetString("ErrorEngineFailure"),
            _ => GetString("ErrorUnknown")
        };
    }
}
