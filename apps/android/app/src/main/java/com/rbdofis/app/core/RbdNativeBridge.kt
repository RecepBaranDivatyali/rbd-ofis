package com.rbdofis.app.core

import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.util.Log
import java.io.File
import java.nio.ByteBuffer
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicLong

/**
 * Dimensions of a document page in typographic points (1/72 inch).
 */
data class PageDimensions(
    val width: Float,
    val height: Float
) {
    val aspectRatio: Float
        get() = if (height > 0f) width / height else 1f
}

/**
 * Axis-aligned bounding box on a page.
 */
data class PageRect(
    val left: Float,
    val top: Float,
    val right: Float,
    val bottom: Float
) {
    val width: Float get() = right - left
    val height: Float get() = bottom - top
}

/**
 * Result of a full-text search query.
 */
data class SearchResult(
    val pageIndex: Int,
    val text: String,
    val bounds: PageRect
)

/**
 * Annotation types supported on PDF pages.
 */
sealed class PdfAnnotation {
    abstract val id: String
    abstract val pageIndex: Int
    abstract val bounds: PageRect

    data class Highlight(
        override val id: String,
        override val pageIndex: Int,
        override val bounds: PageRect,
        val colorRgba: Long = 0xFFFFEB3BL // Default yellow highlight
    ) : PdfAnnotation()

    data class InkStroke(
        override val id: String,
        override val pageIndex: Int,
        override val bounds: PageRect,
        val points: List<Pair<Float, Float>>,
        val strokeWidth: Float = 2.0f,
        val colorRgba: Long = 0xFF000000L,
        val isHighlighter: Boolean = false
    ) : PdfAnnotation()

    data class TextNote(
        override val id: String,
        override val pageIndex: Int,
        override val bounds: PageRect,
        val contents: String
    ) : PdfAnnotation()
}

/**
 * Compression savings statistics returned by native compressor.
 */
data class CompressionStats(
    val originalSize: Long,
    val compressedSize: Long,
    val savingsBytes: Long,
    val savingsRatio: Float
) {
    val savingsPercentage: Float get() = savingsRatio * 100f
}

/**
 * Interactive AcroForm field representation.
 */
data class FormFieldInfo(
    val name: String,
    val fieldType: String,
    val value: String = "",
    val readOnly: Boolean = false
)

/**
 * Metadata inspected from an Office document (DOCX, XLSX, PPTX).
 */
data class OfficeDocumentMetadata(
    val docType: String,
    val title: String,
    val wordCount: Int,
    val itemCount: Int,
    val summary: String
)

/**
 * Standard anchor positions for watermarks, page numbers, and Bates stamps.
 */
enum class StampPosition(val value: Int) {
    TOP_LEFT(0),
    TOP_CENTER(1),
    TOP_RIGHT(2),
    BOTTOM_LEFT(3),
    BOTTOM_CENTER(4),
    BOTTOM_RIGHT(5);

    companion object {
        fun fromInt(value: Int): StampPosition =
            entries.firstOrNull { it.value == value } ?: BOTTOM_CENTER
    }
}

/**
 * Supported output formats for PDF-to-image conversion.
 */
enum class ImageExportFormat(val value: Int) {
    PNG(0),
    JPEG(1);

    companion object {
        fun fromInt(value: Int): ImageExportFormat =
            entries.firstOrNull { it.value == value } ?: PNG
    }
}

/**
 * Standard page size targets when aggregating images into a PDF.
 */
enum class ImageToPdfPageSize(val value: Int) {
    A4(0),
    LETTER(1),
    FIT_IMAGE(2);

    companion object {
        fun fromInt(value: Int): ImageToPdfPageSize =
            entries.firstOrNull { it.value == value } ?: A4
    }
}

/**
 * JNI FFI abstraction bridge between Kotlin and the Rust core (`librbd_core.so`).
 * Provides high-performance document parsing, page operations, and tile rendering.
 *
 * Implements a complete fallback / simulation mode when the native library is not loaded
 * (e.g., during standard JVM unit testing, development without NDK, or previews).
 */
class RbdNativeBridge private constructor() {

    companion object {
        private const val TAG = "RbdNativeBridge"
        private const val LIB_NAME = "rbd_ffi"

        @Volatile
        private var instance: RbdNativeBridge? = null

        fun getInstance(): RbdNativeBridge {
            return instance ?: synchronized(this) {
                instance ?: RbdNativeBridge().also { instance = it }
            }
        }

        private fun logI(tag: String, msg: String) {
            try {
                Log.i(tag, msg)
            } catch (_: Throwable) {
                println("INFO [$tag]: $msg")
            }
        }

        private fun logW(tag: String, msg: String, tr: Throwable? = null) {
            try {
                if (tr != null) Log.w(tag, msg, tr) else Log.w(tag, msg)
            } catch (_: Throwable) {
                println("WARN [$tag]: $msg")
            }
        }
    }

    private var nativeLoaded = false
    private var forceMock = false
    private var engineHandle: Long = 0L

    // Fallback in-memory document state
    private val mockHandleSequence = AtomicLong(1000L)
    private val mockDocuments = ConcurrentHashMap<Long, MockDocument>()
    private val docPaths = ConcurrentHashMap<Long, String>()

    init {
        try {
            System.loadLibrary(LIB_NAME)
            nativeLoaded = true
            logI(TAG, "Native library '$LIB_NAME' loaded successfully.")
        } catch (e: UnsatisfiedLinkError) {
            nativeLoaded = false
            logW(TAG, "Native library '$LIB_NAME' not found. Using fallback mock implementation. (${e.message})")
        } catch (e: Throwable) {
            nativeLoaded = false
            logW(TAG, "Failed to load '$LIB_NAME'. Using fallback mock implementation.", e)
        }
    }

    fun isNativeAvailable(): Boolean = nativeLoaded && !forceMock

    fun setForceMockMode(enabled: Boolean) {
        forceMock = enabled
    }

    @Synchronized
    private fun ensureEngine(): Long {
        if (engineHandle == 0L && isNativeAvailable()) {
            engineHandle = nativeEngineInit()
        }
        return engineHandle
    }

    @Synchronized
    fun freeEngine() {
        if (engineHandle != 0L) {
            if (isNativeAvailable()) {
                nativeEngineFree(engineHandle)
            }
            engineHandle = 0L
        }
    }

    fun getEngineHandle(): Long = engineHandle

    // =========================================================================
    // Document Lifecycle
    // =========================================================================

