// RbdOfisTests.swift - RBD OFİS Apple Swift Tests
// AGPL-3.0-or-later

import XCTest
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
}
