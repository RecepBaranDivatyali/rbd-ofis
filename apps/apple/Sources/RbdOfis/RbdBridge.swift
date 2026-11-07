import Foundation
import CRbdFfi

/// Supported pixel formats matching Rust `rbd_engine::PixelFormat`.
public enum RbdPixelFormat: Int32 {
    /// 32-bit BGRA (Blue, Green, Red, Alpha) - Apple CoreGraphics & Direct2D standard.
    case bgra32 = 0
    /// 32-bit RGBA (Red, Green, Blue, Alpha).
    case rgba32 = 1
}

/// Search hit representation for in-document text queries.
public struct RbdSearchHit {
    public let pageIndex: Int
    public let text: String
    public let bounds: [Double]

    public init(pageIndex: Int, text: String, bounds: [Double]) {
        self.pageIndex = pageIndex
        self.text = text
        self.bounds = bounds
    }
}

/// Structured Swift error for RBD OFİS native engine operations.
public enum RbdError: Error, LocalizedError {
    case nullPointer
    case invalidParameter
    case fileIO(String)
    case corruptPdf(String)
    case passwordRequired
    case pageOutOfRange(index: Int32, total: Int32)
    case bufferTooSmall
    case engineFailure(String)
    case panic(String)
    case unknown(Int32)

    public var errorDescription: String? {
        switch self {
        case .nullPointer: return "Null pointer passed to native API."
        case .invalidParameter: return "Invalid parameter provided."
        case .fileIO(let msg): return "File I/O error: \(msg)"
        case .corruptPdf(let msg): return "Corrupt or invalid PDF: \(msg)"
        case .passwordRequired: return "Password required for encrypted document."
        case .pageOutOfRange(let i, let t): return "Page \(i) out of range (total: \(t))."
        case .bufferTooSmall: return "Buffer too small."
        case .engineFailure(let msg): return "Engine failure: \(msg)"
        case .panic(let msg): return "Rust core panic caught: \(msg)"
        case .unknown(let code): return "Unknown native error code: \(code)"
        }
    }

    static func from(code: Int32, message: String? = nil) -> RbdError {
        let msg: String
        if let message = message {
            msg = message
        } else if let errPtr = rbd_get_last_error() {
            msg = String(cString: errPtr)
        } else {
            msg = "No error details available"
        }

        switch code {
        case RBD_ERR_NULL_PTR: return .nullPointer
        case RBD_ERR_INVALID_PARAM: return .invalidParameter
        case RBD_ERR_FILE_IO: return .fileIO(msg)
        case RBD_ERR_CORRUPT_PDF: return .corruptPdf(msg)
        case RBD_ERR_PASSWORD_REQUIRED: return .passwordRequired
        case RBD_ERR_PAGE_OUT_OF_RANGE: return .pageOutOfRange(index: 0, total: 0)
        case RBD_ERR_BUFFER_TOO_SMALL: return .bufferTooSmall
        case RBD_ERR_ENGINE_FAILURE: return .engineFailure(msg)
        case RBD_ERR_PANIC: return .panic(msg)
        default: return .unknown(code)
        }
    }
}

/// Helper executing native C buffer operations with dynamic buffer doubling on `RBD_ERR_BUFFER_TOO_SMALL`.
private func callWithDynamicBuffer(
    initialCapacity: Int = 65536,
    maxCapacity: Int = 64 * 1024 * 1024,
    _ body: (UnsafeMutablePointer<CChar>, Int) -> Int32
) throws -> String {
    var capacity = initialCapacity
    while capacity <= maxCapacity {
        var buffer = [CChar](repeating: 0, count: capacity)
        let status = buffer.withUnsafeMutableBufferPointer { ptr -> Int32 in
            guard let base = ptr.baseAddress else { return RBD_ERR_NULL_PTR }
            return body(base, capacity)
        }
        if status == RBD_OK {
            return String(cString: buffer)
        } else if status == RBD_ERR_BUFFER_TOO_SMALL {
            // Buffer too small: double capacity and retry
            capacity *= 2
        } else {
            throw RbdError.from(code: status)
        }
    }
    throw RbdError.bufferTooSmall
}

/// Native engine wrapper.
public final class RbdEngine {
    private var handle: RbdEngineHandle?