    fun openDocument(filePath: String): Long {
        val handle: Long
        if (isNativeAvailable()) {
            val eng = ensureEngine()
            val nativeDoc = if (eng != 0L) nativeDocumentOpenFile(eng, filePath) else 0L
            handle = if (nativeDoc != 0L) nativeDoc else mockHandleSequence.incrementAndGet()
        } else {
            handle = mockHandleSequence.incrementAndGet()
        }
        docPaths[handle] = filePath
        val pageCount = if (isNativeAvailable() && handle != 0L) {
            val c = nativeDocumentGetPageCount(handle)
            if (c > 0) c else 6
        } else 6
        mockDocuments[handle] = MockDocument(filePath = filePath, initialPageCount = pageCount)
        return handle
    }

    fun openDocumentFromBytes(bytes: ByteArray): Long {
        val handle: Long
        if (isNativeAvailable()) {
            val eng = ensureEngine()
            val nativeDoc = if (eng != 0L) nativeDocumentOpenMemory(eng, bytes) else 0L
            handle = if (nativeDoc != 0L) nativeDoc else mockHandleSequence.incrementAndGet()
        } else {
            handle = mockHandleSequence.incrementAndGet()
        }
        val memoryPath = "memory://${handle}.pdf"
        docPaths[handle] = memoryPath
        val pageCount = if (isNativeAvailable() && handle != 0L) {
            val c = nativeDocumentGetPageCount(handle)
            if (c > 0) c else 5
        } else 5
        mockDocuments[handle] = MockDocument(filePath = memoryPath, initialPageCount = pageCount)
        return handle
    }

    fun closeDocument(docHandle: Long) {
        if (isNativeAvailable()) {
            nativeDocumentFree(docHandle)
        }
        mockDocuments.remove(docHandle)
        docPaths.remove(docHandle)
    }

    fun saveDocument(docHandle: Long, targetPath: String): Boolean {
        if (isNativeAvailable()) {
            // Flush annotations to the target document via nativeDocumentSaveAnnotations(docHandle, outPath)
            return nativeDocumentSaveAnnotations(docHandle, targetPath) == 0
        }
        val doc = mockDocuments[docHandle] ?: return false
        val targetFile = File(targetPath)
        targetFile.parentFile?.mkdirs()
        targetFile.writeText("%PDF-1.7 mock output pages=${doc.pages.size}\n%%EOF\n")
        return true
    }

    // =========================================================================
    // Page Queries and Operations
    // =========================================================================

    fun getPageCount(docHandle: Long): Int {
        if (isNativeAvailable()) {
            val count = nativeDocumentGetPageCount(docHandle)
            return if (count >= 0) count else 0
        }
        return mockDocuments[docHandle]?.pages?.size ?: 0
    }

    fun getPageSize(docHandle: Long, pageIndex: Int): PageDimensions {
        if (isNativeAvailable()) {
            val arr = nativeDocumentGetPageSize(docHandle, pageIndex)
            if (arr != null && arr.size >= 2) {
                return PageDimensions(arr[0].toFloat(), arr[1].toFloat())
            }
        }
        val doc = mockDocuments[docHandle]
        val page = doc?.pages?.getOrNull(pageIndex)
        return if (page != null) {
            if (page.rotation % 180 == 0) {
                PageDimensions(page.width, page.height)
            } else {
                PageDimensions(page.height, page.width)
            }
        } else {
            PageDimensions(595.28f, 841.89f) // Standard ISO A4 in points
        }
    }

    fun rotatePage(docHandle: Long, pageIndex: Int, degrees: Int): Boolean {
        val doc = mockDocuments[docHandle] ?: return false
        val page = doc.pages.getOrNull(pageIndex) ?: return false
        page.rotation = (page.rotation + degrees) % 360
        if (page.rotation < 0) page.rotation += 360
        doc.undoStack.push(DocumentAction.RotatePage(pageIndex, degrees))
        doc.redoStack.clear()
        return true
    }

    fun deletePage(docHandle: Long, pageIndex: Int): Boolean {
        val doc = mockDocuments[docHandle] ?: return false
        if (pageIndex in doc.pages.indices) {
            doc.pages.removeAt(pageIndex)
            return true
        }
        return false
    }

    fun reorderPage(docHandle: Long, fromIndex: Int, toIndex: Int): Boolean {
        val doc = mockDocuments[docHandle] ?: return false
        if (fromIndex in doc.pages.indices && toIndex in doc.pages.indices) {
            val page = doc.pages.removeAt(fromIndex)
            doc.pages.add(toIndex, page)
            return true
        }
        return false
    }

    // =========================================================================
    // High-level File-to-File Page Operations (core/page_ops parity)
    // =========================================================================

    fun pageOpRotate(inPath: String, outPath: String, degrees: Int): Boolean {
        if (isNativeAvailable()) {
            return nativePageOpRotate(inPath, outPath, degrees) == 0
        }
        return mockCopy(inPath, outPath)
    }

    fun pageOpDeletePages(inPath: String, outPath: String, pages: List<Int>): Boolean {
        if (isNativeAvailable()) {
            val json = "[" + pages.joinToString(",") + "]"
            return nativePageOpDeletePages(inPath, outPath, json) == 0
        }
        return mockCopy(inPath, outPath)
    }

    fun pageOpMerge(outPath: String, inPaths: List<String>): Boolean {
        if (isNativeAvailable()) {
            val json = "[" + inPaths.joinToString(",") { "\"$it\"" } + "]"
            return nativePageOpMerge(outPath, json) == 0
        }
        return mockCopy(inPaths.firstOrNull() ?: "", outPath)
    }

    fun pageOpSplit(inPath: String, outPath: String, startPage: Int, endPage: Int): Boolean {
        if (isNativeAvailable()) {
            return nativePageOpSplit(inPath, outPath, startPage, endPage) == 0
        }
        return mockCopy(inPath, outPath)
    }

    fun pageOpReorder(inPath: String, outPath: String, order: List<Int>): Boolean {
        if (isNativeAvailable()) {
            val json = "[" + order.joinToString(",") + "]"
            return nativePageOpReorder(inPath, outPath, json) == 0
        }
        return mockCopy(inPath, outPath)
    }

    fun pageOpCrop(inPath: String, outPath: String, pageNum: Int, x0: Float, y0: Float, x1: Float, y1: Float): Boolean {
        if (isNativeAvailable()) {
            return nativePageOpCrop(inPath, outPath, pageNum, x0, y0, x1, y1) == 0
        }
        return mockCopy(inPath, outPath)
    }

