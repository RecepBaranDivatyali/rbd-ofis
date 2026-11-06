// PencilKitCanvas.swift - Apple Pencil & Stylus Annotation Canvas (iOS / iPadOS)
// AGPL-3.0-or-later

import SwiftUI

#if canImport(PencilKit) && canImport(UIKit)
import PencilKit
import UIKit

public struct PencilKitCanvas: UIViewRepresentable {
    @Binding public var canvasView: PKCanvasView
    public let toolPicker = PKToolPicker()

    public init(canvasView: Binding<PKCanvasView>) {
        self._canvasView = canvasView
    }

    public func makeUIView(context: Context) -> PKCanvasView {
        canvasView.drawingPolicy = .anyInput // Supports both Apple Pencil and Touch
        canvasView.backgroundColor = .clear
        canvasView.isOpaque = false

        // Attach system tool picker (Pen, Pencil, Marker, Eraser, Ruler, Color Palette)
        toolPicker.setVisible(true, forFirstResponder: canvasView)
        toolPicker.addObserver(canvasView)
        canvasView.becomeFirstResponder()

        return canvasView
    }

    public func updateUIView(_ uiView: PKCanvasView, context: Context) {
        // Updates can be synced with annotation layers
    }
}
#else
public struct PencilKitCanvas: View {
    public init() {}
    public var body: some View {
        Text("PencilKit yalnızca iPadOS ve iOS cihazlarda desteklenir.")
            .foregroundColor(.secondary)
            .padding()
    }
}
#endif
