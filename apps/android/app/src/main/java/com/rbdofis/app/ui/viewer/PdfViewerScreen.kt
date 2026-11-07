package com.rbdofis.app.ui.viewer

import android.graphics.Bitmap
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectTransformGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.ArrowForward
import androidx.compose.material.icons.filled.AutoAwesome
import androidx.compose.material.icons.filled.AutoFixHigh
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.DocumentScanner
import androidx.compose.material.icons.filled.Draw
import androidx.compose.material.icons.filled.FitScreen
import androidx.compose.material.icons.filled.GridView
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Share
import androidx.compose.material.icons.filled.Transform
import androidx.compose.material.icons.filled.ZoomIn
import androidx.compose.material.icons.filled.ZoomOut
import androidx.compose.material3.BottomAppBar
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.rbdofis.app.R
import com.rbdofis.app.ui.theme.PageCanvasBackgroundDark
import com.rbdofis.app.ui.theme.PageCanvasBackgroundLight

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PdfViewerScreen(
    viewModel: PdfViewerViewModel,
    onNavigateToOrganizer: () -> Unit,
    onScanDocumentRequested: () -> Unit,
    onShareRequested: () -> Unit,
    modifier: Modifier = Modifier
) {
    val uiState by viewModel.uiState.collectAsState()
    val docState = uiState.documentState
    var isAiSheetOpen by remember { mutableStateOf(false) }

    Scaffold(
        modifier = modifier.fillMaxSize(),
        topBar = {
            Column {
                TopAppBar(
                    title = {
                        Text(
                            text = if (docState.isOpen) docState.fileName else stringResource(R.string.app_name),
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                            style = MaterialTheme.typography.titleMedium
                        )
                    },
                    actions = {
                        // Drawing / Stylus Toggle
                        IconButton(
                            onClick = { viewModel.toggleInkMode() },
                            enabled = docState.isOpen
                        ) {
                            Icon(
                                imageVector = Icons.Default.Draw,
                                contentDescription = stringResource(R.string.action_toggle_ink),
                                tint = if (uiState.isInkModeActive) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurface
                            )
                        }

                        // Search
                        IconButton(
                            onClick = { viewModel.toggleSearch(!uiState.isSearchOpen) },
                            enabled = docState.isOpen
                        ) {
                            Icon(
                                imageVector = if (uiState.isSearchOpen) Icons.Default.Close else Icons.Default.Search,
                                contentDescription = stringResource(R.string.action_search)
                            )
                        }

                        // Page Organizer
                        IconButton(
                            onClick = onNavigateToOrganizer,
                            enabled = docState.isOpen
                        ) {
                            Icon(
                                imageVector = Icons.Default.GridView,
                                contentDescription = stringResource(R.string.action_organizer)
                            )
                        }

                        // Document Camera Scanner
                        IconButton(onClick = onScanDocumentRequested) {
                            Icon(
                                imageVector = Icons.Default.DocumentScanner,
                                contentDescription = stringResource(R.string.action_scan_document)
                            )
                        }

                        // Pro Tools
                        IconButton(
                            onClick = { viewModel.openProTools() },
                            enabled = docState.isOpen
                        ) {
                            Icon(
                                imageVector = Icons.Default.AutoFixHigh,
                                contentDescription = stringResource(R.string.action_pro_tools)
                            )
                        }

                        // Converter Tools
                        IconButton(onClick = { viewModel.openConverter() }) {
                            Icon(
                                imageVector = Icons.Default.Transform,
                                contentDescription = stringResource(R.string.action_convert)
                            )
                        }

                        // Offline AI Tools (Phase 8 Track 8D)
                        IconButton(
                            onClick = { isAiSheetOpen = true },
                            enabled = docState.isOpen
                        ) {
                            Icon(
                                imageVector = Icons.Default.AutoAwesome,
                                contentDescription = stringResource(R.string.ai_title)
                            )
                        }

                        // Share
                        IconButton(
                            onClick = onShareRequested,
                            enabled = docState.isOpen
                        ) {
                            Icon(
                                imageVector = Icons.Default.Share,
                                contentDescription = stringResource(R.string.action_share)
                            )
                        }
                    },
                    colors = TopAppBarDefaults.topAppBarColors(
                        containerColor = MaterialTheme.colorScheme.surface
                    )
                )

                // Search Bar dropdown when active
                if (uiState.isSearchOpen) {
                    Surface(
                        modifier = Modifier.fillMaxWidth(),
                        tonalElevation = 4.dp
                    ) {
                        Column(modifier = Modifier.padding(8.dp)) {
                            OutlinedTextField(
                                value = uiState.searchQuery,
                                onValueChange = { viewModel.onSearchQueryChanged(it) },
                                modifier = Modifier.fillMaxWidth(),
                                placeholder = { Text(stringResource(R.string.action_search_hint)) },
                                singleLine = true,
                                trailingIcon = {
                                    if (uiState.searchQuery.isNotEmpty()) {
                                        IconButton(onClick = { viewModel.onSearchQueryChanged("") }) {
                                            Icon(Icons.Default.Close, contentDescription = null)
                                        }
                                    }
                                }
                            )
                            if (uiState.searchQuery.length >= 2) {
                                Text(
                                    text = if (uiState.searchResults.isNotEmpty()) {
                                        stringResource(R.string.status_search_results, uiState.searchResults.size)
                                    } else {
                                        stringResource(R.string.status_no_search_results)
                                    },
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    modifier = Modifier.padding(top = 4.dp, start = 4.dp)
                                )
                            }
                        }
                    }
                }
            }
        },
        bottomBar = {
            BottomAppBar(
                containerColor = MaterialTheme.colorScheme.surface,
                tonalElevation = 3.dp
            ) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 8.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    // Page Navigation
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        IconButton(
                            onClick = { viewModel.previousPage() },
                            enabled = docState.isOpen && docState.currentPageIndex > 0
                        ) {
                            Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = null)
                        }

                        val indicatorText = if (docState.isOpen && docState.pageCount > 0) {
                            stringResource(
                                R.string.page_indicator,
                                docState.currentPageIndex + 1,
                                docState.pageCount
                            )
                        } else {
                            stringResource(R.string.status_no_document)
                        }
                        Text(
                            text = indicatorText,
                            style = MaterialTheme.typography.labelMedium,
                            modifier = Modifier.padding(horizontal = 4.dp)
                        )

                        IconButton(
                            onClick = { viewModel.nextPage() },
                            enabled = docState.isOpen && docState.currentPageIndex < docState.pageCount - 1
                        ) {
                            Icon(Icons.AutoMirrored.Filled.ArrowForward, contentDescription = null)
                        }
                    }

                    // Zoom Controls
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        IconButton(onClick = { viewModel.zoomOut() }) {
                            Icon(Icons.Default.ZoomOut, contentDescription = stringResource(R.string.action_zoom_out))
                        }

                        val zoomPercent = (uiState.zoomScale * 100).toInt()
                        Text(
                            text = stringResource(R.string.zoom_indicator, zoomPercent),
                            style = MaterialTheme.typography.labelSmall,
                            modifier = Modifier.width(42.dp)
                        )

                        IconButton(onClick = { viewModel.zoomIn() }) {
                            Icon(Icons.Default.ZoomIn, contentDescription = stringResource(R.string.action_zoom_in))
                        }

                        IconButton(onClick = { viewModel.fitToWidth() }) {
                            Icon(Icons.Default.FitScreen, contentDescription = stringResource(R.string.action_fit_width))
                        }
                    }
                }
            }
        }
    ) { paddingValues ->
        val isDark = MaterialTheme.colorScheme.surface.red < 0.5f
        val canvasBg = if (isDark) PageCanvasBackgroundDark else PageCanvasBackgroundLight

        Box(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
                .background(canvasBg),
            contentAlignment = Alignment.Center
        ) {
            when {
                docState.isLoading -> {
                    Column(horizontalAlignment = Alignment.CenterHorizontally) {
                        CircularProgressIndicator()
                        Spacer(modifier = Modifier.height(16.dp))
                        Text(
                            text = stringResource(R.string.status_loading),
                            style = MaterialTheme.typography.bodyMedium
                        )
                    }
                }
                !docState.isOpen -> {
                    Column(
                        horizontalAlignment = Alignment.CenterHorizontally,
                        modifier = Modifier.padding(32.dp)
                    ) {
                        Text(
                            text = stringResource(R.string.status_no_document),
                            style = MaterialTheme.typography.titleMedium
                        )
                        Spacer(modifier = Modifier.height(8.dp))
                        Text(
                            text = stringResource(R.string.privacy_assurance),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                }
                else -> {
                    // PDF Page Render Surface with synchronized Ink Canvas
                    val bitmap: Bitmap? = uiState.currentBitmap
                    if (bitmap != null) {
                        Box(
                            modifier = Modifier
                                .graphicsLayer(
                                    scaleX = uiState.zoomScale,
                                    scaleY = uiState.zoomScale,
                                    translationX = uiState.panOffset.x,
                                    translationY = uiState.panOffset.y
                                )
                                .pointerInput(uiState.isInkModeActive, uiState.activeInkTool) {
                                    if (!uiState.isInkModeActive || uiState.activeInkTool == InkTool.PAN) {
                                        detectTransformGestures { centroid, pan, zoom, _ ->
                                            viewModel.onTransform(centroid, pan, zoom)
                                        }
                                    }
                                }
                                .shadow(6.dp, RoundedCornerShape(2.dp))
                                .clip(RoundedCornerShape(2.dp))
                        ) {
                            Image(
                                bitmap = bitmap.asImageBitmap(),
                                contentDescription = stringResource(
                                    R.string.page_indicator,
                                    docState.currentPageIndex + 1,
                                    docState.pageCount
                                )
                            )

                            // Ink Drawing Overlay
                            InkAnnotationCanvas(
                                pageIndex = docState.currentPageIndex,
                                annotations = uiState.annotations,
                                activeTool = if (uiState.isInkModeActive) uiState.activeInkTool else InkTool.PAN,
                                penColor = uiState.activeInkColor,
                                strokeWidthPx = uiState.strokeWidthPx,
                                onAnnotationAdded = { stroke -> viewModel.addInkStroke(stroke) },
                                onAnnotationRemoved = { id -> viewModel.removeAnnotation(id) },
                                modifier = Modifier.matchParentSize()
                            )
                        }
                    } else {
                        CircularProgressIndicator(modifier = Modifier.size(36.dp))
                    }
                }
            }

            // Floating Ink Toolbar at bottom when Ink Mode is active
            AnimatedVisibility(
                visible = uiState.isInkModeActive && docState.isOpen,
                enter = slideInVertically(initialOffsetY = { it }),
                exit = slideOutVertically(targetOffsetY = { it }),
                modifier = Modifier.align(Alignment.BottomCenter)
            ) {
                InkToolBar(
                    activeTool = uiState.activeInkTool,
                    onToolSelected = { viewModel.selectInkTool(it) },
                    activeColor = uiState.activeInkColor,
                    onColorSelected = { viewModel.selectInkColor(it) },
                    strokeWidthPx = uiState.strokeWidthPx,
                    onStrokeWidthSelected = { viewModel.selectStrokeWidth(it) },
                    canUndo = uiState.canUndo,
                    onUndo = { viewModel.undo() },
                    canRedo = uiState.canRedo,
                    onRedo = { viewModel.redo() },
                    onClearAll = { viewModel.clearAllAnnotationsOnPage() }
                )
            }
        }

        // Pro PDF Tools Bottom Sheet
        if (uiState.isProToolsOpen) {
            val context = androidx.compose.ui.platform.LocalContext.current
            com.rbdofis.app.ui.ProToolsBottomSheet(
                uiState = uiState,
                onDismiss = { viewModel.closeProTools() },
                onSelectTab = { viewModel.selectProToolTab(it) },
                onCompress = { u, r, m -> viewModel.compressDocument(context, u, r, m) },
                onApplyWatermark = { t, f, rot, op, r, g, b, top -> viewModel.applyWatermark(context, t, f, rot, op, r, g, b, top) },
                onApplyPageNumbers = { p, pos, f, m -> viewModel.applyPageNumbers(context, p, pos, f, m) },
                onApplyBatesStamp = { p, s, pad, suf, pos -> viewModel.applyBatesStamp(context, p, s, pad, suf, pos) },
                onApplyRedaction = { json -> viewModel.applyRedaction(context, json) },
                onFillFormFields = { json, flat -> viewModel.fillFormFields(context, json, flat) }
            )
        }

        // Document Converter Bottom Sheet (Phase 5 Track 5D)
        if (uiState.isConverterOpen) {
            val context = androidx.compose.ui.platform.LocalContext.current
            com.rbdofis.app.ui.DocumentConverterBottomSheet(
                uiState = uiState,
                onDismiss = { viewModel.closeConverter() },
                onSelectTab = { viewModel.selectConverterTab(it) },
                onPdfToImages = { dpi, format, range -> viewModel.convertPdfToImages(context, dpi, format, range) },
                onImagesToPdf = { paths, size, margin -> viewModel.convertImagesToPdf(context, paths, size, margin) },
                onMarkdownToPdf = { md -> viewModel.convertMarkdownToPdf(context, md) }
            )
        }

        // Document AI Bottom Sheet (Phase 8 Track 8D)
        if (isAiSheetOpen) {
            com.rbdofis.app.ui.AiDocumentBottomSheet(
                docHandle = viewModel.getActiveDocHandle(),
                onDismissRequest = { isAiSheetOpen = false },
                onNavigateToPage = { page ->
                    viewModel.goToPage(page - 1)
                    isAiSheetOpen = false
                }
            )
        }
    }
}