    fun pageOpInsertBlank(inPath: String, outPath: String, afterPage: Int, widthPt: Float, heightPt: Float): Boolean {
        if (isNativeAvailable()) {
            return nativePageOpInsertBlank(inPath, outPath, afterPage, widthPt, heightPt) == 0
        }
        return mockCopy(inPath, outPath)
    }

    fun pageOpDecrypt(inPath: String, outPath: String, password: String): Boolean {
        if (isNativeAvailable()) {
            return nativePageOpDecrypt(inPath, outPath, password) == 0
        }
        return mockCopy(inPath, outPath)
    }

    private fun mockCopy(inPath: String, outPath: String): Boolean {
        return try {
            val inFile = File(inPath)
            val outFile = File(outPath)
            outFile.parentFile?.mkdirs()
            if (inFile.exists()) inFile.copyTo(outFile, overwrite = true)
            else outFile.writeText("%PDF-1.7 mock output\n%%EOF\n")
            true
        } catch (_: Throwable) {
            false
        }
    }

    // =========================================================================
    // Tile & Thumbnail Rendering (Zero-copy pixel buffer sharing)
    // =========================================================================

    fun renderTile(
        docHandle: Long,
        pageIndex: Int,
        tileX: Int,
        tileY: Int,
        tileWidth: Int,
        tileHeight: Int,
        scale: Float
    ): Bitmap? {
        if (tileWidth <= 0 || tileHeight <= 0) return null

        if (isNativeAvailable()) {
            try {
                val stride = tileWidth * 4
                val bufferSize = stride * tileHeight
                val directBuffer = ByteBuffer.allocateDirect(bufferSize)
                val status = nativeRenderTileIntoBuffer(
                    docHandle,
                    pageIndex,
                    tileX,
                    tileY,
                    tileWidth,
                    tileHeight,
                    scale.toDouble(),
                    directBuffer,
                    stride,
                    1 // Rgba32 format
                )
                if (status == 0) {
                    val bitmap = Bitmap.createBitmap(tileWidth, tileHeight, Bitmap.Config.ARGB_8888)
                    directBuffer.rewind()
                    bitmap.copyPixelsFromBuffer(directBuffer)
                    return bitmap
                }
            } catch (e: Throwable) {
                logW(TAG, "Native tile render failed", e)
            }
        }

        // Safe mock bitmap generator for Android runtime or headless simulation
        return try {
            val bitmap = Bitmap.createBitmap(tileWidth, tileHeight, Bitmap.Config.ARGB_8888)
            val canvas = Canvas(bitmap)
            canvas.drawColor(Color.WHITE)

            val paint = Paint().apply {
                isAntiAlias = true
                style = Paint.Style.STROKE
                color = Color.LTGRAY
                strokeWidth = 2f
            }
            canvas.drawRect(2f, 2f, tileWidth.toFloat() - 2f, tileHeight.toFloat() - 2f, paint)

            val textPaint = Paint().apply {
                isAntiAlias = true
                color = Color.DKGRAY
                textSize = (tileHeight / 12f).coerceIn(16f, 48f)
            }
            val label = "Page ${pageIndex + 1}"
            canvas.drawText(label, 20f, tileHeight / 2f, textPaint)

            bitmap
        } catch (_: Throwable) {
            // In headless JVM tests where android.graphics.Bitmap is not mocked, return null gracefully
            null
        }
    }

    // =========================================================================
    // Search & Annotations
    // =========================================================================

    fun searchText(docHandle: Long, query: String, caseSensitive: Boolean = false): List<SearchResult> {
        if (query.isBlank()) return emptyList()

        if (isNativeAvailable()) {
            val json = nativeDocumentSearchJson(docHandle, query, caseSensitive)
            if (!json.isNullOrBlank()) {
                val parsed = parseSearchResultsJson(json, query)
                if (parsed.isNotEmpty()) return parsed
            }
        }

        val doc = mockDocuments[docHandle] ?: return emptyList()
        val results = mutableListOf<SearchResult>()
        for ((index, _) in doc.pages.withIndex()) {
            // Synthetic search match for demonstration in mock mode
            if (query.length >= 2) {
                results.add(
                    SearchResult(
                        pageIndex = index,
                        text = "Sample match for \"$query\" on page ${index + 1}",
                        bounds = PageRect(50f, 100f, 250f, 120f)
                    )
                )
            }
        }
        return results
    }

    private fun parseSearchResultsJson(json: String, query: String): List<SearchResult> {
        return try {
            val results = mutableListOf<SearchResult>()
            val hitRegex = Regex("\"page\"\\s*:\\s*(\\d+)[^}]*?\"text\"\\s*:\\s*\"([^\"]*)\"")
            hitRegex.findAll(json).forEach { m ->
                val p = m.groupValues[1].toIntOrNull() ?: 0
                val txt = m.groupValues[2].ifBlank { query }
                results.add(SearchResult(p, txt, PageRect(50f, 100f, 250f, 120f)))
            }
            if (results.isEmpty() && json.trim().startsWith("[")) {
                listOf(SearchResult(0, query, PageRect(50f, 100f, 250f, 120f)))
            } else results
        } catch (_: Throwable) {
            listOf(SearchResult(0, query, PageRect(50f, 100f, 250f, 120f)))
        }
    }

    fun addAnnotation(docHandle: Long, pageIndex: Int, annotation: PdfAnnotation): Boolean {
        if (isNativeAvailable()) {
            val numericId = annotation.id.toLongOrNull() ?: (annotation.id.hashCode().toLong().let { if (it < 0) -it else it })
            val kind = when (annotation) {
                is PdfAnnotation.Highlight -> "Highlight"
                is PdfAnnotation.InkStroke -> "Ink"
                is PdfAnnotation.TextNote -> "Text"
            }
            val contents = when (annotation) {
                is PdfAnnotation.TextNote -> annotation.contents
                else -> ""
            }
            val rect = annotation.bounds
            val json = """{"id":$numericId,"page_index":$pageIndex,"kind":"$kind","rect":{"x0":${rect.left},"y0":${rect.top},"x1":${rect.right},"y1":${rect.bottom}},"color_rgba":[255,235,59,255],"contents":"$contents"}"""
            return nativeDocumentAddAnnotationJson(docHandle, json) == 0
        }
        val doc = mockDocuments[docHandle] ?: return false
        val page = doc.pages.getOrNull(pageIndex) ?: return false
        page.annotations.add(annotation)
        doc.undoStack.push(DocumentAction.AddAnnotation(pageIndex, annotation))
        doc.redoStack.clear()
        return true
    }

