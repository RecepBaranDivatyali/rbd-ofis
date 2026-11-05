package com.rbdofis.app.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Description
import androidx.compose.material.icons.filled.Image
import androidx.compose.material.icons.filled.PictureAsPdf
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.MenuAnchorType
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.PrimaryTabRow
import androidx.compose.material3.Slider
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import com.rbdofis.app.R
import com.rbdofis.app.core.ImageExportFormat
import com.rbdofis.app.core.ImageToPdfPageSize
import com.rbdofis.app.ui.viewer.ConverterTab
import com.rbdofis.app.ui.viewer.ViewerUiState

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun DocumentConverterBottomSheet(
    uiState: ViewerUiState,
    onDismiss: () -> Unit,
    onSelectTab: (ConverterTab) -> Unit,
    onPdfToImages: (dpi: Int, format: ImageExportFormat, pageRange: List<Int>?) -> Unit,
    onImagesToPdf: (imagePaths: List<String>, pageSize: ImageToPdfPageSize, marginPt: Float) -> Unit,
    onMarkdownToPdf: (markdownText: String) -> Unit,
    modifier: Modifier = Modifier
) {
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)

    ModalBottomSheet(
        onDismissRequest = onDismiss,
        sheetState = sheetState,
        modifier = modifier
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(bottom = 32.dp)
        ) {
            // Header
            Row(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 20.dp, vertical = 8.dp),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = stringResource(R.string.converter_title),
                    style = MaterialTheme.typography.titleLarge,
                    fontWeight = FontWeight.Bold
                )
                IconButton(onClick = onDismiss) {
                    Icon(
                        imageVector = Icons.Default.Close,
                        contentDescription = stringResource(R.string.action_close)
                    )
                }
            }

            // Tab Navigation
            val tabs = listOf(
                ConverterTabItem(ConverterTab.PDF_TO_IMAGES, stringResource(R.string.converter_tab_pdf_to_images), Icons.Default.Image),
                ConverterTabItem(ConverterTab.IMAGES_TO_PDF, stringResource(R.string.converter_tab_images_to_pdf), Icons.Default.PictureAsPdf),
                ConverterTabItem(ConverterTab.MARKDOWN_TO_PDF, stringResource(R.string.converter_tab_markdown_to_pdf), Icons.Default.Description)
            )
            val selectedTabIndex = tabs.indexOfFirst { it.tab == uiState.activeConverterTab }.coerceAtLeast(0)

            PrimaryTabRow(selectedTabIndex = selectedTabIndex) {
                tabs.forEachIndexed { index, tabItem ->
                    Tab(
                        selected = selectedTabIndex == index,
                        onClick = { onSelectTab(tabItem.tab) },
                        text = { Text(tabItem.title, maxLines = 1) },
                        icon = { Icon(tabItem.icon, contentDescription = null, modifier = Modifier.size(18.dp)) }
                    )
                }
            }

            // Status feedback banner
            if (uiState.isProcessingConverter) {
                Card(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 20.dp, vertical = 8.dp),
                    colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.primaryContainer)
                ) {
                    Row(
                        modifier = Modifier.padding(16.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        CircularProgressIndicator(
                            modifier = Modifier.size(24.dp),
                            strokeWidth = 2.dp,
                            color = MaterialTheme.colorScheme.primary
                        )
                        Spacer(modifier = Modifier.width(16.dp))
                        Text(
                            text = stringResource(R.string.status_processing_converter),
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onPrimaryContainer
                        )
                    }
                }
            }

            uiState.converterMessage?.let { msg ->
                val isSuccess = msg == "success"
                Card(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 20.dp, vertical = 8.dp),
                    colors = CardDefaults.cardColors(
                        containerColor = if (isSuccess) MaterialTheme.colorScheme.secondaryContainer else MaterialTheme.colorScheme.errorContainer
                    )
                ) {
                    Text(
                        text = if (isSuccess) {
                            if (uiState.activeConverterTab == ConverterTab.PDF_TO_IMAGES) {
                                stringResource(R.string.converter_success_images, uiState.convertedImagePaths.size)
                            } else {
                                stringResource(R.string.converter_success_pdf)
                            }
                        } else {
                            stringResource(R.string.status_pro_tool_failed, msg)
                        },
                        style = MaterialTheme.typography.bodyMedium,
                        color = if (isSuccess) MaterialTheme.colorScheme.onSecondaryContainer else MaterialTheme.colorScheme.onErrorContainer,
                        modifier = Modifier.padding(16.dp)
                    )
                }
            }

            // Tab Content
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .verticalScroll(rememberScrollState())
                    .padding(horizontal = 20.dp, vertical = 12.dp)
            ) {
                when (uiState.activeConverterTab) {
                    ConverterTab.PDF_TO_IMAGES -> PdfToImagesSection(
                        isDocOpen = uiState.documentState.isOpen,
                        onConvert = onPdfToImages,
                        convertedPaths = uiState.convertedImagePaths
                    )
                    ConverterTab.IMAGES_TO_PDF -> ImagesToPdfSection(
                        onConvert = onImagesToPdf
                    )
                    ConverterTab.MARKDOWN_TO_PDF -> MarkdownToPdfSection(
                        onConvert = onMarkdownToPdf
                    )
                }
            }
        }
    }
}

