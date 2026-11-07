// RbdOfisTests.swift - RBD OFİS Apple Swift Tests
// AGPL-3.0-or-later

import XCTest
import CRbdFfi
@testable import RbdOfis

final class RbdOfisTests: XCTestCase {
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
            "zoom_format"
        ]
        for key in keys {
            let localized = NSLocalizedString(key, bundle: .module, comment: "")
            XCTAssertFalse(localized.isEmpty, "Missing localization for \(key)")
        }
    }
}
