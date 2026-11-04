using System;
using System.Collections.Generic;
using RbdOfis.Interop.Security;
using Xunit;

namespace RbdOfis.Windows.Tests;

public class OfflineSecurityPolicyTests
{
    [Theory]
    [InlineData("https://rbd-office.local/")]
    [InlineData("https://rbd-office.local/index.html")]
    [InlineData("https://rbd-office.local/sdkjs/word/sdk-all.js")]
    [InlineData("https://rbd-office.local/web-apps/apps/documenteditor/main/index.html")]
    [InlineData("https://rbd-office.local/web-apps/apps/spreadsheeteditor/main/index.html")]
    [InlineData("https://rbd-office.local/web-apps/apps/presentationeditor/main/index.html")]
    [InlineData("https://rbd-office.local/assets/fonts/NotoSans-Regular.ttf")]
    [InlineData("about:blank")]
    public void Verify_AllowedVirtualHost_ReturnsTrue(string allowedUrl)
    {
        bool allowed = OfflineSecurityPolicy.IsRequestAllowed(allowedUrl, out string reason);
        Assert.True(allowed, $"Expected '{allowedUrl}' to be allowed, but was blocked: {reason}");
        Assert.Empty(reason);
    }

    [Theory]
    [InlineData("http://rbd-office.local/index.html")] // Non-HTTPS
    [InlineData("ftp://rbd-office.local/file.txt")]
    [InlineData("ws://rbd-office.local/socket")]
    [InlineData("file:///C:/Windows/System32/cmd.exe")]
    [InlineData("javascript:alert(1)")]
    public void Verify_NonHttps_Schemes_Are_Blocked(string url)
    {
        bool allowed = OfflineSecurityPolicy.IsRequestAllowed(url, out string reason);
        Assert.False(allowed, $"Expected scheme for '{url}' to be blocked.");
        Assert.NotEmpty(reason);
    }

    [Theory]
    [InlineData("https://google.com/")]
    [InlineData("https://www.google-analytics.com/collect")]
    [InlineData("https://telemetry.onlyoffice.com/v1/track")]
    [InlineData("https://api.github.com/repos/rbdofis")]
    [InlineData("https://fonts.googleapis.com/css?family=Roboto")]
    [InlineData("https://cdn.jsdelivr.net/npm/bootstrap")]
    [InlineData("https://example.com/")]
    public void Verify_ExternalDomains_Are_Blocked(string externalUrl)
    {
        // Enforcing Rule 5 of AGENTS.md: zero network access / no telemetries / no cloud
        bool allowed = OfflineSecurityPolicy.IsRequestAllowed(externalUrl, out string reason);
        Assert.False(allowed, $"Expected external domain '{externalUrl}' to be strictly blocked.");
        Assert.Contains("Rule 5 of AGENTS.md", reason);
    }

    [Theory]
    [InlineData("http://127.0.0.1:8080/")]
    [InlineData("http://localhost:3000/")]
    [InlineData("https://127.0.0.1:8443/")]
    [InlineData("https://rbd-office.local:8080/index.html")] // Non-default port
    public void Verify_Localhost_And_Custom_TCP_Ports_Are_Blocked(string localNetworkUrl)
    {
        // Enforcing ADR-0004: 'Yerel TCP portu açılmaz'
        bool allowed = OfflineSecurityPolicy.IsRequestAllowed(localNetworkUrl, out string reason);
        Assert.False(allowed, $"Expected TCP port/host '{localNetworkUrl}' to be blocked per ADR-0004.");
        Assert.NotEmpty(reason);
    }

    [Fact]
    public void Verify_SecurityViolation_Event_Fired_And_Audit_Logged()
    {
        OfflineSecurityPolicy.ClearViolations();
        var capturedViolations = new List<BlockedRequestEventArgs>();

        EventHandler<BlockedRequestEventArgs> handler = (_, e) =>
        {
            capturedViolations.Add(e);
        };

        OfflineSecurityPolicy.SecurityViolationDetected += handler;
        try
        {
            const string attackUrl = "https://malicious-telemetry.example.org/leak?token=123";
            bool allowed = OfflineSecurityPolicy.IsRequestAllowed(attackUrl);

            Assert.False(allowed);
            Assert.Single(capturedViolations);
            Assert.Equal(attackUrl, capturedViolations[0].Url);
            Assert.NotEmpty(capturedViolations[0].Reason);
            Assert.NotEmpty(OfflineSecurityPolicy.Violations);
        }
        finally
        {
            OfflineSecurityPolicy.SecurityViolationDetected -= handler;
        }
    }
}
