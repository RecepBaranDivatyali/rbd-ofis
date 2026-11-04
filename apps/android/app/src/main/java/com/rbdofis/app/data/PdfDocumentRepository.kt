package com.rbdofis.app.data

import android.graphics.Bitmap
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
     * Sets the active page index in the viewer.
     */
    fun setCurrentPageIndex(index: Int) {
        if (index in 0 until _documentState.value.pageCount) {
            _documentState.value = _documentState.value.copy(currentPageIndex = index)
        }
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
