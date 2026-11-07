package com.rbdofis.app.ui.office

import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.PictureAsPdf
import androidx.compose.material.icons.filled.Save
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.rbdofis.app.R
import com.rbdofis.app.core.OfficeDocumentMetadata
import com.rbdofis.app.core.RbdNativeBridge
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.File

/**
 * Mobile Office Document Editor Screen (Phase 6 Track 6D).
 * Compliant with ADR-0004:
 * - 100% offline, zero network access.
 * - Supports editing and viewing DOCX, XLSX, and PPTX documents.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun OfficeEditorScreen(
    filePath: String,
    onNavigateBack: () -> Unit,
    onPdfGenerated: (String) -> Unit,
    modifier: Modifier = Modifier
) {
    val coroutineScope = rememberCoroutineScope()
    val snackbarHostState = remember { SnackbarHostState() }
    val bridge = remember { RbdNativeBridge.getInstance() }

    var metadata by remember { mutableStateOf<OfficeDocumentMetadata?>(null) }
    var textContent by remember { mutableStateOf("") }
    var isLoading by remember { mutableStateOf(true) }
    var isSaving by remember { mutableStateOf(false) }

    val docType = remember(filePath) {
        val ext = filePath.substringAfterLast('.', "docx").uppercase()
        if (ext in listOf("DOCX", "DOC", "XLSX", "XLS", "PPTX", "PPT")) ext else "DOCX"
    }

    val typeColor = when (docType) {
        "XLSX", "XLS" -> Color(0xFF217346)
        "PPTX", "PPT" -> Color(0xFFD24726)
        else -> Color(0xFF2B579A)
    }

    // Load document content in background IO thread
    LaunchedEffect(filePath) {
        withContext(Dispatchers.IO) {
            val meta = bridge.officeInspect(filePath)
            val text = bridge.officeExtractText(filePath) ?: ""
            withContext(Dispatchers.Main) {
                metadata = meta
                textContent = text
                isLoading = false
            }
        }
    }

    Scaffold(
        modifier = modifier.fillMaxSize(),
        snackbarHost = { SnackbarHost(snackbarHostState) },
        topBar = {
            TopAppBar(
                title = {
                    Column {
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Text(
                                text = File(filePath).name,
                                style = MaterialTheme.typography.titleMedium,
                                maxLines = 1
                            )
                            Spacer(modifier = Modifier.width(8.dp))
                            Surface(
                                color = typeColor,
                                shape = RoundedCornerShape(4.dp)
                            ) {
                                Text(
                                    text = docType,
                                    color = Color.White,
                                    fontSize = 10.sp,
                                    fontWeight = FontWeight.Bold,
                                    modifier = Modifier.padding(horizontal = 6.dp, vertical = 2.dp)
                                )
                            }
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
                    // Export to PDF button
                    IconButton(
                        onClick = {
                            coroutineScope.launch {
                                isSaving = true
                                val outPdfPath = filePath.substringBeforeLast('.') + "_export.pdf"
                                val success = withContext(Dispatchers.IO) {
                                    bridge.markdownToPdf(textContent, outPdfPath)
                                }
                                isSaving = false
                                if (success) {
                                    snackbarHostState.showSnackbar(
                                        message = "PDF belgesi oluşturuldu."
                                    )
                                    onPdfGenerated(outPdfPath)
                                }
                            }
                        },
                        enabled = !isLoading && !isSaving
                    ) {
                        Icon(
                            imageVector = Icons.Default.PictureAsPdf,
                            contentDescription = stringResource(R.string.office_action_export_pdf)
                        )
                    }

                    // Save Document button
                    IconButton(
                        onClick = {
                            coroutineScope.launch {
                                isSaving = true
                                val success = withContext(Dispatchers.IO) {
                                    bridge.officeCreateDocx(textContent, filePath)
                                }
                                isSaving = false
                                if (success) {
                                    snackbarHostState.showSnackbar(
                                        message = "Belge başarıyla kaydedildi."
                                    )
                                }
                            }
                        },
                        enabled = !isLoading && !isSaving
                    ) {
                        Icon(
                            imageVector = Icons.Default.Save,
                            contentDescription = stringResource(R.string.office_action_save)
                        )
                    }
                }
            )
        }
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
        ) {
            // Status and metadata bar
            Surface(
                color = MaterialTheme.colorScheme.surfaceVariant,
                modifier = Modifier.fillMaxWidth()
            ) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 16.dp, vertical = 8.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    val words = textContent.trim().split(Regex("\\s+")).filter { it.isNotBlank() }.size
                    Text(
                        text = stringResource(R.string.office_words_count, words),
                        style = MaterialTheme.typography.bodySmall
                    )
                    Text(
                        text = "🔒 Çevrimdışı (Sıfır Ağ)",
                        style = MaterialTheme.typography.bodySmall,
                        color = Color(0xFF107C41),
                        fontWeight = FontWeight.Bold
                    )
                }
            }

            if (isLoading) {
                Box(
                    modifier = Modifier.fillMaxSize(),
                    contentAlignment = Alignment.Center
                ) {
                    CircularProgressIndicator()
                }
            } else {
                when (docType) {
                    "XLSX", "XLS" -> {
                        // Spreadsheet tabular view
                        SpreadsheetGrid(
                            content = textContent,
                            onContentChanged = { textContent = it },
                            modifier = Modifier.fillMaxSize()
                        )
                    }
                    else -> {
                        // Document editor text view
                        OutlinedTextField(
                            value = textContent,
                            onValueChange = { textContent = it },
                            modifier = Modifier
                                .fillMaxSize()
                                .padding(16.dp),
                            placeholder = { Text("Belge içeriğini buraya yazın…") },
                            colors = OutlinedTextFieldDefaults.colors(
                                focusedContainerColor = Color.White,
                                unfocusedContainerColor = Color.White
                            )
                        )
                    }
                }
            }
        }
    }
}

/**
 * Mobile spreadsheet tabular grid representation.
 */
