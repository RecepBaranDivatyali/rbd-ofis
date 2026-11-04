package com.rbdofis.app.ui.viewer

import android.graphics.Bitmap
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.rbdofis.app.core.PdfAnnotation
import com.rbdofis.app.core.SearchResult
import com.rbdofis.app.data.DocumentState
import com.rbdofis.app.data.PdfDocumentRepository
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

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
    val canRedo: Boolean = false
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
}