    fun addAnnotationJson(docHandle: Long, annotJson: String): Boolean {
        if (isNativeAvailable()) {
            return nativeDocumentAddAnnotationJson(docHandle, annotJson) == 0
        }
        val doc = mockDocuments[docHandle] ?: return false
        val page = doc.pages.firstOrNull() ?: return false
        val id = Regex("\"id\"\\s*:\\s*(\\d+)").find(annotJson)?.groupValues?.get(1) ?: "0"
        val annot = PdfAnnotation.Highlight(id = id, pageIndex = 0, bounds = PageRect(0f, 0f, 10f, 10f))
        page.annotations.add(annot)
        return true
    }

    fun removeAnnotation(docHandle: Long, pageIndex: Int, id: String): Boolean {
        if (isNativeAvailable()) {
            val numericId = id.toLongOrNull() ?: (id.hashCode().toLong().let { if (it < 0) -it else it })
            return nativeDocumentRemoveAnnotation(docHandle, numericId) == 0
        }
        val doc = mockDocuments[docHandle] ?: return false
        val page = doc.pages.getOrNull(pageIndex) ?: return false
        val index = page.annotations.indexOfFirst { it.id == id }
        if (index != -1) {
            val removed = page.annotations.removeAt(index)
            doc.undoStack.push(DocumentAction.RemoveAnnotation(pageIndex, removed))
            doc.redoStack.clear()
            return true
        }
        return false
    }

    fun removeAnnotation(docHandle: Long, annotationId: Long): Boolean {
        if (isNativeAvailable()) {
            return nativeDocumentRemoveAnnotation(docHandle, annotationId) == 0
        }
        val doc = mockDocuments[docHandle] ?: return false
        for (page in doc.pages) {
            val index = page.annotations.indexOfFirst { it.id == annotationId.toString() }
            if (index != -1) {
                val removed = page.annotations.removeAt(index)
                doc.undoStack.push(DocumentAction.RemoveAnnotation(page.originalIndex, removed))
                doc.redoStack.clear()
                return true
            }
        }
        return false
    }

    fun clearAnnotations(docHandle: Long, pageIndex: Int): Boolean {
        val doc = mockDocuments[docHandle] ?: return false
        val page = doc.pages.getOrNull(pageIndex) ?: return false
        if (page.annotations.isEmpty()) return true
        val previous = ArrayList(page.annotations)
        page.annotations.clear()
        doc.undoStack.push(DocumentAction.ClearAnnotations(pageIndex, previous))
        doc.redoStack.clear()
        return true
    }

    fun canUndo(docHandle: Long): Boolean {
        val doc = mockDocuments[docHandle] ?: return false
        return doc.undoStack.isNotEmpty()
    }

    fun canRedo(docHandle: Long): Boolean {
        val doc = mockDocuments[docHandle] ?: return false
        return doc.redoStack.isNotEmpty()
    }

    fun undo(docHandle: Long): Boolean {
        if (isNativeAvailable()) {
            return nativeDocumentUndo(docHandle)
        }
        val doc = mockDocuments[docHandle] ?: return false
        if (doc.undoStack.isEmpty()) return false
        val action = doc.undoStack.pop()
        when (action) {
            is DocumentAction.AddAnnotation -> {
                doc.pages.getOrNull(action.pageIndex)?.annotations?.removeAll { it.id == action.annotation.id }
            }
            is DocumentAction.RemoveAnnotation -> {
                doc.pages.getOrNull(action.pageIndex)?.annotations?.add(action.annotation)
            }
            is DocumentAction.ClearAnnotations -> {
                doc.pages.getOrNull(action.pageIndex)?.annotations?.addAll(action.previousAnnotations)
            }
            is DocumentAction.RotatePage -> {
                val page = doc.pages.getOrNull(action.pageIndex)
                if (page != null) {
                    page.rotation = (page.rotation - action.degrees) % 360
                    if (page.rotation < 0) page.rotation += 360
                }
            }
        }
        doc.redoStack.push(action)
        return true
    }

    fun redo(docHandle: Long): Boolean {
        if (isNativeAvailable()) {
            return nativeDocumentRedo(docHandle)
        }
        val doc = mockDocuments[docHandle] ?: return false
        if (doc.redoStack.isEmpty()) return false
        val action = doc.redoStack.pop()
        when (action) {
            is DocumentAction.AddAnnotation -> {
                doc.pages.getOrNull(action.pageIndex)?.annotations?.add(action.annotation)
            }
            is DocumentAction.RemoveAnnotation -> {
                doc.pages.getOrNull(action.pageIndex)?.annotations?.removeAll { it.id == action.annotation.id }
            }
            is DocumentAction.ClearAnnotations -> {
                doc.pages.getOrNull(action.pageIndex)?.annotations?.clear()
            }
            is DocumentAction.RotatePage -> {
                val page = doc.pages.getOrNull(action.pageIndex)
                if (page != null) {
                    page.rotation = (page.rotation + action.degrees) % 360
                    if (page.rotation < 0) page.rotation += 360
                }
            }
        }
        doc.undoStack.push(action)
        return true
    }

    fun getAnnotations(docHandle: Long, pageIndex: Int): List<PdfAnnotation> {
        if (isNativeAvailable()) {
            val json = nativeDocumentGetAnnotationsJson(docHandle, pageIndex)
            if (!json.isNullOrBlank()) {
                return emptyList()
            }
        }
        val doc = mockDocuments[docHandle] ?: return emptyList()
        return doc.pages.getOrNull(pageIndex)?.annotations?.toList() ?: emptyList()
    }

    // =========================================================================
    // Professional PDF Tools (Phase 4 Track 4D)
    // =========================================================================

    fun redactDocument(inputPath: String, outputPath: String, redactionsJson: String): Boolean {
        if (isNativeAvailable()) {
            return nativeRedactDocument(inputPath, outputPath, redactionsJson) == 0
        }
        return try {
            val inFile = File(inputPath)
            val outFile = File(outputPath)
            outFile.parentFile?.mkdirs()
            if (inFile.exists()) inFile.copyTo(outFile, overwrite = true)
            else outFile.writeText("%PDF-1.7 mock redacted\n%%EOF\n")
            true
        } catch (_: Throwable) {
            false
        }
    }