@Composable
private fun SpreadsheetGrid(
    content: String,
    onContentChanged: (String) -> Unit,
    modifier: Modifier = Modifier
) {
    val lines = remember(content) {
        content.split('\n').filter { it.isNotBlank() }
    }

    Column(
        modifier = modifier
            .verticalScroll(rememberScrollState())
            .horizontalScroll(rememberScrollState())
            .padding(12.dp)
    ) {
        // Column headers
        Row {
            Box(
                modifier = Modifier
                    .width(40.dp)
                    .height(30.dp)
                    .background(Color(0xFFE0E0E0)),
                contentAlignment = Alignment.Center
            ) {
                Text("#", fontSize = 12.sp, fontWeight = FontWeight.Bold)
            }
            for (c in 0 until 6) {
                Box(
                    modifier = Modifier
                        .width(90.dp)
                        .height(30.dp)
                        .background(Color(0xFFF0F0F0)),
                    contentAlignment = Alignment.Center
                ) {
                    Text(('A'.code + c).toChar().toString(), fontSize = 12.sp, fontWeight = FontWeight.Bold)
                }
            }
        }

        // Rows
        lines.forEachIndexed { rIdx, line ->
            val cells = line.split('\t')
            Row {
                Box(
                    modifier = Modifier
                        .width(40.dp)
                        .height(36.dp)
                        .background(Color(0xFFF0F0F0)),
                    contentAlignment = Alignment.Center
                ) {
                    Text((rIdx + 1).toString(), fontSize = 11.sp, color = Color.Gray)
                }
                for (cIdx in 0 until 6) {
                    val cellVal = cells.getOrElse(cIdx) { "" }
                    Surface(
                        modifier = Modifier
                            .width(90.dp)
                            .height(36.dp)
                            .padding(1.dp),
                        shape = RoundedCornerShape(2.dp),
                        color = Color.White,
                        tonalElevation = 1.dp
                    ) {
                        Box(
                            modifier = Modifier
                                .fillMaxSize()
                                .padding(4.dp),
                            contentAlignment = Alignment.CenterStart
                        ) {
                            Text(text = cellVal, fontSize = 12.sp, maxLines = 1)
                        }
                    }
                }
            }
        }
    }
}
