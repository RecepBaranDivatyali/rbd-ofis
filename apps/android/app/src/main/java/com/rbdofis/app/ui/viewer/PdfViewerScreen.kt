package com.rbdofis.app.ui.viewer

import android.graphics.Bitmap
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
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.DocumentScanner
import androidx.compose.material.icons.filled.FitScreen
import androidx.compose.material.icons.filled.GridView
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Share
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
                        IconButton(
                            onClick = { viewModel.toggleSearch(!uiState.isSearchOpen) }
                        ) {
                            Icon(
                                imageVector = if (uiState.isSearchOpen) Icons.Default.Close else Icons.Default.Search,
                                contentDescription = stringResource(R.string.action_search)
                            )
                        }
                        IconButton(onClick = onNavigateToOrganizer) {
                            Icon(
                                imageVector = Icons.Default.GridView,
                                contentDescription = stringResource(R.string.action_organizer)
                            )
                        }
                        IconButton(onClick = onScanDocumentRequested) {
                            Icon(
                                imageVector = Icons.Default.DocumentScanner,
                                contentDescription = stringResource(R.string.action_scan_document)
                            )
                        }
                        IconButton(onClick = onShareRequested) {
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
                    // PDF Page Render Surface with pinch-to-zoom and pan gestures
                    val bitmap: Bitmap? = uiState.currentBitmap
                    if (bitmap != null) {
                        Image(
                            bitmap = bitmap.asImageBitmap(),
                            contentDescription = stringResource(
                                R.string.page_indicator,
                                docState.currentPageIndex + 1,
                                docState.pageCount
                            ),
                            modifier = Modifier
                                .graphicsLayer(
                                    scaleX = uiState.zoomScale,
                                    scaleY = uiState.zoomScale,
                                    translationX = uiState.panOffset.x,
                                    translationY = uiState.panOffset.y
                                )
                                .pointerInput(Unit) {
                                    detectTransformGestures { centroid, pan, zoom, _ ->
                                        viewModel.onTransform(centroid, pan, zoom)
                                    }
                                }
                                .shadow(6.dp, RoundedCornerShape(2.dp))
                                .clip(RoundedCornerShape(2.dp))
                        )
                    } else {
                        CircularProgressIndicator(modifier = Modifier.size(36.dp))
                    }
                }
            }
        }
    }
}
