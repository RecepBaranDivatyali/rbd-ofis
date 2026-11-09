// OfficeHostView.swift - Offline Office Web Host View for macOS & iOS
// AGPL-3.0-or-later

import SwiftUI
import WebKit

private func locateOfficeHtml() -> URL? {
    #if SWIFT_PACKAGE
    if let url = Bundle.module.url(forResource: "index", withExtension: "html", subdirectory: "assets") {
        return url
    }
    if let url = Bundle.module.url(forResource: "index", withExtension: "html") {
        return url
    }
    #endif
    if let url = Bundle.main.url(forResource: "index", withExtension: "html", subdirectory: "assets") {
        return url
    }
    return Bundle.main.url(forResource: "index", withExtension: "html")
}

private func getFallbackOfficeHtml() -> String {
    let appTitle = NSLocalizedString("app_name", bundle: .module, comment: "App Name")
    let readyText = NSLocalizedString("office_editor_ready", bundle: .module, comment: "Office Editor Ready")
    return "<html><body style='font-family:sans-serif;padding:20px;'><h3>\(appTitle)</h3><p>\(readyText)</p></body></html>"
}

/// Enforces strict offline zero-network isolation for WKWebView.
/// Only permits local file URLs, local virtual host (https://rbd-office.local/), and about:blank.
public final class OfficeNavigationDelegate: NSObject, WKNavigationDelegate {
    public static let shared = OfficeNavigationDelegate()

    public static func isAllowed(url: URL?) -> Bool {
        guard let url = url else { return false }
        if url.absoluteString == "about:blank" {
            return true
        }
        if url.isFileURL {
            return true
        }
        if url.scheme?.lowercased() == "https" && url.host?.lowercased() == "rbd-office.local" {
            return true
        }
        return false
    }

    public func webView(
        _ webView: WKWebView,
        decidePolicyFor navigationAction: WKNavigationAction,
        decisionHandler: @escaping (WKNavigationActionPolicy) -> Void
    ) {
        if Self.isAllowed(url: navigationAction.request.url) {
            decisionHandler(.allow)
        } else {
            decisionHandler(.cancel)
        }
    }
}

#if canImport(UIKit)
import UIKit

public struct OfficeHostView: UIViewRepresentable {
    public let initialHtml: String?
    
    public init(initialHtml: String? = nil) {
        self.initialHtml = initialHtml
    }

    public func makeCoordinator() -> OfficeNavigationDelegate {
        OfficeNavigationDelegate.shared
    }

    public func makeUIView(context: Context) -> WKWebView {
        let config = WKWebViewConfiguration()
        // Strict Offline Policy: Zero network access
        config.allowsAirPlayForMediaPlayback = false
        config.defaultWebpagePreferences.allowsContentJavaScript = true

        let webView = WKWebView(frame: .zero, configuration: config)
        webView.navigationDelegate = context.coordinator
        loadOfflineContent(into: webView)
        return webView
    }

    public func updateUIView(_ uiView: WKWebView, context: Context) {}

    private func loadOfflineContent(into webView: WKWebView) {
        if let html = initialHtml {
            webView.loadHTMLString(html, baseURL: URL(string: "https://rbd-office.local/"))
        } else if let resourceUrl = locateOfficeHtml() {
            webView.loadFileURL(resourceUrl, allowingReadAccessTo: resourceUrl.deletingLastPathComponent())
        } else {
            webView.loadHTMLString(getFallbackOfficeHtml(), baseURL: nil)
        }
    }
}
#elseif canImport(AppKit)
import AppKit

public struct OfficeHostView: NSViewRepresentable {
    public let initialHtml: String?

    public init(initialHtml: String? = nil) {
        self.initialHtml = initialHtml
    }

    public func makeCoordinator() -> OfficeNavigationDelegate {
        OfficeNavigationDelegate.shared
    }

    public func makeNSView(context: Context) -> WKWebView {
        let config = WKWebViewConfiguration()
        config.defaultWebpagePreferences.allowsContentJavaScript = true

        let webView = WKWebView(frame: .zero, configuration: config)
        webView.navigationDelegate = context.coordinator
        loadOfflineContent(into: webView)
        return webView
    }

    public func updateNSView(_ nsView: WKWebView, context: Context) {}

    private func loadOfflineContent(into webView: WKWebView) {
        if let html = initialHtml {
            webView.loadHTMLString(html, baseURL: URL(string: "https://rbd-office.local/"))
        } else if let resourceUrl = locateOfficeHtml() {
            webView.loadFileURL(resourceUrl, allowingReadAccessTo: resourceUrl.deletingLastPathComponent())
        } else {
            webView.loadHTMLString(getFallbackOfficeHtml(), baseURL: nil)
        }
    }
}
#endif
