// DocumentView.swift - RBD OFİS Apple Document Viewer (macOS & iOS)
// AGPL-3.0-or-later

import SwiftUI
import CRbdFfi

#if canImport(UIKit)
import UIKit
typealias PlatformImage = UIImage
#elseif canImport(AppKit)
import AppKit
typealias PlatformImage = NSImage
#endif

public struct DocumentView: View {
    @ObservedObject public var document: ObservableRbdDocument
    @State private var currentPage: Int32 = 1
    @State private var zoomScale: Double = 1.0
    @State private var searchText: String = ""
    @State private var searchResults: [RbdSearchHit] = []

    public init(document: ObservableRbdDocument) {
        self.document = document
    }

    public var body: some View {
        VStack(spacing: 0) {
            // Document Toolbar
            HStack {
                Button(action: { if currentPage > 1 { currentPage -= 1 } }) {
                    Image(systemName: "chevron.left")
                }
                .disabled(currentPage <= 1)

                Text(String(format: NSLocalizedString("page_indicator_format", bundle: .module, comment: ""), currentPage, document.pageCount))
                    .font(.caption)

                Button(action: { if currentPage < document.pageCount { currentPage += 1 } }) {
                    Image(systemName: "chevron.right")
                }
                .disabled(currentPage >= document.pageCount)

                Spacer()

                Button(action: { zoomScale = max(0.5, zoomScale - 0.25) }) {
                    Image(systemName: "minus.magnifyingglass")
                }
                Text(String(format: NSLocalizedString("zoom_format", bundle: .module, comment: ""), Int(zoomScale * 100)))
                    .font(.caption)
                Button(action: { zoomScale = min(4.0, zoomScale + 0.25) }) {
                    Image(systemName: "plus.magnifyingglass")
                }
            }
            .padding(8)

            Divider()

            // Page Render Canvas
            ScrollView([.horizontal, .vertical]) {
                if let image = renderCurrentPage() {
                    #if canImport(UIKit)
                    Image(uiImage: image)
                        .resizable()
                        .aspectRatio(contentMode: .fit)
                        .frame(width: CGFloat(document.width) * CGFloat(zoomScale),
                               height: CGFloat(document.height) * CGFloat(zoomScale))
                    #elseif canImport(AppKit)
                    Image(nsImage: image)
                        .resizable()
                        .aspectRatio(contentMode: .fit)
                        .frame(width: CGFloat(document.width) * CGFloat(zoomScale),
                               height: CGFloat(document.height) * CGFloat(zoomScale))
                    #endif
                } else {
                    ProgressView(NSLocalizedString("loading_document", bundle: .module, comment: ""))
                        .frame(width: 400, height: 600)
                }
            }
        }
    }

    private func renderCurrentPage() -> PlatformImage? {
        let width = Int(Double(document.width) * zoomScale)
        let height = Int(Double(document.height) * zoomScale)
        guard width > 0 && height > 0 else { return nil }

        guard let bgraBytes = try? document.doc.renderTile(
            pageIndex: Int(currentPage - 1),
            tileX: 0,
            tileY: 0,
            width: width,
            height: height,
            zoom: zoomScale,
            pixelFormat: .bgra32
        ) else {
            return nil
        }

        return createPlatformImage(from: bgraBytes, width: width, height: height)
    }

    private func createPlatformImage(from bgraBytes: Data, width: Int, height: Int) -> PlatformImage? {
        guard bgraBytes.count == width * height * 4 else { return nil }
        
        let colorSpace = CGColorSpaceCreateDeviceRGB()
        let bitmapInfo = CGBitmapInfo.byteOrder32Little.rawValue | CGImageAlphaInfo.premultipliedFirst.rawValue // BGRA

        guard let provider = CGDataProvider(data: bgraBytes as CFData),
              let cgImage = CGImage(
                width: width,
                height: height,
                bitsPerComponent: 8,
                bitsPerPixel: 32,
                bytesPerRow: width * 4,
                space: colorSpace,
                bitmapInfo: CGBitmapInfo(rawValue: bitmapInfo),
                provider: provider,
                decode: nil,
                shouldInterpolate: true,
                intent: .defaultIntent
              ) else {
            return nil
        }

        #if canImport(UIKit)
        return UIImage(cgImage: cgImage)
        #elseif canImport(AppKit)
        return NSImage(cgImage: cgImage, size: NSSize(width: width, height: height))
        #endif
    }
}

public class ObservableRbdDocument: ObservableObject {
    public let doc: RbdDocument
    @Published public var pageCount: Int32
    @Published public var width: Double
    @Published public var height: Double

    public init(doc: RbdDocument) {
        self.doc = doc
        self.pageCount = Int32(doc.pageCount)
        let size = doc.pageSize(for: 0)
        self.width = size.width > 0 ? size.width : 595.0
        self.height = size.height > 0 ? size.height : 842.0
    }
}