    public init() throws {
        var h: RbdEngineHandle = nil
        let status = rbd_engine_init(&h)
        guard status == RBD_OK, let h = h else {
            throw RbdError.from(code: status)
        }
        self.handle = h
    }

    deinit {
        if let h = handle {
            rbd_engine_free(h)
        }
    }

    public func openDocument(at path: String) throws -> RbdDocument {
        guard let h = handle else { throw RbdError.nullPointer }
        var docHandle: RbdDocumentHandle = nil
        let status = path.withCString { cPath in
            rbd_document_open_file(h, cPath, &docHandle)
        }
        guard status == RBD_OK, let doc = docHandle else {
            throw RbdError.from(code: status)
        }
        return RbdDocument(handle: doc, filePath: path)
    }

    public static var version: String {
        guard let v = rbd_version_string() else { return "0.1.0" }
        return String(cString: v)
    }
}

/// Open document wrapper providing page count, text extraction, search, annotations, and rendering.
public final class RbdDocument {
    private var handle: RbdDocumentHandle?
    public let filePath: String?

    init(handle: RbdDocumentHandle, filePath: String? = nil) {
        self.handle = handle
        self.filePath = filePath
    }

    deinit {
        if let h = handle {
            rbd_document_free(h)
        }
    }

    public var pageCount: Int {
        guard let h = handle else { return 0 }
        var count: Int32 = 0
        let status = rbd_document_get_page_count(h, &count)
        return status == RBD_OK ? Int(count) : 0
    }

    public func pageSize(for pageIndex: Int) -> (width: Double, height: Double) {
        guard let h = handle else { return (0, 0) }
        var w: Double = 0
        var hPt: Double = 0
        let status = rbd_document_get_page_size(h, Int32(pageIndex), &w, &hPt)
        return status == RBD_OK ? (w, hPt) : (0, 0)
    }

    public func extractText(for pageIndex: Int) throws -> String {
        guard let h = handle else { throw RbdError.nullPointer }
        var textPtr: UnsafeMutablePointer<CChar>? = nil
        let status = rbd_document_extract_page_text(h, UInt32(pageIndex), &textPtr)
        guard status == RBD_OK, let ptr = textPtr else {
            throw RbdError.from(code: status)
        }
        defer { rbd_free_string(ptr) }
        return String(cString: ptr)
    }

