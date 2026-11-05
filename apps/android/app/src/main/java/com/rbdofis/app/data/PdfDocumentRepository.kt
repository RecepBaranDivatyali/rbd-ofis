package com.rbdofis.app.data

import android.graphics.Bitmap
import com.rbdofis.app.core.CompressionStats
import com.rbdofis.app.core.FormFieldInfo
import com.rbdofis.app.core.ImageExportFormat
import com.rbdofis.app.core.ImageToPdfPageSize
import com.rbdofis.app.core.PageDimensions
import com.rbdofis.app.core.PdfAnnotation
import com.rbdofis.app.core.RbdNativeBridge
import com.rbdofis.app.core.SearchResult
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.withContext
import java.io.File
import java.util.Collections

/**
 * High-level state of a currently open document.
 */
data class DocumentState(
    val isOpen: Boolean = false,
    val filePath: String = "",
    val fileName: String = "",
    val pageCount: Int = 0,
    val currentPageIndex: Int = 0,
    val isModified: Boolean = false,
    val isLoading: Boolean = false,
    val errorMessage: String? = null
)

/**
 * Metadata for a single page.
 */
data class PageItem(
    val index: Int,
    val dimensions: PageDimensions,
    val rotation: Int = 0
)

/**
 * Central repository managing PDF document lifecycle, caching, and manipulation.
 * Delegates low-level rendering and document mutations to [RbdNativeBridge].
 */
