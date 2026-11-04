package com.rbdofis.app.ui

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
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
import androidx.compose.material.icons.filled.Compress
import androidx.compose.material.icons.filled.FormatListNumbered
import androidx.compose.material.icons.filled.Gavel
import androidx.compose.material.icons.filled.Security
import androidx.compose.material.icons.filled.TextFields
import androidx.compose.material.icons.filled.WaterDrop
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
import androidx.compose.material3.PrimaryScrollableTabRow
import androidx.compose.material3.Slider
import androidx.compose.material3.Switch
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateMapOf
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
import com.rbdofis.app.core.CompressionStats
import com.rbdofis.app.core.FormFieldInfo
import com.rbdofis.app.core.StampPosition
import com.rbdofis.app.ui.viewer.ProToolTab
import com.rbdofis.app.ui.viewer.ViewerUiState

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ProToolsBottomSheet(
    uiState: ViewerUiState,
    onDismiss: () -> Unit,
    onSelectTab: (ProToolTab) -> Unit,
    onCompress: (removeUnused: Boolean, recompress: Boolean, stripMetadata: Boolean) -> Unit,
    onApplyWatermark: (text: String, fontSize: Float, rotationDeg: Float, opacity: Float, r: Int, g: Int, b: Int, onTop: Boolean) -> Unit,
    onApplyPageNumbers: (pattern: String, position: Int, fontSize: Float, margin: Float) -> Unit,
    onApplyBatesStamp: (prefix: String, startNumber: Int, paddingDigits: Int, suffix: String, position: Int) -> Unit,
    onApplyRedaction: (redactionsJson: String) -> Unit,
    onFillFormFields: (fieldValuesJson: String, flatten: Boolean) -> Unit,
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
                    text = stringResource(R.string.pro_tools_title),
                    style = MaterialTheme.typography.titleLarge,
                    fontWeight = FontWeight.Bold
                )
                IconButton(onClick = onDismiss) {
                    Icon(Icons.Default.Close, contentDescription = stringResource(R.string.action_close))
                }
            }

            // Tab navigation
            val tabs = ProToolTab.entries
            PrimaryScrollableTabRow(
                selectedTabIndex = tabs.indexOf(uiState.activeProTool),
                edgePadding = 16.dp,
                modifier = Modifier.fillMaxWidth()
            ) {
                tabs.forEach { tab ->
                    val isSelected = uiState.activeProTool == tab
                    Tab(
                        selected = isSelected,
                        onClick = { onSelectTab(tab) },
                        text = { Text(stringResource(tab.titleRes)) },
                        icon = { Icon(tab.icon, contentDescription = null) }
                    )
                }
            }

            // Active Tool Content Area
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(horizontal = 20.dp, vertical = 16.dp)
            ) {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .verticalScroll(rememberScrollState()),
                    verticalArrangement = Arrangement.spacedBy(16.dp)
                ) {
                    when (uiState.activeProTool) {
                        ProToolTab.COMPRESS -> CompressToolContent(
                            stats = uiState.compressionStats,
                            isProcessing = uiState.isProcessingProTool,
                            onExecute = onCompress
                        )
                        ProToolTab.WATERMARK -> WatermarkToolContent(
                            isProcessing = uiState.isProcessingProTool,
                            onExecute = onApplyWatermark
                        )
                        ProToolTab.PAGE_NUMBERS -> PageNumbersToolContent(
                            isProcessing = uiState.isProcessingProTool,
                            onExecute = onApplyPageNumbers
                        )
                        ProToolTab.BATES_STAMP -> BatesStampToolContent(
                            isProcessing = uiState.isProcessingProTool,
                            onExecute = onApplyBatesStamp
                        )
                        ProToolTab.REDACT -> RedactToolContent(
                            isProcessing = uiState.isProcessingProTool,
                            onExecute = onApplyRedaction
                        )
                        ProToolTab.FORM_FIELDS -> FormFieldsToolContent(
                            fields = uiState.formFields,
                            isProcessing = uiState.isProcessingProTool,
                            onExecute = onFillFormFields
                        )
                    }

                    // Status feedback banner
                    uiState.proToolMessage?.let { msg ->
                        val isSuccess = msg == "success"
                        Card(
                            colors = CardDefaults.cardColors(
                                containerColor = if (isSuccess) MaterialTheme.colorScheme.primaryContainer else MaterialTheme.colorScheme.errorContainer
                            ),
                            shape = RoundedCornerShape(12.dp),
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Text(
                                text = if (isSuccess) stringResource(R.string.status_pro_tool_success)
                                else stringResource(R.string.status_pro_tool_failed, msg),
                                modifier = Modifier.padding(14.dp),
                                style = MaterialTheme.typography.bodyMedium,
                                color = if (isSuccess) MaterialTheme.colorScheme.onPrimaryContainer else MaterialTheme.colorScheme.onErrorContainer
                            )
                        }
                    }
                }

                // Loading overlay
                if (uiState.isProcessingProTool) {
                    Box(
                        modifier = Modifier
                            .matchParentSize()
                            .background(MaterialTheme.colorScheme.surface.copy(alpha = 0.75f)),
                        contentAlignment = Alignment.Center
                    ) {
                        Row(
                            horizontalArrangement = Arrangement.spacedBy(12.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            CircularProgressIndicator(modifier = Modifier.size(24.dp))
                            Text(
                                text = stringResource(R.string.status_processing_pro_tool),
                                style = MaterialTheme.typography.bodyMedium
                            )
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tool Tab Contents
// ---------------------------------------------------------------------------

@Composable
private fun CompressToolContent(
    stats: CompressionStats?,
    isProcessing: Boolean,
    onExecute: (Boolean, Boolean, Boolean) -> Unit
) {
    var removeUnused by remember { mutableStateOf(true) }
    var recompress by remember { mutableStateOf(true) }
    var stripMetadata by remember { mutableStateOf(false) }

    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Text(stringResource(R.string.compress_remove_unused))
        Switch(checked = removeUnused, onCheckedChange = { removeUnused = it })
    }

    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Text(stringResource(R.string.compress_recompress_images))
        Switch(checked = recompress, onCheckedChange = { recompress = it })
    }

    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Text(stringResource(R.string.compress_strip_metadata))
        Switch(checked = stripMetadata, onCheckedChange = { stripMetadata = it })
    }

    Button(
        onClick = { onExecute(removeUnused, recompress, stripMetadata) },
        enabled = !isProcessing,
        modifier = Modifier.fillMaxWidth()
    ) {
        Text(stringResource(R.string.compress_action))
    }

    stats?.let { s ->
        Card(
            shape = RoundedCornerShape(12.dp),
            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant),
            modifier = Modifier.fillMaxWidth()
        ) {
            val origFmt = formatFileSize(s.originalSize)
            val compFmt = formatFileSize(s.compressedSize)
            val saveFmt = formatFileSize(s.savingsBytes)
            Text(
                text = stringResource(R.string.compress_savings_report, origFmt, compFmt, saveFmt, s.savingsPercentage),
                modifier = Modifier.padding(16.dp),
                style = MaterialTheme.typography.bodyMedium
            )
        }
    }
}

