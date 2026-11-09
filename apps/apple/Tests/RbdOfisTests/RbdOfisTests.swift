// RbdOfisTests.swift - RBD OFİS Apple Swift Tests
// AGPL-3.0-or-later

import XCTest
import CRbdFfi
@testable import RbdOfis

final class RbdOfisTests: XCTestCase {
    private func locateProjectFile(named name: String) -> URL? {
        let currentDir = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
        let candidates = [
            URL(fileURLWithPath: #file).deletingLastPathComponent().deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent(name),
            currentDir.appendingPathComponent(name),
            currentDir.appendingPathComponent("apps/apple").appendingPathComponent(name)
        ]
        return candidates.first(where: { FileManager.default.fileExists(atPath: $0.path) })
    }

    func testSupportedOfficeExtensions() {
        XCTAssertTrue(RbdOfficeService.isSupportedOfficeFile(path: "report.docx"))
        XCTAssertTrue(RbdOfficeService.isSupportedOfficeFile(path: "table.xlsx"))
        XCTAssertTrue(RbdOfficeService.isSupportedOfficeFile(path: "slides.pptx"))
        XCTAssertFalse(RbdOfficeService.isSupportedOfficeFile(path: "image.png"))
    }

    func testOfficeServiceExtractEmpty() {
        XCTAssertThrowsError(try RbdOfficeService.extractText(at: "/non/existent/doc.docx"))
    }

    func testOfficeServiceInspectEmpty() {
        XCTAssertThrowsError(try RbdOfficeService.inspect(at: "/non/existent/sheet.xlsx"))
    }

    func testOfficeCreateDocxInvalidPath() {
        XCTAssertThrowsError(try RbdOfficeService.createDocx(text: "Hello", outputPath: "/invalid/path/out.docx"))
    }

    func testPixelFormatValues() {
        XCTAssertEqual(RbdPixelFormat.bgra32.rawValue, 0)
        XCTAssertEqual(RbdPixelFormat.rgba32.rawValue, 1)
    }

    func testRbdErrorMapping() {
        switch RbdError.from(code: RBD_ERR_NULL_PTR) {
        case .nullPointer:
            break
        default:
            XCTFail("Expected .nullPointer")
        }

        switch RbdError.from(code: RBD_ERR_BUFFER_TOO_SMALL) {
        case .bufferTooSmall:
            break
        default:
            XCTFail("Expected .bufferTooSmall")
        }
    }

    func testVersionStringNotEmpty() {
        let v = RbdEngine.version
        XCTAssertFalse(v.isEmpty)
    }

    func testLocalizationParity() {
        let keys = [
            "app_name",
            "app_subtitle",
            "page_indicator_format",
            "loading_document",
            "office_editor_ready",
            "pencilkit_unsupported",
            "offline_badge",
            "document_summary",
            "ask_document",
            "zoom_format",
            "camera_usage_description"
        ]
        for key in keys {
            let localized = NSLocalizedString(key, bundle: .module, comment: "")
            XCTAssertFalse(localized.isEmpty, "Missing localization for \(key)")
        }
    }

    func testOfficeNavigationDelegateZeroNetwork() {
        XCTAssertTrue(OfficeNavigationDelegate.isAllowed(url: URL(string: "about:blank")))
        XCTAssertTrue(OfficeNavigationDelegate.isAllowed(url: URL(fileURLWithPath: "/tmp/index.html")))
        XCTAssertTrue(OfficeNavigationDelegate.isAllowed(url: URL(string: "https://rbd-office.local/index.html")))
        XCTAssertFalse(OfficeNavigationDelegate.isAllowed(url: URL(string: "https://google.com")))
        XCTAssertFalse(OfficeNavigationDelegate.isAllowed(url: URL(string: "http://malicious.com")))
        XCTAssertFalse(OfficeNavigationDelegate.isAllowed(url: nil))
    }

    func testInfoPlistValidityAndAppStoreReadiness() {
        guard let url = locateProjectFile(named: "Info.plist"),
              let data = try? Data(contentsOf: url) else {
            XCTFail("Info.plist not found")
            return
        }

        guard let plist = try? PropertyListSerialization.propertyList(from: data, options: [], format: nil) as? [String: Any] else {
            XCTFail("Failed to deserialize Info.plist")
            return
        }

        XCTAssertEqual(plist["CFBundleIdentifier"] as? String, "com.rbdofis.app")
        XCTAssertEqual(plist["CFBundleName"] as? String, "RBD OFİS")
        XCTAssertEqual(plist["CFBundleDisplayName"] as? String, "RBD OFİS")
        XCTAssertEqual(plist["CFBundleVersion"] as? String, "1")
        XCTAssertEqual(plist["CFBundleShortVersionString"] as? String, "1.0.0")
        XCTAssertEqual(plist["LSApplicationCategoryType"] as? String, "public.app-category.productivity")

        let cameraDesc = plist["NSCameraUsageDescription"] as? String
        XCTAssertNotNil(cameraDesc)
        XCTAssertFalse(cameraDesc?.isEmpty ?? true)

        guard let docTypes = plist["CFBundleDocumentTypes"] as? [[String: Any]] else {
            XCTFail("CFBundleDocumentTypes must be an array of dictionaries")
            return
        }
        XCTAssertEqual(docTypes.count, 4, "Expected 4 document types: PDF, DOCX, XLSX, PPTX")

        let allExtensions = docTypes.compactMap { $0["CFBundleTypeExtensions"] as? [String] }.flatMap { $0 }
        XCTAssertTrue(allExtensions.contains("pdf"))
        XCTAssertTrue(allExtensions.contains("docx"))
        XCTAssertTrue(allExtensions.contains("xlsx"))
        XCTAssertTrue(allExtensions.contains("pptx"))

        let allMimeTypes = docTypes.compactMap { $0["CFBundleTypeMIMETypes"] as? [String] }.flatMap { $0 }
        XCTAssertTrue(allMimeTypes.contains("application/pdf"))
        XCTAssertTrue(allMimeTypes.contains("application/vnd.openxmlformats-officedocument.wordprocessingml.document"))
        XCTAssertTrue(allMimeTypes.contains("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"))
        XCTAssertTrue(allMimeTypes.contains("application/vnd.openxmlformats-officedocument.presentationml.presentation"))

        for doc in docTypes {
            XCTAssertEqual(doc["CFBundleTypeRole"] as? String, "Editor")
        }
    }

    func testEntitlementsAppSandboxAndStrictOffline() {
        guard let url = locateProjectFile(named: "RbdOfis.entitlements"),
              let data = try? Data(contentsOf: url) else {
            XCTFail("RbdOfis.entitlements not found")
            return
        }

        guard let plist = try? PropertyListSerialization.propertyList(from: data, options: [], format: nil) as? [String: Any] else {
            XCTFail("Failed to deserialize RbdOfis.entitlements")
            return
        }

        XCTAssertEqual(plist["com.apple.security.app-sandbox"] as? Bool, true)
        XCTAssertEqual(plist["com.apple.security.files.user-selected.read-write"] as? Bool, true)
        XCTAssertEqual(plist["com.apple.security.device.camera"] as? Bool, true)
        XCTAssertEqual(plist["com.apple.security.network.client"] as? Bool, false)
        XCTAssertEqual(plist["com.apple.security.network.server"] as? Bool, false)
    }
}
