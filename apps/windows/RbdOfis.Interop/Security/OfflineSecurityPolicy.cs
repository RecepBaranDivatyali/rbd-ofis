using System;
using System.Collections.Concurrent;
using System.Collections.Generic;

namespace RbdOfis.Interop.Security;

/// <summary>
/// Event arguments for detected offline security policy violations.
/// </summary>
public sealed class BlockedRequestEventArgs : EventArgs
{
    public string Url { get; }
    public string Reason { get; }
    public DateTime Timestamp { get; }

    public BlockedRequestEventArgs(string url, string reason)
    {
        Url = url;
        Reason = reason;
        Timestamp = DateTime.UtcNow;
    }
}

/// <summary>
/// Enforces the strict offline sandbox security policy for WebView2 (Office virtual host spike).
/// In accordance with AGENTS.md Rule 5 (zero network access / telemetry prohibited)
/// and ADR-0004 (offline virtual-host mapping to https://rbd-office.local/ with no local TCP ports).
/// </summary>
public static class OfflineSecurityPolicy
{
    public const string AllowedHost = "rbd-office.local";
    public const string AllowedScheme = "https";
    public const string AllowedOrigin = "https://rbd-office.local";

    private static readonly ConcurrentQueue<BlockedRequestEventArgs> _violations = new();

    /// <summary>
    /// Fired when an unauthorized network or URI request is intercepted and blocked.
    /// </summary>
    public static event EventHandler<BlockedRequestEventArgs>? SecurityViolationDetected;

    /// <summary>
    /// Gets the audit log of all blocked request violations.
    /// </summary>
    public static IReadOnlyCollection<BlockedRequestEventArgs> Violations => _violations.ToArray();

    /// <summary>
    /// Clears the recorded audit violations.
    /// </summary>
    public static void ClearViolations()
    {
        _violations.Clear();
    }

    /// <summary>
    /// Validates whether a given URI is strictly permitted under the offline security policy.
    /// </summary>
    public static bool IsRequestAllowed(Uri? uri, out string reason)
    {
        if (uri == null)
        {
            reason = "URI is null.";
            RecordViolation("<null>", reason);
            return false;
        }

        // Allow harmless internal about:blank browser initialization
        if (string.Equals(uri.Scheme, "about", StringComparison.OrdinalIgnoreCase) &&
            string.Equals(uri.AbsoluteUri, "about:blank", StringComparison.OrdinalIgnoreCase))
        {
            reason = string.Empty;
            return true;
        }

        // Must be HTTPS scheme
        if (!string.Equals(uri.Scheme, AllowedScheme, StringComparison.OrdinalIgnoreCase))
        {
            reason = $"Scheme '{uri.Scheme}' is prohibited. Only '{AllowedScheme}' virtual-host mapping is permitted.";
            RecordViolation(uri.OriginalString, reason);
            return false;
        }

        // Must strictly target rbd-office.local
        if (!string.Equals(uri.Host, AllowedHost, StringComparison.OrdinalIgnoreCase))
        {
            reason = $"External host '{uri.Host}' is prohibited. Network access is blocked per Rule 5 of AGENTS.md.";
            RecordViolation(uri.OriginalString, reason);
            return false;
        }

        // Must not use custom TCP ports (ADR-0004: 'Yerel TCP portu açılmaz')
        if (!uri.IsDefaultPort && uri.Port != 443)
        {
            reason = $"Custom network port '{uri.Port}' is prohibited per ADR-0004.";
            RecordViolation(uri.OriginalString, reason);
            return false;
        }

        reason = string.Empty;
        return true;
    }

    /// <summary>
    /// Validates a URI string.
    /// </summary>
    public static bool IsRequestAllowed(string? urlString, out string reason)
    {
        if (string.IsNullOrWhiteSpace(urlString))
        {
            reason = "URL string is null or empty.";
            RecordViolation(urlString ?? "<null>", reason);
            return false;
        }

        if (!Uri.TryCreate(urlString, UriKind.Absolute, out Uri? uri))
        {
            reason = $"Malformed URI: '{urlString}'.";
            RecordViolation(urlString, reason);
            return false;
        }

        return IsRequestAllowed(uri, out reason);
    }

    /// <summary>
    /// Validates a URI without out parameter.
    /// </summary>
    public static bool IsRequestAllowed(Uri? uri) => IsRequestAllowed(uri, out _);

    /// <summary>
    /// Validates a URL string without out parameter.
    /// </summary>
    public static bool IsRequestAllowed(string? urlString) => IsRequestAllowed(urlString, out _);

    private static void RecordViolation(string url, string reason)
    {
        var evt = new BlockedRequestEventArgs(url, reason);
        _violations.Enqueue(evt);
        SecurityViolationDetected?.Invoke(null, evt);
    }
}