@Composable
private fun WatermarkToolContent(
    isProcessing: Boolean,
    onExecute: (String, Float, Float, Float, Int, Int, Int, Boolean) -> Unit
) {
    val defaultText = stringResource(R.string.watermark_text_default)
    var text by remember { mutableStateOf(defaultText) }
    var opacity by remember { mutableFloatStateOf(0.3f) }
    var rotation by remember { mutableFloatStateOf(45f) }
    var fontSize by remember { mutableFloatStateOf(36f) }
    var onTop by remember { mutableStateOf(true) }

    OutlinedTextField(
        value = text,
        onValueChange = { text = it },
        label = { Text(stringResource(R.string.watermark_text_label)) },
        modifier = Modifier.fillMaxWidth(),
        singleLine = true
    )

    Column {
        Text(stringResource(R.string.watermark_opacity, (opacity * 100).toInt()))
        Slider(value = opacity, onValueChange = { opacity = it }, valueRange = 0.05f..1.0f)
    }

    Column {
        Text(stringResource(R.string.watermark_rotation, rotation.toInt()))
        Slider(value = rotation, onValueChange = { rotation = it }, valueRange = 0f..360f)
    }

    Column {
        Text(stringResource(R.string.watermark_font_size, fontSize.toInt()))
        Slider(value = fontSize, onValueChange = { fontSize = it }, valueRange = 12f..72f)
    }

    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Text(stringResource(R.string.watermark_on_top))
        Switch(checked = onTop, onCheckedChange = { onTop = it })
    }

    Button(
        onClick = { onExecute(text, fontSize, rotation, opacity, 128, 128, 128, onTop) },
        enabled = !isProcessing && text.isNotBlank(),
        modifier = Modifier.fillMaxWidth()
    ) {
        Text(stringResource(R.string.watermark_apply))
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun PageNumbersToolContent(
    isProcessing: Boolean,
    onExecute: (String, Int, Float, Float) -> Unit
) {
    val defaultPattern = stringResource(R.string.stamp_pattern_default)
    var pattern by remember { mutableStateOf(defaultPattern) }
    var selectedPosition by remember { mutableStateOf(StampPosition.BOTTOM_CENTER) }
    var fontSize by remember { mutableFloatStateOf(10f) }
    var margin by remember { mutableFloatStateOf(36f) }

    OutlinedTextField(
        value = pattern,
        onValueChange = { pattern = it },
        label = { Text(stringResource(R.string.stamp_pattern_label)) },
        modifier = Modifier.fillMaxWidth(),
        singleLine = true
    )

    PositionDropdown(
        selected = selectedPosition,
        onSelected = { selectedPosition = it }
    )

    Column {
        Text(stringResource(R.string.watermark_font_size, fontSize.toInt()))
        Slider(value = fontSize, onValueChange = { fontSize = it }, valueRange = 8f..24f)
    }

    Column {
        Text(stringResource(R.string.stamp_margin_label, margin.toInt()))
        Slider(value = margin, onValueChange = { margin = it }, valueRange = 10f..72f)
    }

    Button(
        onClick = { onExecute(pattern, selectedPosition.value, fontSize, margin) },
        enabled = !isProcessing && pattern.isNotBlank(),
        modifier = Modifier.fillMaxWidth()
    ) {
        Text(stringResource(R.string.stamp_apply_numbers))
    }
}

@Composable
private fun BatesStampToolContent(
    isProcessing: Boolean,
    onExecute: (String, Int, Int, String, Int) -> Unit
) {
    var prefix by remember { mutableStateOf("RBD-") }
    var startNum by remember { mutableIntStateOf(1) }
    var paddingDigits by remember { mutableIntStateOf(6) }
    var suffix by remember { mutableStateOf("") }
    var selectedPosition by remember { mutableStateOf(StampPosition.BOTTOM_RIGHT) }

    Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        OutlinedTextField(
            value = prefix,
            onValueChange = { prefix = it },
            label = { Text(stringResource(R.string.bates_prefix_label)) },
            modifier = Modifier.weight(1f),
            singleLine = true
        )
        OutlinedTextField(
            value = suffix,
            onValueChange = { suffix = it },
            label = { Text(stringResource(R.string.bates_suffix_label)) },
            modifier = Modifier.weight(1f),
            singleLine = true
        )
    }

    Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        OutlinedTextField(
            value = startNum.toString(),
            onValueChange = { startNum = it.toIntOrNull() ?: 1 },
            label = { Text(stringResource(R.string.bates_start_num_label)) },
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
            modifier = Modifier.weight(1f),
            singleLine = true
        )
        OutlinedTextField(
            value = paddingDigits.toString(),
            onValueChange = { paddingDigits = it.toIntOrNull() ?: 6 },
            label = { Text(stringResource(R.string.bates_padding_label)) },
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
            modifier = Modifier.weight(1f),
            singleLine = true
        )
    }

    PositionDropdown(
        selected = selectedPosition,
        onSelected = { selectedPosition = it }
    )

    Button(
        onClick = { onExecute(prefix, startNum, paddingDigits, suffix, selectedPosition.value) },
        enabled = !isProcessing,
        modifier = Modifier.fillMaxWidth()
    ) {
        Text(stringResource(R.string.stamp_apply_bates))
    }
}