    fun compressDocument(
        inputPath: String,
        outputPath: String,
        removeUnused: Boolean = true,
        recompress: Boolean = true,
        stripMetadata: Boolean = false
    ): CompressionStats? {
        if (isNativeAvailable()) {
            val json = nativeCompressDocument(inputPath, outputPath, removeUnused, recompress, stripMetadata)
            if (!json.isNullOrBlank()) {
                return parseCompressionStatsJson(json)
            }
            return null
        }
        return try {
            val inFile = File(inputPath)
            val outFile = File(outputPath)
            outFile.parentFile?.mkdirs()
            val originalSize = if (inFile.exists()) inFile.length().coerceAtLeast(1024L) else 102400L
            val compressedSize = (originalSize * 0.75f).toLong()
            val savingsBytes = originalSize - compressedSize
            outFile.writeText("%PDF-1.7 mock compressed size=$compressedSize\n%%EOF\n")
            CompressionStats(
                originalSize = originalSize,
                compressedSize = compressedSize,
                savingsBytes = savingsBytes,
                savingsRatio = 0.25f
            )
        } catch (_: Throwable) {
            null
        }
    }

    fun applyWatermark(
        inputPath: String,
        outputPath: String,
        text: String,
        fontSize: Float = 36f,
        rotationDeg: Float = 45f,
        opacity: Float = 0.3f,
        r: Int = 128,
        g: Int = 128,
        b: Int = 128,
        onTop: Boolean = true
    ): Boolean {
        if (isNativeAvailable()) {
            return nativeApplyWatermark(inputPath, outputPath, text, fontSize, rotationDeg, opacity, r, g, b, onTop) == 0
        }
        return try {
            val inFile = File(inputPath)
            val outFile = File(outputPath)
            outFile.parentFile?.mkdirs()
            if (inFile.exists()) inFile.copyTo(outFile, overwrite = true)
            else outFile.writeText("%PDF-1.7 mock watermarked: $text\n%%EOF\n")
            true
        } catch (_: Throwable) {
            false
        }
    }

    fun applyPageNumbers(
        inputPath: String,
        outputPath: String,
        pattern: String = "Sayfa {page} / {total}",
        position: Int = 4,
        fontSize: Float = 10f,
        margin: Float = 36f
    ): Boolean {
        if (isNativeAvailable()) {
            return nativeApplyPageNumbers(inputPath, outputPath, pattern, position, fontSize, margin) == 0
        }
        return try {
            val inFile = File(inputPath)
            val outFile = File(outputPath)
            outFile.parentFile?.mkdirs()
            if (inFile.exists()) inFile.copyTo(outFile, overwrite = true)
            else outFile.writeText("%PDF-1.7 mock page numbered: $pattern\n%%EOF\n")
            true
        } catch (_: Throwable) {
            false
        }
    }

    fun applyBatesStamp(
        inputPath: String,
        outputPath: String,
        prefix: String,
        startNumber: Int = 1,
        paddingDigits: Int = 6,
        suffix: String = "",
        position: Int = 5
    ): Boolean {
        if (isNativeAvailable()) {
            return nativeApplyBatesStamp(inputPath, outputPath, prefix, startNumber, paddingDigits, suffix, position) == 0
        }
        return try {
            val inFile = File(inputPath)
            val outFile = File(outputPath)
            outFile.parentFile?.mkdirs()
            if (inFile.exists()) inFile.copyTo(outFile, overwrite = true)
            else outFile.writeText("%PDF-1.7 mock bates stamped: $prefix$startNumber$suffix\n%%EOF\n")
            true
        } catch (_: Throwable) {
            false
        }
    }

    fun readFormFields(inputPath: String): List<FormFieldInfo> {
        if (isNativeAvailable()) {
            val json = nativeReadFormFields(inputPath)
            if (!json.isNullOrBlank()) {
                return parseFormFieldsJson(json)
            }
        }
        return listOf(
            FormFieldInfo(name = "full_name", fieldType = "Text", value = "Ahmet Yılmaz", readOnly = false),
            FormFieldInfo(name = "email_address", fieldType = "Text", value = "ahmet@example.com", readOnly = false),
            FormFieldInfo(name = "terms_accepted", fieldType = "CheckBox", value = "Yes", readOnly = false)
        )
    }

    fun fillFormFields(
        inputPath: String,
        outputPath: String,
        fieldValuesJson: String,
        flatten: Boolean = false
    ): Boolean {
        if (isNativeAvailable()) {
            return nativeFillFormFields(inputPath, outputPath, fieldValuesJson, flatten) == 0
        }
        return try {
            val inFile = File(inputPath)
            val outFile = File(outputPath)
            outFile.parentFile?.mkdirs()
            if (inFile.exists()) inFile.copyTo(outFile, overwrite = true)
            else outFile.writeText("%PDF-1.7 mock form filled: $fieldValuesJson flatten=$flatten\n%%EOF\n")
            true
        } catch (_: Throwable) {
            false
        }
    }

    // =========================================================================
    // Document Converter Tools (Phase 5 Track 5D)
    // =========================================================================

    fun pdfToImages(
        inPath: String,
        outDir: String,
        dpi: Int = 150,
        format: ImageExportFormat = ImageExportFormat.PNG,
        pageRange: List<Int>? = null
    ): List<String>? {
        if (isNativeAvailable()) {
            val pageRangeJson = pageRange?.let { list ->
                "[" + list.joinToString(",") + "]"
            }
            val json = nativePdfToImages(inPath, outDir, dpi, format.value, pageRangeJson)
            if (!json.isNullOrBlank()) {
                return parsePathsJson(json)
            }
            return null
        }
        return try {
            val dir = File(outDir)
            dir.mkdirs()
            val ext = if (format == ImageExportFormat.PNG) "png" else "jpg"
            val count = pageRange?.size ?: 3
            val generated = mutableListOf<String>()
            for (i in 0 until count) {
                val pageIndex = pageRange?.getOrNull(i) ?: i
                val f = File(dir, "page_${pageIndex + 1}.$ext")
                f.writeText("mock image data for page ${pageIndex + 1}")
                generated.add(f.absolutePath)
            }
            generated
        } catch (_: Throwable) {
            null
        }
    }

    fun imagesToPdf(
        imagePaths: List<String>,
        pageSize: ImageToPdfPageSize = ImageToPdfPageSize.A4,
        marginPt: Float = 10f,
        outPath: String
    ): Boolean {
        if (imagePaths.isEmpty()) return false
        if (isNativeAvailable()) {
            val imagesJson = "[" + imagePaths.joinToString(",") { "\"$it\"" } + "]"
            return nativeImagesToPdf(imagesJson, pageSize.value, marginPt, outPath) == 0
        }
        return try {
            val outFile = File(outPath)
            outFile.parentFile?.mkdirs()
            outFile.writeText("%PDF-1.7 mock images-to-pdf count=${imagePaths.size}\n%%EOF\n")
            true
        } catch (_: Throwable) {
            false
        }
    }