private data class ConverterTabItem(
    val tab: ConverterTab,
    val title: String,
    val icon: ImageVector
)

// =============================================================================
// Section 1: PDF to Images
// =============================================================================

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun PdfToImagesSection(
    isDocOpen: Boolean,
    onConvert: (dpi: Int, format: ImageExportFormat, pageRange: List<Int>?) -> Unit,
    convertedPaths: List<String>,
    modifier: Modifier = Modifier
) {
    var selectedDpi by remember { mutableIntStateOf(150) }
    var selectedFormat by remember { mutableStateOf(ImageExportFormat.PNG) }
    var pageRangeText by remember { mutableStateOf("") }
    var formatExpanded by remember { mutableStateOf(false) }

    Column(modifier = modifier.fillMaxWidth()) {
        if (!isDocOpen) {
            Card(
                modifier = Modifier.fillMaxWidth(),
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant)
            ) {
                Text(
                    text = stringResource(R.string.error_open_document),
                    style = MaterialTheme.typography.bodyMedium,
                    modifier = Modifier.padding(16.dp)
                )
            }
            return
        }

        // DPI Slider
        Text(
            text = stringResource(R.string.converter_dpi_label, selectedDpi),
            style = MaterialTheme.typography.titleSmall,
            fontWeight = FontWeight.SemiBold
        )
        Slider(
            value = selectedDpi.toFloat(),
            onValueChange = { selectedDpi = (it / 25).toInt() * 25 },
            valueRange = 72f..300f,
            steps = 8,
            modifier = Modifier.fillMaxWidth()
        )

        Spacer(modifier = Modifier.height(12.dp))

        // Format Dropdown
        Text(
            text = stringResource(R.string.converter_format_label),
            style = MaterialTheme.typography.titleSmall,
            fontWeight = FontWeight.SemiBold
        )
        Spacer(modifier = Modifier.height(4.dp))
        ExposedDropdownMenuBox(
            expanded = formatExpanded,
            onExpandedChange = { formatExpanded = !formatExpanded }
        ) {
            OutlinedTextField(
                value = if (selectedFormat == ImageExportFormat.PNG) stringResource(R.string.converter_format_png) else stringResource(R.string.converter_format_jpeg),
                onValueChange = {},
                readOnly = true,
                trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = formatExpanded) },
                modifier = Modifier
                    .menuAnchor(MenuAnchorType.PrimaryNotEditable)
                    .fillMaxWidth()
            )
            ExposedDropdownMenu(
                expanded = formatExpanded,
                onDismissRequest = { formatExpanded = false }
            ) {
                DropdownMenuItem(
                    text = { Text(stringResource(R.string.converter_format_png)) },
                    onClick = {
                        selectedFormat = ImageExportFormat.PNG
                        formatExpanded = false
                    }
                )
                DropdownMenuItem(
                    text = { Text(stringResource(R.string.converter_format_jpeg)) },
                    onClick = {
                        selectedFormat = ImageExportFormat.JPEG
                        formatExpanded = false
                    }
                )
            }
        }

        Spacer(modifier = Modifier.height(16.dp))

        // Page Range Input
        OutlinedTextField(
            value = pageRangeText,
            onValueChange = { pageRangeText = it },
            label = { Text(stringResource(R.string.converter_page_range_label)) },
            modifier = Modifier.fillMaxWidth(),
            singleLine = true
        )

        Spacer(modifier = Modifier.height(20.dp))

        Button(
            onClick = {
                val parsedRange = parsePageRange(pageRangeText)
                onConvert(selectedDpi, selectedFormat, parsedRange)
            },
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(8.dp)
        ) {
            Text(stringResource(R.string.converter_action_export_images))
        }

        if (convertedPaths.isNotEmpty()) {
            Spacer(modifier = Modifier.height(16.dp))
            Card(
                modifier = Modifier.fillMaxWidth(),
                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant)
            ) {
                Column(modifier = Modifier.padding(12.dp)) {
                    Text(
                        text = stringResource(R.string.converter_success_images, convertedPaths.size),
                        style = MaterialTheme.typography.titleSmall,
                        fontWeight = FontWeight.SemiBold
                    )
                    Spacer(modifier = Modifier.height(6.dp))
                    convertedPaths.forEach { path ->
                        Text(
                            text = path.substringAfterLast(java.io.File.separator),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                }
            }
        }
    }
}

