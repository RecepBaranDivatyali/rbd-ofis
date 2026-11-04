package com.rbdofis.app.ui.organizer

import android.graphics.Bitmap
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.rbdofis.app.data.DocumentState
import com.rbdofis.app.data.PageItem
import com.rbdofis.app.data.PdfDocumentRepository
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class OrganizerUiState(
    val documentState: DocumentState = DocumentState(),
    val pages: List<PageItem> = emptyList(),
    val selectedPageIndex: Int? = null,
    val thumbnails: Map<Int, Bitmap> = emptyMap(),
    val isSaving: Boolean = false,
    val deleteConfirmationPage: Int? = null
)

class PageOrganizerViewModel(
    private val repository: PdfDocumentRepository
) : ViewModel() {

    private val _uiState = MutableStateFlow(OrganizerUiState())
    val uiState: StateFlow<OrganizerUiState> = _uiState.asStateFlow()

    init {
        viewModelScope.launch {
            repository.documentState.collect { docState ->
                _uiState.value = _uiState.value.copy(documentState = docState)
                if (docState.isOpen) {
                    loadPages()
                }
            }
        }
    }

    fun loadPages() {
        viewModelScope.launch {
            val pages = repository.getAllPages()
            _uiState.value = _uiState.value.copy(pages = pages)
            loadThumbnails(pages)
        }
    }

    private fun loadThumbnails(pages: List<PageItem>) {
        viewModelScope.launch {
            val thumbs = mutableMapOf<Int, Bitmap>()
            for (page in pages) {
                val bitmap = repository.renderPage(
                    pageIndex = page.index,
                    targetWidth = 240,
                    targetHeight = 340,
                    scale = 0.3f
                )
                if (bitmap != null) {
                    thumbs[page.index] = bitmap
                }
            }
            _uiState.value = _uiState.value.copy(thumbnails = thumbs)
        }
    }

    fun selectPage(index: Int) {
        val current = _uiState.value.selectedPageIndex
        _uiState.value = _uiState.value.copy(
            selectedPageIndex = if (current == index) null else index
        )
    }

    fun rotateSelectedPage(degrees: Int) {
        val index = _uiState.value.selectedPageIndex ?: return
        viewModelScope.launch {
            val success = repository.rotatePage(index, degrees)
            if (success) {
                loadPages()
            }
        }
    }

    fun requestDeletePage(index: Int) {
        _uiState.value = _uiState.value.copy(deleteConfirmationPage = index)
    }

    fun dismissDeleteDialog() {
        _uiState.value = _uiState.value.copy(deleteConfirmationPage = null)
    }

    fun confirmDeletePage() {
        val index = _uiState.value.deleteConfirmationPage ?: return
        viewModelScope.launch {
            val success = repository.deletePage(index)
            if (success) {
                _uiState.value = _uiState.value.copy(
                    deleteConfirmationPage = null,
                    selectedPageIndex = null
                )
                loadPages()
            }
        }
    }

    fun movePage(fromIndex: Int, toIndex: Int) {
        if (fromIndex == toIndex) return
        viewModelScope.launch {
            val success = repository.reorderPage(fromIndex, toIndex)
            if (success) {
                _uiState.value = _uiState.value.copy(selectedPageIndex = toIndex)
                loadPages()
            }
        }
    }

    fun saveDocument(onComplete: (Boolean) -> Unit) {
        val doc = _uiState.value.documentState
        if (!doc.isOpen || doc.filePath.isBlank()) {
            onComplete(false)
            return
        }
        viewModelScope.launch {
            _uiState.value = _uiState.value.copy(isSaving = true)
            val success = repository.saveDocument(doc.filePath)
            _uiState.value = _uiState.value.copy(isSaving = false)
            onComplete(success)
        }
    }
}