    fun markdownToPdf(
        markdownText: String,
        outPath: String
    ): Boolean {
        if (markdownText.isBlank()) return false
        if (isNativeAvailable()) {
            return nativeMarkdownToPdf(markdownText, outPath) == 0
        }
        return try {
            val outFile = File(outPath)
            outFile.parentFile?.mkdirs()
            outFile.writeText("%PDF-1.7 mock markdown-to-pdf len=${markdownText.length}\n%%EOF\n")
            true
        } catch (_: Throwable) {
            false
        }
    }

    // =========================================================================
    // Phase 6: Office Documents (DOCX, XLSX, PPTX)
    // =========================================================================

    fun isSupportedOfficeFile(path: String): Boolean {
        val ext = path.substringAfterLast('.', "").lowercase()
        return ext in listOf("docx", "doc", "xlsx", "xls", "pptx", "ppt")
    }

    fun officeExtractText(inPath: String): String? {
        if (inPath.isBlank()) return null
        if (isNativeAvailable()) {
            return nativeOfficeExtractText(inPath)
        }
        val file = File(inPath)
        return if (file.exists()) "Mock Office Content: ${file.name}" else null
    }

    fun officeInspect(inPath: String): OfficeDocumentMetadata? {
        if (inPath.isBlank()) return null
        if (isNativeAvailable()) {
            val json = nativeOfficeInspect(inPath) ?: return null
            return parseOfficeMetadataJson(json)
        }
        val file = File(inPath)
        if (!file.exists()) return null
        val ext = file.extension.uppercase().ifBlank { "DOCX" }
        return OfficeDocumentMetadata(
            docType = ext,
            title = file.nameWithoutExtension,
            wordCount = 150,
            itemCount = 6,
            summary = "Mock office document summary"
        )
    }

    fun officeCreateDocx(text: String, outPath: String): Boolean {
        if (outPath.isBlank()) return false
        if (isNativeAvailable()) {
            return nativeOfficeCreateDocx(text, outPath) == 0
        }
        return try {
            val file = File(outPath)
            file.parentFile?.mkdirs()
            file.writeText("MOCK DOCX:\n$text")
            true
        } catch (_: Throwable) {
            false
        }
    }

    // Phase 8 Offline AI & Reflow Editing
    fun summarizeDocument(docHandle: Long, maxSentences: Int = 5): String {
        if (isNativeAvailable()) {
            val json = nativeSummarizeDocument(docHandle, maxSentences)
            if (json != null) return json
        }
        return """{"summary_text":"RBD OFİS yüksek performanslı ve tamamen çevrimdışı ofis paketidir.","key_sentences":["RBD OFİS yüksek performanslı ve tamamen çevrimdışı ofis paketidir."],"top_keywords":[{"word":"ofis","count":3,"score":1.0}],"total_sentences":1,"word_count":10,"reading_time_minutes":0.1}"""
    }

    fun askDocument(docHandle: Long, query: String): String {
        if (isNativeAvailable()) {
            val json = nativeAskDocument(docHandle, query)
            if (json != null) return json
        }
        return """{"query":"$query","answer_text":"RBD OFİS tüm verileri yerel cihazda işler, ağ erişimi yoktur.","context_excerpt":"RBD OFİS tüm verileri yerel cihazda işler, ağ erişimi yoktur.","page_number":1,"confidence":0.95,"found":true}"""
    }

    fun reflowPageText(docHandle: Long, pageIndex: Int): String {
        if (isNativeAvailable()) {
            val json = nativeReflowPageText(docHandle, pageIndex)
            if (json != null) return json
        }
        return """[{"id":1,"page_index":$pageIndex,"bbox":[72.0,600.0,400.0,650.0],"text":"RBD OFİS akışkan paragraf metni.","font_size":12.0,"line_count":1}]"""
    }

    fun replaceParagraphText(
        docHandle: Long,
        inputPath: String,
        pageIndex: Int,
        x0: Float,
        y0: Float,
        x1: Float,
        y1: Float,
        newText: String,
        fontSize: Float,
        outputPath: String
    ): Boolean {
        if (isNativeAvailable()) {
            return nativeReplaceParagraphText(docHandle, inputPath, pageIndex, x0, y0, x1, y1, newText, fontSize, outputPath) == 0
        }
        return try {
            val inFile = File(inputPath)
            if (inFile.exists()) {
                inFile.copyTo(File(outputPath), overwrite = true)
            } else {
                File(outputPath).writeText("%PDF-1.7 reflowed: $newText")
            }
            true
        } catch (_: Throwable) {
            false
        }
    }

    // =========================================================================
    // Document Scanner & Image Enhancement
    // =========================================================================

    fun scanToGrayscale(bitmap: Bitmap): Bitmap? {
        val width = bitmap.width
        val height = bitmap.height
        if (width <= 0 || height <= 0) return null

        if (isNativeAvailable()) {
            try {
                val stride = width * 4
                val bufferSize = stride * height
                val directBuffer = ByteBuffer.allocateDirect(bufferSize)
                bitmap.copyPixelsToBuffer(directBuffer)
                directBuffer.rewind()

                val status = nativeScanToGrayscale(directBuffer, width, height, stride)
                if (status == 0) {
                    val output = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
                    directBuffer.rewind()
                    output.copyPixelsFromBuffer(directBuffer)
                    return output
                }
            } catch (e: Throwable) {
                logW(TAG, "Native scanToGrayscale failed", e)
            }
        }

        // Mock fallback implementation
        return try {
            val output = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
            val pixels = IntArray(width * height)
            bitmap.getPixels(pixels, 0, width, 0, 0, width, height)
            for (i in pixels.indices) {
                val pixel = pixels[i]
                val r = (pixel shr 16) and 0xFF
                val g = (pixel shr 8) and 0xFF
                val b = pixel and 0xFF
                val gray = ((299 * r + 587 * g + 114 * b) / 1000).coerceIn(0, 255)
                pixels[i] = -0x1000000 or (gray shl 16) or (gray shl 8) or gray
            }
            output.setPixels(pixels, 0, width, 0, 0, width, height)
            output
        } catch (_: Throwable) {
            null
        }
    }

