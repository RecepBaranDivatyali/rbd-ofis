package com.rbdofis.app.ui.organizer

import android.graphics.Bitmap
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.ArrowForward
import androidx.compose.material.icons.automirrored.filled.RotateLeft
import androidx.compose.material.icons.automirrored.filled.RotateRight
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Save
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.BottomAppBar
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.rbdofis.app.R
import kotlinx.coroutines.launch

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun PageOrganizerScreen(
    viewModel: PageOrganizerViewModel,
    onNavigateBack: () -> Unit,
    modifier: Modifier = Modifier
) {
    val context = LocalContext.current
    val uiState by viewModel.uiState.collectAsState()
    val docState = uiState.documentState
    val snackbarHostState = remember { SnackbarHostState() }
    val scope = rememberCoroutineScope()

    // Delete confirmation dialog
    if (uiState.deleteConfirmationPage != null) {
        val targetPage = (uiState.deleteConfirmationPage ?: 0) + 1
        AlertDialog(
            onDismissRequest = { viewModel.dismissDeleteDialog() },
            title = { Text(stringResource(R.string.dialog_delete_page_title)) },
            text = { Text(stringResource(R.string.dialog_delete_page_message, targetPage)) },
            confirmButton = {
                TextButton(
                    onClick = { viewModel.confirmDeletePage() }
                ) {
                    Text(stringResource(R.string.dialog_confirm), color = MaterialTheme.colorScheme.error)
                }
            },
            dismissButton = {
                TextButton(onClick = { viewModel.dismissDeleteDialog() }) {
                    Text(stringResource(R.string.dialog_cancel))
                }
            }
        )
    }

    Scaffold(
        modifier = modifier.fillMaxSize(),
        snackbarHost = { SnackbarHost(snackbarHostState) },
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Text(
                            text = stringResource(R.string.screen_organizer),
                            style = MaterialTheme.typography.titleMedium
                        )
                        if (docState.isOpen) {
                            Text(
                                text = docState.fileName,
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis
                            )
                        }
                    }
                },
                navigationIcon = {
                    IconButton(onClick = onNavigateBack) {
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = stringResource(R.string.action_close)
                        )
                    }
                },
                actions = {
                    if (docState.isModified) {
                        IconButton(
                            onClick = {
                                viewModel.saveDocument { success ->
                                    scope.launch {
                                        val message = if (success) {
                                            context.getString(R.string.status_saved)
                                        } else {
                                            context.getString(R.string.status_save_failed)
                                        }
                                        snackbarHostState.showSnackbar(message)
                                    }
                                }
                            }
                        ) {
                            Icon(
                                imageVector = Icons.Default.Save,
                                contentDescription = stringResource(R.string.action_save)
                            )
                        }
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.surface
                )
            )
        },
        bottomBar = {
            if (uiState.selectedPageIndex != null) {
                val selectedIndex = uiState.selectedPageIndex ?: 0
                BottomAppBar(
                    containerColor = MaterialTheme.colorScheme.surfaceContainerHigh,
                    tonalElevation = 6.dp
                ) {
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(horizontal = 8.dp),
                        horizontalArrangement = Arrangement.SpaceEvenly,
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        // Move Previous
                        IconButton(
                            onClick = { viewModel.movePage(selectedIndex, selectedIndex - 1) },
                            enabled = selectedIndex > 0
                        ) {
                            Icon(
                                Icons.AutoMirrored.Filled.ArrowBack,
                                contentDescription = stringResource(R.string.action_reorder_page)
                            )
                        }

                        // Rotate Counter-Clockwise (-90)
                        IconButton(onClick = { viewModel.rotateSelectedPage(-90) }) {
                            Icon(
                                Icons.AutoMirrored.Filled.RotateLeft,
                                contentDescription = stringResource(R.string.action_rotate_counterclockwise)
                            )
                        }

                        // Rotate Clockwise (+90)
                        IconButton(onClick = { viewModel.rotateSelectedPage(90) }) {
                            Icon(
                                Icons.AutoMirrored.Filled.RotateRight,
                                contentDescription = stringResource(R.string.action_rotate_clockwise)
                            )
                        }

                        // Delete Page
                        IconButton(
                            onClick = { viewModel.requestDeletePage(selectedIndex) },
                            enabled = uiState.pages.size > 1
                        ) {
                            Icon(
                                Icons.Default.Delete,
                                contentDescription = stringResource(R.string.action_delete_page),
                                tint = MaterialTheme.colorScheme.error
                            )
                        }

                        // Move Next
                        IconButton(
                            onClick = { viewModel.movePage(selectedIndex, selectedIndex + 1) },
                            enabled = selectedIndex < uiState.pages.size - 1
                        ) {
                            Icon(
                                Icons.AutoMirrored.Filled.ArrowForward,
                                contentDescription = stringResource(R.string.action_reorder_page)
                            )
                        }
                    }
                }
            }
        }
    ) { paddingValues ->
        if (uiState.pages.isEmpty()) {
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(paddingValues),
                contentAlignment = Alignment.Center
            ) {
                CircularProgressIndicator()
            }
        } else {
            LazyVerticalGrid(
                columns = GridCells.Fixed(2),
                contentPadding = PaddingValues(16.dp),
                horizontalArrangement = Arrangement.spacedBy(16.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp),
                modifier = Modifier
                    .fillMaxSize()
                    .padding(paddingValues)
            ) {
                items(uiState.pages, key = { it.index }) { page ->
                    val isSelected = uiState.selectedPageIndex == page.index
                    val border = if (isSelected) {
                        BorderStroke(3.dp, MaterialTheme.colorScheme.primary)
                    } else {
                        BorderStroke(1.dp, MaterialTheme.colorScheme.outlineVariant)
                    }

                    Card(
                        modifier = Modifier
                            .fillMaxWidth()
                            .clickable { viewModel.selectPage(page.index) },
                        shape = RoundedCornerShape(8.dp),
                        border = border,
                        colors = CardDefaults.cardColors(
                            containerColor = MaterialTheme.colorScheme.surface
                        ),
                        elevation = CardDefaults.cardElevation(if (isSelected) 6.dp else 2.dp)
                    ) {
                        Column(
                            horizontalAlignment = Alignment.CenterHorizontally,
                            modifier = Modifier.padding(8.dp)
                        ) {
                            Box(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .aspectRatio(page.dimensions.aspectRatio)
                                    .clip(RoundedCornerShape(4.dp))
                                    .background(MaterialTheme.colorScheme.surfaceVariant),
                                contentAlignment = Alignment.Center
                            ) {
                                val thumb: Bitmap? = uiState.thumbnails[page.index]
                                if (thumb != null) {
                                    Image(
                                        bitmap = thumb.asImageBitmap(),
                                        contentDescription = null,
                                        modifier = Modifier.fillMaxSize()
                                    )
                                } else {
                                    CircularProgressIndicator(modifier = Modifier.size(24.dp))
                                }

                                // Selection badge
                                if (isSelected) {
                                    Box(
                                        modifier = Modifier
                                            .align(Alignment.TopEnd)
                                            .padding(6.dp)
                                            .size(20.dp)
                                            .background(MaterialTheme.colorScheme.primary, CircleShape),
                                        contentAlignment = Alignment.Center
                                    ) {
                                        Text(
                                            text = "✓",
                                            color = MaterialTheme.colorScheme.onPrimary,
                                            style = MaterialTheme.typography.labelSmall
                                        )
                                    }
                                }
                            }

                            Spacer(modifier = Modifier.height(6.dp))

                            Text(
                                text = "${page.index + 1}",
                                style = MaterialTheme.typography.labelMedium,
                                color = if (isSelected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurface
                            )
                        }
                    }
                }
            }
        }
    }
}