// =============================================================================
// Section 2: Images to PDF
// =============================================================================

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ImagesToPdfSection(
    onConvert: (imagePaths: List<String>, pageSize: ImageToPdfPageSize, marginPt: Float) -> Unit,
    modifier: Modifier = Modifier
) {
    var imagesInput by remember { mutableStateOf("") }
    var selectedSize by remember { mutableStateOf(ImageToPdfPageSize.A4) }
    var marginPt by remember { mutableFloatStateOf(10f) }
    var sizeExpanded by remember { mutableStateOf(false) }

    Column(modifier = modifier.fillMaxWidth()) {
        Text(
            text = stringResource(R.string.converter_images_to_pdf_hint),
            style = MaterialTheme.typography.bodyMedium
        )
        Spacer(modifier = Modifier.height(6.dp))

        OutlinedTextField(
            value = imagesInput,
            onValueChange = { imagesInput = it },
            placeholder = { Text("/storage/emulated/0/DCIM/sample.jpg\n/storage/emulated/0/Download/scan.png") },
            modifier = Modifier
                .fillMaxWidth()
                .height(120.dp),
            maxLines = 6
        )

        Spacer(modifier = Modifier.height(16.dp))

        // Page Size Dropdown
        Text(
            text = stringResource(R.string.converter_page_size_label),
            style = MaterialTheme.typography.titleSmall,
            fontWeight = FontWeight.SemiBold
        )
        Spacer(modifier = Modifier.height(4.dp))
        ExposedDropdownMenuBox(
            expanded = sizeExpanded,
            onExpandedChange = { sizeExpanded = !sizeExpanded }
        ) {
            val sizeLabel = when (selectedSize) {
                ImageToPdfPageSize.A4 -> stringResource(R.string.converter_size_a4)
                ImageToPdfPageSize.LETTER -> stringResource(R.string.converter_size_letter)
                ImageToPdfPageSize.FIT_IMAGE -> stringResource(R.string.converter_size_fit_image)
            }
            OutlinedTextField(
                value = sizeLabel,
                onValueChange = {},
                readOnly = true,
                trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = sizeExpanded) },
                modifier = Modifier
                    .menuAnchor(MenuAnchorType.PrimaryNotEditable)
                    .fillMaxWidth()
            )
            ExposedDropdownMenu(
                expanded = sizeExpanded,
                onDismissRequest = { sizeExpanded = false }
            ) {
                DropdownMenuItem(
                    text = { Text(stringResource(R.string.converter_size_a4)) },
                    onClick = {
                        selectedSize = ImageToPdfPageSize.A4
                        sizeExpanded = false
                    }
                )
                DropdownMenuItem(
                    text = { Text(stringResource(R.string.converter_size_letter)) },
                    onClick = {
                        selectedSize = ImageToPdfPageSize.LETTER
                        sizeExpanded = false
                    }
                )
                DropdownMenuItem(
                    text = { Text(stringResource(R.string.converter_size_fit_image)) },
                    onClick = {
                        selectedSize = ImageToPdfPageSize.FIT_IMAGE
                        sizeExpanded = false
                    }
                )
            }
        }

        Spacer(modifier = Modifier.height(16.dp))

        // Margin Slider
        Text(
            text = stringResource(R.string.converter_margin_label, marginPt.toInt()),
            style = MaterialTheme.typography.titleSmall,
            fontWeight = FontWeight.SemiBold
        )
        Slider(
            value = marginPt,
            onValueChange = { marginPt = it },
            valueRange = 0f..50f,
            steps = 9,
            modifier = Modifier.fillMaxWidth()
        )

        Spacer(modifier = Modifier.height(20.dp))

        Button(
            onClick = {
                val paths = imagesInput.lines()
                    .map { it.trim() }
                    .filter { it.isNotBlank() }
                if (paths.isNotEmpty()) {
                    onConvert(paths, selectedSize, marginPt)
                }
            },
            enabled = imagesInput.isNotBlank(),
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(8.dp)
        ) {
            Text(stringResource(R.string.converter_action_create_pdf))
        }
    }
}