class PdfDocumentRepository(
    private val nativeBridge: RbdNativeBridge = RbdNativeBridge.getInstance(),
    private val ioDispatcher: CoroutineDispatcher = Dispatchers.IO
) {

    private val _documentState = MutableStateFlow(DocumentState())
    val documentState: StateFlow<DocumentState> = _documentState.asStateFlow()

    private var currentDocHandle: Long = 0L

    // In-memory tile/thumbnail cache: max 128 items, LRU order, fully platform independent
    private val maxCacheEntries = 128
    private val tileCache = Collections.synchronizedMap(
        object : LinkedHashMap<String, Bitmap>(64, 0.75f, true) {
            override fun removeEldestEntry(eldest: MutableMap.MutableEntry<String, Bitmap>?): Boolean {
                return size > maxCacheEntries
            }
        }
    )

    /**
     * Opens a PDF document from a local file path.
     */
    suspend fun openDocument(filePath: String): Result<Int> = withContext(ioDispatcher) {
        try {
            _documentState.value = _documentState.value.copy(isLoading = true, errorMessage = null)

            // Close existing document if one is open
            if (currentDocHandle != 0L) {
                nativeBridge.closeDocument(currentDocHandle)
                tileCache.clear()
                currentDocHandle = 0L
            }

            val handle = nativeBridge.openDocument(filePath)
            if (handle <= 0L) {
                _documentState.value = _documentState.value.copy(
                    isLoading = false,
                    errorMessage = "Failed to open document: $filePath"
                )
                return@withContext Result.failure(IllegalStateException("Invalid document handle returned"))
            }

            currentDocHandle = handle
            val pageCount = nativeBridge.getPageCount(handle)
            val fileName = File(filePath).name

            _documentState.value = DocumentState(
                isOpen = true,
                filePath = filePath,
                fileName = fileName,
                pageCount = pageCount,
                currentPageIndex = 0,
                isModified = false,
                isLoading = false
            )

            Result.success(pageCount)
        } catch (e: Exception) {
            _documentState.value = _documentState.value.copy(
                isLoading = false,
                errorMessage = e.localizedMessage
            )
            Result.failure(e)
        }
    }

    /**
     * Opens a PDF from in-memory byte array (e.g. copied from content URI).
     */
    suspend fun openDocumentBytes(bytes: ByteArray, displayName: String): Result<Int> = withContext(ioDispatcher) {
        try {
            _documentState.value = _documentState.value.copy(isLoading = true, errorMessage = null)

            if (currentDocHandle != 0L) {
                nativeBridge.closeDocument(currentDocHandle)
                tileCache.clear()
                currentDocHandle = 0L
            }

            val handle = nativeBridge.openDocumentFromBytes(bytes)
            if (handle <= 0L) {
                _documentState.value = _documentState.value.copy(
                    isLoading = false,
                    errorMessage = "Failed to open document from stream"
                )
                return@withContext Result.failure(IllegalStateException("Invalid document handle returned"))
            }

            currentDocHandle = handle
            val pageCount = nativeBridge.getPageCount(handle)

            _documentState.value = DocumentState(
                isOpen = true,
                filePath = "",
                fileName = displayName,
                pageCount = pageCount,
                currentPageIndex = 0,
                isModified = false,
                isLoading = false
            )

            Result.success(pageCount)
        } catch (e: Exception) {
            _documentState.value = _documentState.value.copy(
                isLoading = false,
                errorMessage = e.localizedMessage
            )
            Result.failure(e)
        }
    }

    /**
     * Closes the active document and clears all memory caches.
     */
    suspend fun closeDocument() = withContext(ioDispatcher) {
        if (currentDocHandle != 0L) {
            nativeBridge.closeDocument(currentDocHandle)
            currentDocHandle = 0L
        }
        tileCache.clear()
        _documentState.value = DocumentState()
    }

    /**
     * Gets the dimensions of the specified page.
     */
    suspend fun getPageDimensions(pageIndex: Int): PageDimensions = withContext(ioDispatcher) {
        if (currentDocHandle == 0L) return@withContext PageDimensions(595.28f, 841.89f)
        nativeBridge.getPageSize(currentDocHandle, pageIndex)
    }

    /**
     * Retrieves all pages metadata for organizer view.
     */
    suspend fun getAllPages(): List<PageItem> = withContext(ioDispatcher) {
        if (currentDocHandle == 0L) return@withContext emptyList()
        val count = nativeBridge.getPageCount(currentDocHandle)
        (0 until count).map { index ->
            PageItem(
                index = index,
                dimensions = nativeBridge.getPageSize(currentDocHandle, index)
            )
        }
    }

    /**
     * Renders a page tile or full-page thumbnail.
     */
    suspend fun renderPage(
        pageIndex: Int,
        targetWidth: Int,
        targetHeight: Int,
        scale: Float = 1.0f
    ): Bitmap? = withContext(ioDispatcher) {
        if (currentDocHandle == 0L) return@withContext null

        val cacheKey = "p_${pageIndex}_w_${targetWidth}_h_${targetHeight}_s_${scale}"
        tileCache.get(cacheKey)?.let { return@withContext it }

        val bitmap = nativeBridge.renderTile(
            docHandle = currentDocHandle,
            pageIndex = pageIndex,
            tileX = 0,
            tileY = 0,
            tileWidth = targetWidth,
            tileHeight = targetHeight,
            scale = scale
        )

        if (bitmap != null) {
            tileCache.put(cacheKey, bitmap)
        }
        bitmap
    }

    /**
     * Rotates a page by the given degrees (typically 90 or -90).
     */
    suspend fun rotatePage(pageIndex: Int, degrees: Int): Boolean = withContext(ioDispatcher) {
        if (currentDocHandle == 0L) return@withContext false
        val success = nativeBridge.rotatePage(currentDocHandle, pageIndex, degrees)
        if (success) {
            clearPageCache(pageIndex)
            _documentState.value = _documentState.value.copy(isModified = true)
        }
        success
    }

    /**
     * Deletes a page from the document.
     */
    suspend fun deletePage(pageIndex: Int): Boolean = withContext(ioDispatcher) {
        if (currentDocHandle == 0L) return@withContext false
        val success = nativeBridge.deletePage(currentDocHandle, pageIndex)
        if (success) {
            tileCache.clear()
            val newCount = nativeBridge.getPageCount(currentDocHandle)
            val nextIndex = _documentState.value.currentPageIndex.coerceAtMost((newCount - 1).coerceAtLeast(0))
            _documentState.value = _documentState.value.copy(
                pageCount = newCount,
                currentPageIndex = nextIndex,
                isModified = true
            )
        }
        success
    }

    /**
     * Reorders a page from one position to another.
     */
    suspend fun reorderPage(fromIndex: Int, toIndex: Int): Boolean = withContext(ioDispatcher) {
        if (currentDocHandle == 0L) return@withContext false
        val success = nativeBridge.reorderPage(currentDocHandle, fromIndex, toIndex)
        if (success) {
            tileCache.clear()
            _documentState.value = _documentState.value.copy(isModified = true)
        }
        success
    }

    /**
     * Saves the modified document to the target file path.
     */
    suspend fun saveDocument(targetPath: String): Boolean = withContext(ioDispatcher) {
        if (currentDocHandle == 0L) return@withContext false
        val success = nativeBridge.saveDocument(currentDocHandle, targetPath)
        if (success) {
            _documentState.value = _documentState.value.copy(isModified = false, filePath = targetPath)
        }
        success
    }

    /**
     * Performs full-text search across the document.
     */
    suspend fun searchText(query: String): List<SearchResult> = withContext(ioDispatcher) {
        if (currentDocHandle == 0L || query.isBlank()) return@withContext emptyList()
        nativeBridge.searchText(currentDocHandle, query)
    }

    /**
     * Adds an annotation to the specified page.
     */
    suspend fun addAnnotation(pageIndex: Int, annotation: PdfAnnotation): Boolean = withContext(ioDispatcher) {
        if (currentDocHandle == 0L) return@withContext false
        val success = nativeBridge.addAnnotation(currentDocHandle, pageIndex, annotation)
        if (success) {
            clearPageCache(pageIndex)
            _documentState.value = _documentState.value.copy(isModified = true)
        }
        success
    }

    /**
     * Removes an annotation by ID.
     */
    suspend fun removeAnnotation(pageIndex: Int, annotationId: String): Boolean = withContext(ioDispatcher) {
        if (currentDocHandle == 0L) return@withContext false
        val success = nativeBridge.removeAnnotation(currentDocHandle, pageIndex, annotationId)
        if (success) {
            clearPageCache(pageIndex)
            _documentState.value = _documentState.value.copy(isModified = true)
        }
        success
    }

    /**
     * Clears all annotations from the specified page.
     */
    suspend fun clearAnnotations(pageIndex: Int): Boolean = withContext(ioDispatcher) {
        if (currentDocHandle == 0L) return@withContext false
        val success = nativeBridge.clearAnnotations(currentDocHandle, pageIndex)
        if (success) {
            clearPageCache(pageIndex)
            _documentState.value = _documentState.value.copy(isModified = true)
        }
        success
    }

    /**
     * Gets all annotations for the given page.
     */
    suspend fun getAnnotations(pageIndex: Int): List<PdfAnnotation> = withContext(ioDispatcher) {
        if (currentDocHandle == 0L) return@withContext emptyList()
        nativeBridge.getAnnotations(currentDocHandle, pageIndex)
    }

    /**
     * Undoes the last annotation or page operation.
     */
    suspend fun undo(): Boolean = withContext(ioDispatcher) {
        if (currentDocHandle == 0L) return@withContext false
        val success = nativeBridge.undo(currentDocHandle)
        if (success) {
            tileCache.clear()
            _documentState.value = _documentState.value.copy(isModified = true)
        }
        success
    }

    /**
     * Redoes the previously undone operation.
     */
    suspend fun redo(): Boolean = withContext(ioDispatcher) {
        if (currentDocHandle == 0L) return@withContext false
        val success = nativeBridge.redo(currentDocHandle)
        if (success) {
            tileCache.clear()
            _documentState.value = _documentState.value.copy(isModified = true)
        }
        success
    }

    /**
     * Returns true if an undo action is available.
     */
    fun canUndo(): Boolean {
        if (currentDocHandle == 0L) return false
        return nativeBridge.canUndo(currentDocHandle)
    }

    /**
     * Returns true if a redo action is available.
     */
    fun canRedo(): Boolean {
        if (currentDocHandle == 0L) return false
        return nativeBridge.canRedo(currentDocHandle)
    }

    /**
     * Sets the active page index in the viewer.
     */
    fun setCurrentPageIndex(index: Int) {
        if (index in 0 until _documentState.value.pageCount) {
            _documentState.value = _documentState.value.copy(currentPageIndex = index)
        }
    }

    // =========================================================================
    // Pro PDF Tools (Phase 4 Track 4D)
    // =========================================================================

    suspend fun compressDocument(
        outputPath: String,
        removeUnused: Boolean = true,
        recompress: Boolean = true,
        stripMetadata: Boolean = false
    ): Result<CompressionStats> = withContext(ioDispatcher) {
        val inputPath = _documentState.value.filePath
        if (inputPath.isBlank()) return@withContext Result.failure(IllegalStateException("No document currently open"))
        val stats = nativeBridge.compressDocument(inputPath, outputPath, removeUnused, recompress, stripMetadata)
        if (stats != null) Result.success(stats)
        else Result.failure(IllegalStateException("Document compression failed"))
    }

    suspend fun applyWatermark(
        outputPath: String,
        text: String,
        fontSize: Float = 36f,
        rotationDeg: Float = 45f,
        opacity: Float = 0.3f,
        r: Int = 128,
        g: Int = 128,
        b: Int = 128,
        onTop: Boolean = true
    ): Result<Unit> = withContext(ioDispatcher) {
        val inputPath = _documentState.value.filePath
        if (inputPath.isBlank()) return@withContext Result.failure(IllegalStateException("No document currently open"))
        val success = nativeBridge.applyWatermark(inputPath, outputPath, text, fontSize, rotationDeg, opacity, r, g, b, onTop)
        if (success) Result.success(Unit)
        else Result.failure(IllegalStateException("Failed to apply watermark"))
    }

    suspend fun applyPageNumbers(
        outputPath: String,
        pattern: String = "Sayfa {page} / {total}",
        position: Int = 4,
        fontSize: Float = 10f,
        margin: Float = 36f
    ): Result<Unit> = withContext(ioDispatcher) {
        val inputPath = _documentState.value.filePath
        if (inputPath.isBlank()) return@withContext Result.failure(IllegalStateException("No document currently open"))
        val success = nativeBridge.applyPageNumbers(inputPath, outputPath, pattern, position, fontSize, margin)
        if (success) Result.success(Unit)
        else Result.failure(IllegalStateException("Failed to apply page numbers"))
    }

    suspend fun applyBatesStamp(
        outputPath: String,
        prefix: String,
        startNumber: Int = 1,
        paddingDigits: Int = 6,
        suffix: String = "",
        position: Int = 5
    ): Result<Unit> = withContext(ioDispatcher) {
        val inputPath = _documentState.value.filePath
        if (inputPath.isBlank()) return@withContext Result.failure(IllegalStateException("No document currently open"))
        val success = nativeBridge.applyBatesStamp(inputPath, outputPath, prefix, startNumber, paddingDigits, suffix, position)
        if (success) Result.success(Unit)
        else Result.failure(IllegalStateException("Failed to apply Bates stamp"))
    }

    suspend fun redactDocument(
        outputPath: String,
        redactionsJson: String
    ): Result<Unit> = withContext(ioDispatcher) {
        val inputPath = _documentState.value.filePath
        if (inputPath.isBlank()) return@withContext Result.failure(IllegalStateException("No document currently open"))
        val success = nativeBridge.redactDocument(inputPath, outputPath, redactionsJson)
        if (success) Result.success(Unit)
        else Result.failure(IllegalStateException("Permanent redaction failed"))
    }

    suspend fun readFormFields(): Result<List<FormFieldInfo>> = withContext(ioDispatcher) {
        val inputPath = _documentState.value.filePath
        if (inputPath.isBlank()) return@withContext Result.failure(IllegalStateException("No document currently open"))
        val fields = nativeBridge.readFormFields(inputPath)
        Result.success(fields)
    }

    suspend fun fillFormFields(
        outputPath: String,
        fieldValuesJson: String,
        flatten: Boolean = false
    ): Result<Unit> = withContext(ioDispatcher) {
        val inputPath = _documentState.value.filePath
        if (inputPath.isBlank()) return@withContext Result.failure(IllegalStateException("No document currently open"))
        val success = nativeBridge.fillFormFields(inputPath, outputPath, fieldValuesJson, flatten)
        if (success) Result.success(Unit)
        else Result.failure(IllegalStateException("Failed to fill form fields"))
    }

    // =========================================================================
    // Document Converter Tools (Phase 5 Track 5D)
    // =========================================================================

    suspend fun convertPdfToImages(
        outDir: String,
        dpi: Int = 150,
        format: ImageExportFormat = ImageExportFormat.PNG,
        pageRange: List<Int>? = null
    ): Result<List<String>> = withContext(ioDispatcher) {
        val inputPath = _documentState.value.filePath
        if (inputPath.isBlank()) return@withContext Result.failure(IllegalStateException("No document currently open"))
        val paths = nativeBridge.pdfToImages(inputPath, outDir, dpi, format, pageRange)
        if (paths != null) Result.success(paths)
        else Result.failure(IllegalStateException("Failed to convert PDF pages to images"))
    }

    suspend fun convertImagesToPdf(
        imagePaths: List<String>,
        pageSize: ImageToPdfPageSize = ImageToPdfPageSize.A4,
        marginPt: Float = 10f,
        outPath: String
    ): Result<Unit> = withContext(ioDispatcher) {
        val success = nativeBridge.imagesToPdf(imagePaths, pageSize, marginPt, outPath)
        if (success) Result.success(Unit)
        else Result.failure(IllegalStateException("Failed to convert images to PDF"))
    }

    suspend fun convertMarkdownToPdf(
        markdownText: String,
        outPath: String
    ): Result<Unit> = withContext(ioDispatcher) {
        val success = nativeBridge.markdownToPdf(markdownText, outPath)
        if (success) Result.success(Unit)
        else Result.failure(IllegalStateException("Failed to convert markdown to PDF"))
    }

    private fun clearPageCache(pageIndex: Int) {
        synchronized(tileCache) {
            val prefix = "p_${pageIndex}_"
            val keysToRemove = tileCache.keys.filter { it.startsWith(prefix) }
            for (key in keysToRemove) {
                tileCache.remove(key)
            }
        }
    }
}
