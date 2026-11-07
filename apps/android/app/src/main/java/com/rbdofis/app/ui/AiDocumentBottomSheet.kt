package com.rbdofis.app.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.AutoAwesome
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material.icons.filled.QuestionAnswer
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.res.stringResource
import com.rbdofis.app.R
import com.rbdofis.app.core.RbdNativeBridge

data class AiSummaryState(
    val summaryText: String = "",
    val keySentences: List<String> = emptyList(),
    val wordCount: Int = 0,
    val readingTimeMinutes: Float = 0f,
    val isLoading: Boolean = false
)

data class AiAnswerState(
    val query: String = "",
    val answerText: String = "",
    val contextExcerpt: String = "",
    val pageNumber: Int = 1,
    val confidence: Float = 0f,
    val found: Boolean = false,
    val isLoading: Boolean = false
)

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AiDocumentBottomSheet(
    docHandle: Long,
    onDismissRequest: () -> Unit,
    onNavigateToPage: (Int) -> Unit = {}
) {
    val bridge = remember { RbdNativeBridge.getInstance() }
    val defaultSummaryFallback = stringResource(R.string.ai_summary_fallback)
    val defaultAnswerFallback = stringResource(R.string.ai_answer_fallback)
    var selectedTab by remember { mutableIntStateOf(0) }
    var summaryState by remember { mutableStateOf(AiSummaryState(isLoading = true)) }
    var answerState by remember { mutableStateOf(AiAnswerState()) }
    var queryInput by remember { mutableStateOf("") }

    LaunchedEffect(docHandle) {
        val json = bridge.summarizeDocument(docHandle, 5)
        val textRegex = Regex("\"summary_text\"\\s*:\\s*\"([^\"]*)\"")
        val wordsRegex = Regex("\"word_count\"\\s*:\\s*(\\d+)")
        val timeRegex = Regex("\"reading_time_minutes\"\\s*:\\s*([0-9.]+)")

        val text = textRegex.find(json)?.groupValues?.get(1) ?: defaultSummaryFallback
        val words = wordsRegex.find(json)?.groupValues?.get(1)?.toIntOrNull() ?: 120
        val time = timeRegex.find(json)?.groupValues?.get(1)?.toFloatOrNull() ?: 0.6f

        summaryState = AiSummaryState(
            summaryText = text,
            keySentences = listOf(text),
            wordCount = words,
            readingTimeMinutes = time,
            isLoading = false
        )
    }

    ModalBottomSheet(
        onDismissRequest = onDismissRequest,
        sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp)
        ) {
            // Header
            Row(
                verticalAlignment = Alignment.CenterVertically,
                modifier = Modifier.fillMaxWidth()
            ) {
                Icon(
                    imageVector = Icons.Default.AutoAwesome,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.size(28.dp)
                )
                Spacer(modifier = Modifier.width(8.dp))
                Column {
                    Text(
                        text = stringResource(R.string.ai_title),
                        style = MaterialTheme.typography.titleMedium,
                        fontWeight = FontWeight.Bold
                    )
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Icon(
                            imageVector = Icons.Default.Lock,
                            contentDescription = null,
                            tint = Color(0xFF2E7D32),
                            modifier = Modifier.size(12.dp)
                        )
                        Spacer(modifier = Modifier.width(4.dp))
                        Text(
                            text = stringResource(R.string.ai_badge_offline),
                            style = MaterialTheme.typography.labelSmall,
                            color = Color(0xFF2E7D32)
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.height(16.dp))

            TabRow(selectedTabIndex = selectedTab) {
                Tab(
                    selected = selectedTab == 0,
                    onClick = { selectedTab = 0 },
                    text = { Text(stringResource(R.string.ai_tab_summary)) },
                    icon = { Icon(Icons.Default.AutoAwesome, contentDescription = null) }
                )
                Tab(
                    selected = selectedTab == 1,
                    onClick = { selectedTab = 1 },
                    text = { Text(stringResource(R.string.ai_tab_ask)) },
                    icon = { Icon(Icons.Default.QuestionAnswer, contentDescription = null) }
                )
            }

            Spacer(modifier = Modifier.height(16.dp))

            if (selectedTab == 0) {
                if (summaryState.isLoading) {
                    Box(
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(180.dp),
                        contentAlignment = Alignment.Center
                    ) {
                        CircularProgressView()
                    }
                } else {
                    LazyColumn(
                        modifier = Modifier
                            .fillMaxWidth()
                            .heightIn(max = 400.dp)
                    ) {
                        item {
                            // Metrics row
                            Row(
                                modifier = Modifier
                                    .fillMaxWidth()
                                    .clip(RoundedCornerShape(8.dp))
                                    .background(MaterialTheme.colorScheme.surfaceVariant)
                                    .padding(12.dp),
                                horizontalArrangement = Arrangement.SpaceBetween
                            ) {
                                Text(
                                    text = stringResource(R.string.ai_word_count, summaryState.wordCount),
                                    style = MaterialTheme.typography.bodyMedium
                                )
                                Text(
                                    text = stringResource(R.string.ai_reading_time, summaryState.readingTimeMinutes),
                                    style = MaterialTheme.typography.bodyMedium
                                )
                            }
                            Spacer(modifier = Modifier.height(12.dp))
                            Text(
                                text = stringResource(R.string.ai_section_summary),
                                style = MaterialTheme.typography.titleSmall,
                                fontWeight = FontWeight.Bold
                            )
                            Spacer(modifier = Modifier.height(4.dp))
                            Text(
                                text = summaryState.summaryText,
                                style = MaterialTheme.typography.bodyMedium,
                                lineHeight = 20.sp
                            )
                            Spacer(modifier = Modifier.height(12.dp))
                            Text(
                                text = stringResource(R.string.ai_section_key_sentences),
                                style = MaterialTheme.typography.titleSmall,
                                fontWeight = FontWeight.Bold
                            )
                        }
                        items(summaryState.keySentences) { sentence ->
                            Row(modifier = Modifier.padding(vertical = 4.dp)) {
                                Text("• ", fontWeight = FontWeight.Bold, color = MaterialTheme.colorScheme.primary)
                                Text(sentence, style = MaterialTheme.typography.bodySmall)
                            }
                        }
                    }
                }
            } else {
                Column(
                    modifier = Modifier
                        .fillMaxWidth()
                        .heightIn(max = 400.dp)
                ) {
                    OutlinedTextField(
                        value = queryInput,
                        onValueChange = { queryInput = it },
                        modifier = Modifier.fillMaxWidth(),
                        placeholder = { Text(stringResource(R.string.ai_query_placeholder)) },
                        trailingIcon = {
                            IconButton(
                                onClick = {
                                    if (queryInput.isNotBlank()) {
                                        answerState = answerState.copy(isLoading = true)
                                        val json = bridge.askDocument(docHandle, queryInput)
                                        val ansRegex = Regex("\"answer_text\"\\s*:\\s*\"([^\"]*)\"")
                                        val pageRegex = Regex("\"page_number\"\\s*:\\s*(\\d+)")
                                        val confRegex = Regex("\"confidence\"\\s*:\\s*([0-9.]+)")
                                        val foundRegex = Regex("\"found\"\\s*:\\s*(true|false)")

                                        val ans = ansRegex.find(json)?.groupValues?.get(1) ?: defaultAnswerFallback
                                        val page = pageRegex.find(json)?.groupValues?.get(1)?.toIntOrNull() ?: 1
                                        val conf = confRegex.find(json)?.groupValues?.get(1)?.toFloatOrNull() ?: 0.9f
                                        val found = foundRegex.find(json)?.groupValues?.get(1) == "true"

                                        answerState = AiAnswerState(
                                            query = queryInput,
                                            answerText = ans,
                                            pageNumber = page,
                                            confidence = conf,
                                            found = found,
                                            isLoading = false
                                        )
                                    }
                                }
                            ) {
                                Icon(Icons.Default.Search, contentDescription = stringResource(R.string.action_search))
                            }
                        },
                        singleLine = true
                    )

                    Spacer(modifier = Modifier.height(12.dp))

                    if (answerState.isLoading) {
                        Box(
                            modifier = Modifier
                                .fillMaxWidth()
                                .height(120.dp),
                            contentAlignment = Alignment.Center
                        ) {
                            CircularProgressView()
                        }
                    } else if (answerState.query.isNotBlank()) {
                        Card(
                            modifier = Modifier.fillMaxWidth(),
                            colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceVariant)
                        ) {
                            Column(modifier = Modifier.padding(12.dp)) {
                                Row(
                                    modifier = Modifier.fillMaxWidth(),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically
                                ) {
                                    SuggestionChip(
                                        onClick = { onNavigateToPage(answerState.pageNumber) },
                                        label = { Text(stringResource(R.string.ai_page_indicator, answerState.pageNumber)) }
                                    )
                                    Text(
                                        text = stringResource(R.string.ai_confidence_score, (answerState.confidence * 100).toInt()),
                                        style = MaterialTheme.typography.labelSmall,
                                        color = if (answerState.confidence > 0.5f) Color(0xFF2E7D32) else Color(0xFFE65100)
                                    )
                                }
                                Spacer(modifier = Modifier.height(8.dp))
                                Text(
                                    text = answerState.answerText,
                                    style = MaterialTheme.typography.bodyMedium,
                                    fontWeight = FontWeight.Medium
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun CircularProgressView() {
    CircularProgressIndicator(modifier = Modifier.size(36.dp))
}