    public func renderTile(
        pageIndex: Int,
        tileX: Int,
        tileY: Int,
        width: Int,
        height: Int,
        zoom: Double,
        pixelFormat: RbdPixelFormat = .bgra32
    ) throws -> Data {
        guard let h = handle else { throw RbdError.nullPointer }
        let stride = width * 4
        var buffer = Data(count: stride * height)
        let status = buffer.withUnsafeMutableBytes { (rawPtr: UnsafeMutableRawBufferPointer) -> Int32 in
            guard let baseAddress = rawPtr.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                return RBD_ERR_NULL_PTR
            }
            return rbd_render_tile_into_buffer(
                h,
                Int32(pageIndex),
                Int32(tileX),
                Int32(tileY),
                Int32(width),
                Int32(height),
                zoom,
                baseAddress,
                Int32(stride),
                pixelFormat.rawValue
            )
        }
        guard status == RBD_OK else {
            throw RbdError.from(code: status)
        }
        return buffer
    }

    // MARK: - Annotation Management & Undo/Redo

    public func addAnnotation(json: String) throws {
        guard let h = handle else { throw RbdError.nullPointer }
        let status = json.withCString { cJson in
            rbd_document_add_annotation_json(h, cJson)
        }
        guard status == RBD_OK else {
            throw RbdError.from(code: status)
        }
    }

    public func removeAnnotation(id: UInt64) throws {
        guard let h = handle else { throw RbdError.nullPointer }
        let status = rbd_document_remove_annotation(h, id)
        guard status == RBD_OK else {
            throw RbdError.from(code: status)
        }
    }

    public func updateAnnotation(json: String) throws {
        guard let h = handle else { throw RbdError.nullPointer }
        let status = json.withCString { cJson in
            rbd_document_update_annotation_json(h, cJson)
        }
        guard status == RBD_OK else {
            throw RbdError.from(code: status)
        }
    }

    public func annotationsJson(for pageIndex: Int) throws -> String {
        guard let h = handle else { throw RbdError.nullPointer }
        var jsonPtr: UnsafeMutablePointer<CChar>? = nil
        let status = rbd_document_get_annotations_json(h, UInt32(pageIndex), &jsonPtr)
        guard status == RBD_OK, let ptr = jsonPtr else {
            throw RbdError.from(code: status)
        }
        defer { rbd_free_string(ptr) }
        return String(cString: ptr)
    }

    public func undo() -> Bool {
        guard let h = handle else { return false }
        var undone: Int32 = 0
        let status = rbd_document_undo(h, &undone)
        return status == RBD_OK && undone != 0
    }

    public func redo() -> Bool {
        guard let h = handle else { return false }
        var redone: Int32 = 0
        let status = rbd_document_redo(h, &redone)
        return status == RBD_OK && redone != 0
    }

    public var canUndo: Bool {
        guard let h = handle else { return false }
        var can: Int32 = 0
        let status = rbd_document_can_undo(h, &can)
        return status == RBD_OK && can != 0
    }

    public var canRedo: Bool {
        guard let h = handle else { return false }
        var can: Int32 = 0
        let status = rbd_document_can_redo(h, &can)
        return status == RBD_OK && can != 0
    }

    // MARK: - Phase 8 Offline AI & Reflow Editing (Dynamic Buffer)

    public func summarize(maxSentences: Int = 5) throws -> String {
        guard let h = handle else { throw RbdError.nullPointer }
        return try callWithDynamicBuffer { buf, cap in
            rbd_ai_summarize_document(h, UInt32(maxSentences), buf, cap)
        }
    }

    public func ask(query: String) throws -> String {
        guard let h = handle else { throw RbdError.nullPointer }
        return try query.withCString { cQuery in
            try callWithDynamicBuffer { buf, cap in
                rbd_ai_ask_document(h, cQuery, buf, cap)
            }
        }
    }

    public func reflowPage(pageIndex: Int) throws -> String {
        guard let h = handle else { throw RbdError.nullPointer }
        return try callWithDynamicBuffer { buf, cap in
            rbd_document_reflow_page_text(h, UInt32(pageIndex), buf, cap)
        }
    }

    public func replaceParagraph(
        inputPath: String,
        pageIndex: Int,
        x0: Double,
        y0: Double,
        x1: Double,
        y1: Double,
        newText: String,
        fontSize: Double = 11.0,
        outputPath: String
    ) throws {
        guard let h = handle else { throw RbdError.nullPointer }
        let status = inputPath.withCString { cIn in
            newText.withCString { cText in
                outputPath.withCString { cOut in
                    rbd_document_replace_paragraph_text(
                        h,
                        cIn,
                        UInt32(pageIndex),
                        Float(x0),
                        Float(y0),
                        Float(x1),
                        Float(y1),
                        cText,
                        Float(fontSize),
                        cOut
                    )
                }
            }
        }
        guard status == RBD_OK else {
            throw RbdError.from(code: status)
        }
    }
}

/// Headless Office document inspection and extraction service (Phase 6 & 7).
public enum RbdOfficeService {
    public static func isSupportedOfficeFile(path: String) -> Bool {
        let ext = (path as NSString).pathExtension.lowercased()
        return ["docx", "doc", "xlsx", "xls", "pptx", "ppt"].contains(ext)
    }

    public static func extractText(at path: String) throws -> String {
        return try path.withCString { cPath in
            try callWithDynamicBuffer { buf, cap in
                rbd_office_extract_text(cPath, buf, cap)
            }
        }
    }

    public static func inspect(at path: String) throws -> String {
        return try path.withCString { cPath in
            try callWithDynamicBuffer { buf, cap in
                rbd_office_inspect(cPath, buf, cap)
            }
        }
    }

    public static func createDocx(text: String, outputPath: String) throws {
        let status = text.withCString { cText in
            outputPath.withCString { cOut in
                rbd_office_create_docx(cText, cOut)
            }
        }
        guard status == RBD_OK else {
            throw RbdError.from(code: status)
        }
    }
}