@Composable
private fun RedactToolContent(
    isProcessing: Boolean,
    onExecute: (String) -> Unit
) {
    val sampleJson = """[{"page_index":0,"rects":[{"x0":50.0,"y0":650.0,"x1":300.0,"y1":750.0}],"fill_color":[0,0,0]}]"""
    var json by remember { mutableStateOf(sampleJson) }

    Text(
        text = stringResource(R.string.redact_description),
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant
    )

    OutlinedTextField(
        value = json,
        onValueChange = { json = it },
        label = { Text(stringResource(R.string.redact_json_label)) },
        modifier = Modifier.fillMaxWidth(),
        maxLines = 6
    )

    Button(
        onClick = { onExecute(json) },
        enabled = !isProcessing && json.isNotBlank(),
        modifier = Modifier.fillMaxWidth()
    ) {
        Text(stringResource(R.string.redact_apply))
    }
}

@Composable
private fun FormFieldsToolContent(
    fields: List<FormFieldInfo>,
    isProcessing: Boolean,
    onExecute: (String, Boolean) -> Unit
) {
    var flatten by remember { mutableStateOf(false) }
    val fieldValues = remember { mutableStateMapOf<String, String>() }

    // Initialize values map
    fields.forEach { f ->
        if (!fieldValues.containsKey(f.name)) {
            fieldValues[f.name] = f.value
        }
    }

    if (fields.isEmpty()) {
        Text(
            text = stringResource(R.string.forms_no_fields),
            style = MaterialTheme.typography.bodyMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant
        )
    } else {
        fields.forEach { field ->
            OutlinedTextField(
                value = fieldValues[field.name] ?: "",
                onValueChange = { fieldValues[field.name] = it },
                label = { Text("${field.name} (${field.fieldType})") },
                enabled = !field.readOnly,
                modifier = Modifier.fillMaxWidth(),
                singleLine = true
            )
        }

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Text(stringResource(R.string.forms_flatten_label))
            Switch(checked = flatten, onCheckedChange = { flatten = it })
        }

        Button(
            onClick = {
                // Build JSON dictionary: {"field_name": "value", ...}
                val entries = fieldValues.entries.joinToString(",") { "\"${it.key}\":\"${it.value}\"" }
                val json = "{$entries}"
                onExecute(json, flatten)
            },
            enabled = !isProcessing,
            modifier = Modifier.fillMaxWidth()
        ) {
            Text(stringResource(R.string.forms_apply))
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun PositionDropdown(
    selected: StampPosition,
    onSelected: (StampPosition) -> Unit
) {
    var expanded by remember { mutableStateOf(false) }
    val options = StampPosition.entries

    ExposedDropdownMenuBox(
        expanded = expanded,
        onExpandedChange = { expanded = !expanded }
    ) {
        OutlinedTextField(
            value = stringResource(selected.titleRes),
            onValueChange = {},
            readOnly = true,
            label = { Text(stringResource(R.string.stamp_position_label)) },
            trailingIcon = { ExposedDropdownMenuDefaults.TrailingIcon(expanded = expanded) },
            modifier = Modifier
                .menuAnchor(MenuAnchorType.PrimaryNotEditable)
                .fillMaxWidth()
        )

        ExposedDropdownMenu(
            expanded = expanded,
            onDismissRequest = { expanded = false }
        ) {
            options.forEach { pos ->
                DropdownMenuItem(
                    text = { Text(stringResource(pos.titleRes)) },
                    onClick = {
                        onSelected(pos)
                        expanded = false
                    }
                )
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers and Enum Mappings
// ---------------------------------------------------------------------------

private val ProToolTab.titleRes: Int
    get() = when (this) {
        ProToolTab.COMPRESS -> R.string.pro_tool_compress
        ProToolTab.WATERMARK -> R.string.pro_tool_watermark
        ProToolTab.PAGE_NUMBERS -> R.string.pro_tool_page_numbers
        ProToolTab.BATES_STAMP -> R.string.pro_tool_bates_stamp
        ProToolTab.REDACT -> R.string.pro_tool_redact
        ProToolTab.FORM_FIELDS -> R.string.pro_tool_forms
    }

private val ProToolTab.icon: ImageVector
    get() = when (this) {
        ProToolTab.COMPRESS -> Icons.Default.Compress
        ProToolTab.WATERMARK -> Icons.Default.WaterDrop
        ProToolTab.PAGE_NUMBERS -> Icons.Default.FormatListNumbered
        ProToolTab.BATES_STAMP -> Icons.Default.Gavel
        ProToolTab.REDACT -> Icons.Default.Security
        ProToolTab.FORM_FIELDS -> Icons.Default.TextFields
    }

private val StampPosition.titleRes: Int
    get() = when (this) {
        StampPosition.TOP_LEFT -> R.string.pos_top_left
        StampPosition.TOP_CENTER -> R.string.pos_top_center
        StampPosition.TOP_RIGHT -> R.string.pos_top_right
        StampPosition.BOTTOM_LEFT -> R.string.pos_bottom_left
        StampPosition.BOTTOM_CENTER -> R.string.pos_bottom_center
        StampPosition.BOTTOM_RIGHT -> R.string.pos_bottom_right
    }

private fun formatFileSize(bytes: Long): String {
    if (bytes < 1024) return "$bytes B"
    val kb = bytes / 1024.0
    if (kb < 1024) return String.format(java.util.Locale.US, "%.1f KB", kb)
    val mb = kb / 1024.0
    return String.format(java.util.Locale.US, "%.1f MB", mb)
}
