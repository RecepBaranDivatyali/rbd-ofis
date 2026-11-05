package com.rbdofis.app.ui.viewer

import android.content.Context
import android.graphics.Bitmap
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.rbdofis.app.core.CompressionStats
import com.rbdofis.app.core.FormFieldInfo
import com.rbdofis.app.core.ImageExportFormat
import com.rbdofis.app.core.ImageToPdfPageSize
import com.rbdofis.app.core.PdfAnnotation
import com.rbdofis.app.core.SearchResult
import com.rbdofis.app.data.DocumentState
import com.rbdofis.app.data.PdfDocumentRepository
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import java.io.File

enum class ProToolTab {
    COMPRESS,
    WATERMARK,
    PAGE_NUMBERS,
    BATES_STAMP,
    REDACT,
    FORM_FIELDS
}

enum class ConverterTab {
    PDF_TO_IMAGES,
    IMAGES_TO_PDF,
    MARKDOWN_TO_PDF
}

data class ViewerUiState(
    val documentState: DocumentState = DocumentState(),
    val currentBitmap: Bitmap? = null,
    val zoomScale: Float = 1.0f,
    val panOffset: Offset = Offset.Zero,
    val isSearchOpen: Boolean = false,
    val searchQuery: String = "",
    val searchResults: List<SearchResult> = emptyList(),
    val isRendering: Boolean = false,

    // Ink & Stylus State (Track 3C)
    val isInkModeActive: Boolean = false,
    val activeInkTool: InkTool = InkTool.PEN,
    val activeInkColor: Color = InkPalette.Black,
    val strokeWidthPx: Float = 3.0f,
    val annotations: List<PdfAnnotation> = emptyList(),
    val canUndo: Boolean = false,
    val canRedo: Boolean = false,

    // Pro PDF Tools State (Phase 4 Track 4D)
    val isProToolsOpen: Boolean = false,
    val activeProTool: ProToolTab = ProToolTab.COMPRESS,
    val compressionStats: CompressionStats? = null,
    val formFields: List<FormFieldInfo> = emptyList(),
    val isProcessingProTool: Boolean = false,
    val proToolMessage: String? = null,

    // Document Converter State (Phase 5 Track 5D)
    val isConverterOpen: Boolean = false,
    val activeConverterTab: ConverterTab = ConverterTab.PDF_TO_IMAGES,
    val convertedImagePaths: List<String> = emptyList(),
    val isProcessingConverter: Boolean = false,
    val converterMessage: String? = null
)