// =============================================================================
// Section 3: Markdown to PDF
// =============================================================================

@Composable
private fun MarkdownToPdfSection(
    onConvert: (markdownText: String) -> Unit,
    modifier: Modifier = Modifier
) {
    var mdContent by remember {
        mutableStateOf(
            "# RBD OFİS\n\nBu belge **RBD OFİS Mobil Belge Dönüştürücüsü** tarafından üretilmiştir.\n\n### Özellikler:\n- %100 Çevrimdışı ve Güvenli\n- Sıfır Telemetri\n- Yüksek Hızlı Rust PDF Çekirdeği\n"
        )
    }

    Column(modifier = modifier.fillMaxWidth()) {
        Text(
            text = stringResource(R.string.converter_md_hint),
            style = MaterialTheme.typography.bodyMedium
        )
        Spacer(modifier = Modifier.height(8.dp))

        OutlinedTextField(
            value = mdContent,
            onValueChange = { mdContent = it },
            modifier = Modifier
                .fillMaxWidth()
                .height(180.dp),
            maxLines = 12
        )

        Spacer(modifier = Modifier.height(20.dp))

        Button(
            onClick = {
                if (mdContent.isNotBlank()) {
                    onConvert(mdContent)
                }
            },
            enabled = mdContent.isNotBlank(),
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(8.dp)
        ) {
            Text(stringResource(R.string.converter_action_render_md))
        }
    }
}

private fun parsePageRange(rangeStr: String): List<Int>? {
    if (rangeStr.isBlank()) return null
    val result = mutableSetOf<Int>()
    val parts = rangeStr.split(",", ";")
    for (part in parts) {
        val trimmed = part.trim()
        if (trimmed.contains("-")) {
            val rangeParts = trimmed.split("-")
            val start = rangeParts.getOrNull(0)?.trim()?.toIntOrNull()
            val end = rangeParts.getOrNull(1)?.trim()?.toIntOrNull()
            if (start != null && end != null && start <= end) {
                for (p in start..end) {
                    if (p >= 1) result.add(p - 1) // 0-based indexing
                }
            }
        } else {
            val p = trimmed.toIntOrNull()
            if (p != null && p >= 1) {
                result.add(p - 1)
            }
        }
    }
    return if (result.isEmpty()) null else result.toList().sorted()
}