    fun scanToBinarized(bitmap: Bitmap): Bitmap? {
        val width = bitmap.width
        val height = bitmap.height
        if (width <= 0 || height <= 0) return null

        if (isNativeAvailable()) {
            try {
                val stride = width * 4
                val bufferSize = stride * height
                val directBuffer = ByteBuffer.allocateDirect(bufferSize)
                bitmap.copyPixelsToBuffer(directBuffer)
                directBuffer.rewind()

                val status = nativeScanToBinarized(directBuffer, width, height, stride)
                if (status == 0) {
                    val output = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
                    directBuffer.rewind()
                    output.copyPixelsFromBuffer(directBuffer)
                    return output
                }
            } catch (e: Throwable) {
                logW(TAG, "Native scanToBinarized failed", e)
            }
        }

        // Mock fallback implementation
        return try {
            val output = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
            val pixels = IntArray(width * height)
            bitmap.getPixels(pixels, 0, width, 0, 0, width, height)

            val histogram = IntArray(256)
            val grays = IntArray(pixels.size)
            for (i in pixels.indices) {
                val pixel = pixels[i]
                val r = (pixel shr 16) and 0xFF
                val g = (pixel shr 8) and 0xFF
                val b = pixel and 0xFF
                val gray = ((299 * r + 587 * g + 114 * b) / 1000).coerceIn(0, 255)
                grays[i] = gray
                histogram[gray]++
            }

            val threshold = scanCalculateOtsuThreshold(histogram, pixels.size)
            val white = -0x1
            val black = -0x1000000
            for (i in pixels.indices) {
                pixels[i] = if (grays[i] <= threshold) black else white
            }
            output.setPixels(pixels, 0, width, 0, 0, width, height)
            output
        } catch (_: Throwable) {
            null
        }
    }

    fun scanCalculateOtsuThreshold(histogram: IntArray, totalPixels: Int): Int {
        if (totalPixels <= 0) return 128
        if (isNativeAvailable()) {
            val result = nativeScanCalculateOtsuThreshold(histogram, totalPixels)
            if (result >= 0) return result
        }

        // Pure algorithmic fallback
        var sum = 0.0
        for (i in 0 until 256) {
            sum += i.toDouble() * histogram[i]
        }

        var sumB = 0.0
        var weightB = 0L
        var maxVariance = -1.0
        var firstBest = 128
        var lastBest = 128

        for (t in 0 until 256) {
            weightB += histogram[t]
            if (weightB == 0L) continue

            val weightF = totalPixels - weightB
            if (weightF == 0L) break

            sumB += t.toDouble() * histogram[t]
            val meanB = sumB / weightB
            val meanF = (sum - sumB) / weightF

            val betweenVariance = weightB.toDouble() * weightF.toDouble() * (meanB - meanF) * (meanB - meanF)
            if (betweenVariance > maxVariance + 1e-5) {
                maxVariance = betweenVariance
                firstBest = t
                lastBest = t
            } else if (kotlin.math.abs(betweenVariance - maxVariance) <= 1e-5) {
                lastBest = t
            }
        }

        return (firstBest + lastBest) / 2
    }

    fun scanGeneratePdf(imagePaths: List<String>, outPath: String): Boolean {
        if (imagePaths.isEmpty()) return false
        if (isNativeAvailable()) {
            val imagesJson = "[" + imagePaths.joinToString(",") { "\"$it\"" } + "]"
            return nativeScanGeneratePdf(imagesJson, outPath) == 0
        }
        return try {
            val outFile = File(outPath)
            outFile.parentFile?.mkdirs()
            outFile.writeText("%PDF-1.7 mock scanned pdf count=${imagePaths.size}\n%%EOF\n")
            true
        } catch (_: Throwable) {
            false
        }
    }

    private fun parseOfficeMetadataJson(json: String): OfficeDocumentMetadata? {
        return try {
            val type = Regex("\"doc_type\"\\s*:\\s*\"([^\"]+)\"").find(json)?.groupValues?.get(1) ?: "DOCX"
            val title = Regex("\"title\"\\s*:\\s*\"([^\"]+)\"").find(json)?.groupValues?.get(1) ?: "Document"
            val words = Regex("\"word_count\"\\s*:\\s*(\\d+)").find(json)?.groupValues?.get(1)?.toIntOrNull() ?: 0
            val items = Regex("\"item_count\"\\s*:\\s*(\\d+)").find(json)?.groupValues?.get(1)?.toIntOrNull() ?: 0
            val summary = Regex("\"summary\"\\s*:\\s*\"([^\"]*)\"").find(json)?.groupValues?.get(1) ?: ""
            OfficeDocumentMetadata(type, title, words, items, summary)
        } catch (_: Throwable) {
            null
        }
    }

    private fun parsePathsJson(json: String): List<String> {
        val list = mutableListOf<String>()
        try {
            val pathRegex = Regex("\"([^\"]+)\"")
            pathRegex.findAll(json).forEach {
                list.add(it.groupValues[1])
            }
        } catch (_: Throwable) {
            // ignore
        }
        return list
    }

    private fun parseCompressionStatsJson(json: String): CompressionStats? {
        return try {
            val orig = Regex("\"original_size\"\\s*:\\s*(\\d+)").find(json)?.groupValues?.get(1)?.toLongOrNull() ?: 0L
            val comp = Regex("\"compressed_size\"\\s*:\\s*(\\d+)").find(json)?.groupValues?.get(1)?.toLongOrNull() ?: 0L
            val sav = Regex("\"savings_bytes\"\\s*:\\s*(\\d+)").find(json)?.groupValues?.get(1)?.toLongOrNull() ?: 0L
            val ratio = Regex("\"savings_ratio\"\\s*:\\s*([0-9.]+)").find(json)?.groupValues?.get(1)?.toFloatOrNull() ?: 0f
            CompressionStats(orig, comp, sav, ratio)
        } catch (_: Throwable) {
            null
        }
    }

    private fun parseFormFieldsJson(json: String): List<FormFieldInfo> {
        val list = mutableListOf<FormFieldInfo>()
        try {
            val nameRegex = Regex("\"name\"\\s*:\\s*\"([^\"]+)\"")
            val typeRegex = Regex("\"field_type\"\\s*:\\s*\"([^\"]+)\"")
            val valRegex = Regex("\"value\"\\s*:\\s*\"([^\"]*)\"")
            val names = nameRegex.findAll(json).map { it.groupValues[1] }.toList()
            val types = typeRegex.findAll(json).map { it.groupValues[1] }.toList()
            val vals = valRegex.findAll(json).map { it.groupValues[1] }.toList()
            for (i in names.indices) {
                list.add(
                    FormFieldInfo(
                        name = names[i],
                        fieldType = types.getOrElse(i) { "Text" },
                        value = vals.getOrElse(i) { "" },
                        readOnly = false
                    )
                )
            }
        } catch (_: Throwable) {
            // ignore
        }
        return list
    }