class PdfViewerViewModel(
    private val repository: PdfDocumentRepository
) : ViewModel() {

    private val _uiState = MutableStateFlow(ViewerUiState())
    val uiState: StateFlow<ViewerUiState> = _uiState.asStateFlow()

    init {
        viewModelScope.launch {
            repository.documentState.collect { docState ->
                _uiState.value = _uiState.value.copy(
                    documentState = docState,
                    canUndo = repository.canUndo(),
                    canRedo = repository.canRedo()
                )
                if (docState.isOpen) {
                    renderCurrentPage()
                    loadAnnotationsForCurrentPage()
                }
            }
        }
    }

    fun onTransform(centroid: Offset, pan: Offset, zoom: Float) {
        // If in drawing mode, do not pan/zoom with touches
        if (_uiState.value.isInkModeActive && _uiState.value.activeInkTool != InkTool.PAN) {
            return
        }

        val currentScale = _uiState.value.zoomScale
        val newScale = (currentScale * zoom).coerceIn(0.5f, 4.0f)
        val currentPan = _uiState.value.panOffset
        val newPan = if (newScale > 1.0f) currentPan + pan else Offset.Zero

        _uiState.value = _uiState.value.copy(
            zoomScale = newScale,
            panOffset = newPan
        )
    }

    fun zoomIn() {
        val newScale = (_uiState.value.zoomScale * 1.25f).coerceAtMost(4.0f)
        _uiState.value = _uiState.value.copy(zoomScale = newScale)
    }

    fun zoomOut() {
        val newScale = (_uiState.value.zoomScale / 1.25f).coerceAtLeast(0.5f)
        val newPan = if (newScale <= 1.0f) Offset.Zero else _uiState.value.panOffset
        _uiState.value = _uiState.value.copy(zoomScale = newScale, panOffset = newPan)
    }

    fun fitToWidth() {
        _uiState.value = _uiState.value.copy(zoomScale = 1.0f, panOffset = Offset.Zero)
    }

    fun nextPage() {
        val doc = _uiState.value.documentState
        if (doc.currentPageIndex < doc.pageCount - 1) {
            goToPage(doc.currentPageIndex + 1)
        }
    }

    fun previousPage() {
        val doc = _uiState.value.documentState
        if (doc.currentPageIndex > 0) {
            goToPage(doc.currentPageIndex - 1)
        }
    }

    fun goToPage(pageIndex: Int) {
        val doc = _uiState.value.documentState
        if (pageIndex in 0 until doc.pageCount) {
            repository.setCurrentPageIndex(pageIndex)
            _uiState.value = _uiState.value.copy(
                zoomScale = 1.0f,
                panOffset = Offset.Zero
            )
            renderCurrentPage()
            loadAnnotationsForCurrentPage()
        }
    }

    fun toggleSearch(open: Boolean) {
        _uiState.value = _uiState.value.copy(
            isSearchOpen = open,
            searchQuery = if (!open) "" else _uiState.value.searchQuery,
            searchResults = if (!open) emptyList() else _uiState.value.searchResults
        )
    }

    fun onSearchQueryChanged(query: String) {
        _uiState.value = _uiState.value.copy(searchQuery = query)
        if (query.length >= 2) {
            viewModelScope.launch {
                val results = repository.searchText(query)
                _uiState.value = _uiState.value.copy(searchResults = results)
            }
        } else {
            _uiState.value = _uiState.value.copy(searchResults = emptyList())
        }
    }

    fun renderCurrentPage(targetWidth: Int = 1080, targetHeight: Int = 1528) {
        val doc = _uiState.value.documentState
        if (!doc.isOpen || doc.pageCount == 0) return

        viewModelScope.launch {
            _uiState.value = _uiState.value.copy(isRendering = true)
            val bitmap = repository.renderPage(
                pageIndex = doc.currentPageIndex,
                targetWidth = targetWidth,
                targetHeight = targetHeight,
                scale = 1.0f
            )
            _uiState.value = _uiState.value.copy(
                currentBitmap = bitmap,
                isRendering = false
            )
        }
    }

    // =========================================================================
    // Ink & Stylus Actions (Track 3C)
    // =========================================================================

    fun toggleInkMode(enabled: Boolean? = null) {
        val newMode = enabled ?: !_uiState.value.isInkModeActive
        _uiState.value = _uiState.value.copy(isInkModeActive = newMode)
    }

    fun selectInkTool(tool: InkTool) {
        _uiState.value = _uiState.value.copy(activeInkTool = tool)
    }

    fun selectInkColor(color: Color) {
        _uiState.value = _uiState.value.copy(activeInkColor = color)
    }

    fun selectStrokeWidth(widthPx: Float) {
        _uiState.value = _uiState.value.copy(strokeWidthPx = widthPx)
    }

    fun addInkStroke(stroke: PdfAnnotation.InkStroke) {
        val doc = _uiState.value.documentState
        if (!doc.isOpen) return

        viewModelScope.launch {
            val success = repository.addAnnotation(doc.currentPageIndex, stroke)
            if (success) {
                loadAnnotationsForCurrentPage()
            }
        }
    }

    fun removeAnnotation(annotationId: String) {
        val doc = _uiState.value.documentState
        if (!doc.isOpen) return

        viewModelScope.launch {
            val success = repository.removeAnnotation(doc.currentPageIndex, annotationId)
            if (success) {
                loadAnnotationsForCurrentPage()
            }
        }
    }

    fun clearAllAnnotationsOnPage() {
        val doc = _uiState.value.documentState
        if (!doc.isOpen) return

        viewModelScope.launch {
            val success = repository.clearAnnotations(doc.currentPageIndex)
            if (success) {
                loadAnnotationsForCurrentPage()
            }
        }
    }

    fun undo() {
        viewModelScope.launch {
            val success = repository.undo()
            if (success) {
                loadAnnotationsForCurrentPage()
            }
        }
    }

    fun redo() {
        viewModelScope.launch {
            val success = repository.redo()
            if (success) {
                loadAnnotationsForCurrentPage()
            }
        }
    }

    private fun loadAnnotationsForCurrentPage() {
        val doc = _uiState.value.documentState
        if (!doc.isOpen) return

        viewModelScope.launch {
            val list = repository.getAnnotations(doc.currentPageIndex)
            _uiState.value = _uiState.value.copy(
                annotations = list,
                canUndo = repository.canUndo(),
                canRedo = repository.canRedo()
            )
        }
    }

    // =========================================================================
    // Pro PDF Tools (Phase 4 Track 4D)
    // =========================================================================

    private fun createProOutputPath(context: Context, suffix: String): String {
        val dir = File(context.cacheDir, "pro_tools")
        dir.mkdirs()
        val originalName = _uiState.value.documentState.fileName.substringBeforeLast(".pdf").ifBlank { "document" }
        val timestamp = System.currentTimeMillis()
        return File(dir, "${originalName}_${suffix}_$timestamp.pdf").absolutePath
    }

    fun openProTools(tab: ProToolTab = ProToolTab.COMPRESS) {
        _uiState.value = _uiState.value.copy(
            isProToolsOpen = true,
            activeProTool = tab,
            proToolMessage = null
        )
        if (tab == ProToolTab.FORM_FIELDS) {
            loadFormFields()
        }
    }

    fun closeProTools() {
        _uiState.value = _uiState.value.copy(isProToolsOpen = false, proToolMessage = null)
    }

    fun selectProToolTab(tab: ProToolTab) {
        _uiState.value = _uiState.value.copy(activeProTool = tab, proToolMessage = null)
        if (tab == ProToolTab.FORM_FIELDS) {
            loadFormFields()
        }
    }

    fun compressDocument(context: Context, removeUnused: Boolean, recompress: Boolean, stripMetadata: Boolean) {
        viewModelScope.launch {
            _uiState.value = _uiState.value.copy(isProcessingProTool = true, proToolMessage = null)
            val outPath = createProOutputPath(context, "compressed")
            val res = repository.compressDocument(outPath, removeUnused, recompress, stripMetadata)
            res.fold(
                onSuccess = { stats ->
                    _uiState.value = _uiState.value.copy(
                        isProcessingProTool = false,
                        compressionStats = stats,
                        proToolMessage = "success"
                    )
                    repository.openDocument(outPath)
                },
                onFailure = { err ->
                    _uiState.value = _uiState.value.copy(
                        isProcessingProTool = false,
                        proToolMessage = err.message ?: "Failed"
                    )
                }
            )
        }
    }

    fun applyWatermark(
        context: Context,
        text: String,
        fontSize: Float,
        rotationDeg: Float,
        opacity: Float,
        r: Int,
        g: Int,
        b: Int,
        onTop: Boolean
    ) {
        viewModelScope.launch {
            _uiState.value = _uiState.value.copy(isProcessingProTool = true, proToolMessage = null)
            val outPath = createProOutputPath(context, "watermark")
            val res = repository.applyWatermark(outPath, text, fontSize, rotationDeg, opacity, r, g, b, onTop)
            res.fold(
                onSuccess = {
                    _uiState.value = _uiState.value.copy(isProcessingProTool = false, proToolMessage = "success")
                    repository.openDocument(outPath)
                },
                onFailure = { err ->
                    _uiState.value = _uiState.value.copy(isProcessingProTool = false, proToolMessage = err.message ?: "Failed")
                }
            )
        }
    }

    fun applyPageNumbers(
        context: Context,
        pattern: String,
        position: Int,
        fontSize: Float,
        margin: Float
    ) {
        viewModelScope.launch {
            _uiState.value = _uiState.value.copy(isProcessingProTool = true, proToolMessage = null)
            val outPath = createProOutputPath(context, "numbered")
            val res = repository.applyPageNumbers(outPath, pattern, position, fontSize, margin)
            res.fold(
                onSuccess = {
                    _uiState.value = _uiState.value.copy(isProcessingProTool = false, proToolMessage = "success")
                    repository.openDocument(outPath)
                },
                onFailure = { err ->
                    _uiState.value = _uiState.value.copy(isProcessingProTool = false, proToolMessage = err.message ?: "Failed")
                }
            )
        }
    }

    fun applyBatesStamp(
        context: Context,
        prefix: String,
        startNumber: Int,
        paddingDigits: Int,
        suffix: String,
        position: Int
    ) {
        viewModelScope.launch {
            _uiState.value = _uiState.value.copy(isProcessingProTool = true, proToolMessage = null)
            val outPath = createProOutputPath(context, "bates")
            val res = repository.applyBatesStamp(outPath, prefix, startNumber, paddingDigits, suffix, position)
            res.fold(
                onSuccess = {
                    _uiState.value = _uiState.value.copy(isProcessingProTool = false, proToolMessage = "success")
                    repository.openDocument(outPath)
                },
                onFailure = { err ->
                    _uiState.value = _uiState.value.copy(isProcessingProTool = false, proToolMessage = err.message ?: "Failed")
                }
            )
        }
    }

    fun applyRedaction(context: Context, redactionsJson: String) {
        viewModelScope.launch {
            _uiState.value = _uiState.value.copy(isProcessingProTool = true, proToolMessage = null)
            val outPath = createProOutputPath(context, "redacted")
            val res = repository.redactDocument(outPath, redactionsJson)
            res.fold(
                onSuccess = {
                    _uiState.value = _uiState.value.copy(isProcessingProTool = false, proToolMessage = "success")
                    repository.openDocument(outPath)
                },
                onFailure = { err ->
                    _uiState.value = _uiState.value.copy(isProcessingProTool = false, proToolMessage = err.message ?: "Failed")
                }
            )
        }
    }

    fun loadFormFields() {
        viewModelScope.launch {
            val res = repository.readFormFields()
            res.fold(
                onSuccess = { fields ->
                    _uiState.value = _uiState.value.copy(formFields = fields)
                },
                onFailure = {
                    _uiState.value = _uiState.value.copy(formFields = emptyList())
                }
            )
        }
    }

    fun fillFormFields(context: Context, fieldValuesJson: String, flatten: Boolean) {
        viewModelScope.launch {
            _uiState.value = _uiState.value.copy(isProcessingProTool = true, proToolMessage = null)
            val outPath = createProOutputPath(context, "form_filled")
            val res = repository.fillFormFields(outPath, fieldValuesJson, flatten)
            res.fold(
                onSuccess = {
                    _uiState.value = _uiState.value.copy(isProcessingProTool = false, proToolMessage = "success")
                    repository.openDocument(outPath)
                },
                onFailure = { err ->
                    _uiState.value = _uiState.value.copy(isProcessingProTool = false, proToolMessage = err.message ?: "Failed")
                }
            )
        }
    }

    // =========================================================================
    // Document Converter Actions (Phase 5 Track 5D)
    // =========================================================================

    private fun createConverterOutputDir(context: Context, prefix: String): File {
        val dir = File(context.cacheDir, "${prefix}_${System.currentTimeMillis()}")
        dir.mkdirs()
        return dir
    }

    private fun createConverterOutputPath(context: Context, prefix: String): String {
        val dir = File(context.cacheDir, "conversions")
        dir.mkdirs()
        return File(dir, "${prefix}_${System.currentTimeMillis()}.pdf").absolutePath
    }

    fun openConverter(tab: ConverterTab = ConverterTab.PDF_TO_IMAGES) {
        _uiState.value = _uiState.value.copy(
            isConverterOpen = true,
            activeConverterTab = tab,
            converterMessage = null
        )
    }

    fun closeConverter() {
        _uiState.value = _uiState.value.copy(isConverterOpen = false, converterMessage = null)
    }

    fun selectConverterTab(tab: ConverterTab) {
        _uiState.value = _uiState.value.copy(activeConverterTab = tab, converterMessage = null)
    }

    fun convertPdfToImages(
        context: Context,
        dpi: Int = 150,
        format: ImageExportFormat = ImageExportFormat.PNG,
        pageRange: List<Int>? = null
    ) {
        viewModelScope.launch {
            _uiState.value = _uiState.value.copy(isProcessingConverter = true, converterMessage = null)
            val outDir = createConverterOutputDir(context, "pdf_to_images")
            val res = repository.convertPdfToImages(outDir.absolutePath, dpi, format, pageRange)
            res.fold(
                onSuccess = { paths ->
                    _uiState.value = _uiState.value.copy(
                        isProcessingConverter = false,
                        convertedImagePaths = paths,
                        converterMessage = "success"
                    )
                },
                onFailure = { err ->
                    _uiState.value = _uiState.value.copy(
                        isProcessingConverter = false,
                        converterMessage = err.message ?: "Failed"
                    )
                }
            )
        }
    }

    fun convertImagesToPdf(
        context: Context,
        imagePaths: List<String>,
        pageSize: ImageToPdfPageSize = ImageToPdfPageSize.A4,
        marginPt: Float = 10f
    ) {
        viewModelScope.launch {
            _uiState.value = _uiState.value.copy(isProcessingConverter = true, converterMessage = null)
            val outPath = createConverterOutputPath(context, "images_to_pdf")
            val res = repository.convertImagesToPdf(imagePaths, pageSize, marginPt, outPath)
            res.fold(
                onSuccess = {
                    _uiState.value = _uiState.value.copy(
                        isProcessingConverter = false,
                        converterMessage = "success"
                    )
                    repository.openDocument(outPath)
                },
                onFailure = { err ->
                    _uiState.value = _uiState.value.copy(
                        isProcessingConverter = false,
                        converterMessage = err.message ?: "Failed"
                    )
                }
            )
        }
    }

    fun convertMarkdownToPdf(
        context: Context,
        markdownText: String
    ) {
        viewModelScope.launch {
            _uiState.value = _uiState.value.copy(isProcessingConverter = true, converterMessage = null)
            val outPath = createConverterOutputPath(context, "markdown_to_pdf")
            val res = repository.convertMarkdownToPdf(markdownText, outPath)
            res.fold(
                onSuccess = {
                    _uiState.value = _uiState.value.copy(
                        isProcessingConverter = false,
                        converterMessage = "success"
                    )
                    repository.openDocument(outPath)
                },
                onFailure = { err ->
                    _uiState.value = _uiState.value.copy(
                        isProcessingConverter = false,
                        converterMessage = err.message ?: "Failed"
                    )
                }
            )
        }
    }
}