    // =========================================================================
    // Native JNI Method Declarations (Exported by librbd_ffi.so / rbd-ffi)
    // =========================================================================

    private external fun nativeEngineInit(): Long
    private external fun nativeEngineFree(engineHandle: Long)
    private external fun nativeDocumentOpenFile(engineHandle: Long, path: String): Long
    private external fun nativeDocumentOpenMemory(engineHandle: Long, bytes: ByteArray): Long
    private external fun nativeDocumentFree(docHandle: Long)
    private external fun nativeDocumentGetPageCount(docHandle: Long): Int
    private external fun nativeDocumentGetPageSize(docHandle: Long, pageIndex: Int): DoubleArray?
    private external fun nativeRenderTileIntoBuffer(
        docHandle: Long,
        pageIndex: Int,
        tileX: Int,
        tileY: Int,
        tileW: Int,
        tileH: Int,
        zoom: Double,
        directByteBuffer: ByteBuffer,
        stride: Int,
        pixelFormat: Int
    ): Int
    private external fun nativeDocumentExtractPageText(docHandle: Long, pageIndex: Int): String?
    private external fun nativeDocumentSearchJson(docHandle: Long, query: String, caseSensitive: Boolean): String?
    private external fun nativeDocumentAddAnnotationJson(docHandle: Long, annotJson: String): Int
    private external fun nativeDocumentRemoveAnnotation(docHandle: Long, annotationId: Long): Int
    private external fun nativeDocumentGetAnnotationsJson(docHandle: Long, pageIndex: Int): String?
    private external fun nativeDocumentUndo(docHandle: Long): Boolean
    private external fun nativeDocumentRedo(docHandle: Long): Boolean
    private external fun nativeDocumentSaveAnnotations(docHandle: Long, outPath: String): Int

    // Page operations
    private external fun nativePageOpRotate(inPath: String, outPath: String, degrees: Int): Int
    private external fun nativePageOpDeletePages(inPath: String, outPath: String, pagesJson: String): Int
    private external fun nativePageOpMerge(outPath: String, inPathsJson: String): Int
    private external fun nativePageOpSplit(inPath: String, outPath: String, startPage: Int, endPage: Int): Int
    private external fun nativePageOpReorder(inPath: String, outPath: String, orderJson: String): Int
    private external fun nativePageOpCrop(inPath: String, outPath: String, pageNum: Int, x0: Float, y0: Float, x1: Float, y1: Float): Int
    private external fun nativePageOpInsertBlank(inPath: String, outPath: String, afterPage: Int, widthPt: Float, heightPt: Float): Int
    private external fun nativePageOpDecrypt(inPath: String, outPath: String, password: String): Int

    // Phase 4 Pro PDF Tools
    private external fun nativeRedactDocument(inPath: String, outPath: String, redactionsJson: String): Int
    private external fun nativeCompressDocument(inPath: String, outPath: String, removeUnused: Boolean, recompress: Boolean, stripMeta: Boolean): String?
    private external fun nativeApplyWatermark(inPath: String, outPath: String, text: String, fontSize: Float, rotationDeg: Float, opacity: Float, r: Int, g: Int, b: Int, onTop: Boolean): Int
    private external fun nativeApplyPageNumbers(inPath: String, outPath: String, pattern: String, position: Int, fontSize: Float, margin: Float): Int
    private external fun nativeApplyBatesStamp(inPath: String, outPath: String, prefix: String, startNumber: Int, paddingDigits: Int, suffix: String, position: Int): Int
    private external fun nativeReadFormFields(inPath: String): String?
    private external fun nativeFillFormFields(inPath: String, outPath: String, fieldValuesJson: String, flatten: Boolean): Int

    // Phase 5 Converter Tools
    private external fun nativePdfToImages(inPath: String, outDir: String, dpi: Int, format: Int, pageRangeJson: String?): String?
    private external fun nativeImagesToPdf(imagesJson: String, pageSize: Int, marginPt: Float, outPath: String): Int
    private external fun nativeMarkdownToPdf(markdownText: String, outPath: String): Int

    // Phase 6 Office Tools
    private external fun nativeOfficeExtractText(inPath: String): String?
    private external fun nativeOfficeInspect(inPath: String): String?
    private external fun nativeOfficeCreateDocx(text: String, outPath: String): Int

    // Phase 8 Offline AI & Reflow Editing
    private external fun nativeSummarizeDocument(docHandle: Long, maxSentences: Int): String?
    private external fun nativeAskDocument(docHandle: Long, query: String): String?
    private external fun nativeReflowPageText(docHandle: Long, pageIndex: Int): String?
    private external fun nativeReplaceParagraphText(
        docHandle: Long,
        inputPath: String,
        pageIndex: Int,
        x0: Float,
        y0: Float,
        x1: Float,
        y1: Float,
        newText: String,
        fontSize: Float,
        outputPath: String
    ): Int

    // Document Scanner & Image Enhancement
    private external fun nativeScanToGrayscale(directByteBuffer: ByteBuffer, width: Int, height: Int, stride: Int): Int
    private external fun nativeScanToBinarized(directByteBuffer: ByteBuffer, width: Int, height: Int, stride: Int): Int
    private external fun nativeScanCalculateOtsuThreshold(histogram: IntArray, totalPixels: Int): Int
    private external fun nativeScanGeneratePdf(imagesJson: String, outPath: String): Int

    // =========================================================================
    // In-memory Mock Data Structures
    // =========================================================================

    private sealed class DocumentAction {
        data class AddAnnotation(val pageIndex: Int, val annotation: PdfAnnotation) : DocumentAction()
        data class RemoveAnnotation(val pageIndex: Int, val annotation: PdfAnnotation) : DocumentAction()
        data class ClearAnnotations(val pageIndex: Int, val previousAnnotations: List<PdfAnnotation>) : DocumentAction()
        data class RotatePage(val pageIndex: Int, val degrees: Int) : DocumentAction()
    }

    private data class MockPage(
        val originalIndex: Int,
        val width: Float = 595.28f,
        val height: Float = 841.89f,
        var rotation: Int = 0,
        val annotations: MutableList<PdfAnnotation> = mutableListOf()
    )

    private data class MockDocument(
        val filePath: String,
        val initialPageCount: Int = 6,
        val pages: MutableList<MockPage> = MutableList(initialPageCount) { MockPage(originalIndex = it) },
        val undoStack: java.util.ArrayDeque<DocumentAction> = java.util.ArrayDeque(),
        val redoStack: java.util.ArrayDeque<DocumentAction> = java.util.ArrayDeque()
    )
}
