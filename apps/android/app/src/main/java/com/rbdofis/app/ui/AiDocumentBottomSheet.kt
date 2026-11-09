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
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import org.json.JSONObject

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
    val coroutineScope = rememberCoroutineScope()
    val bridge = remember { RbdNativeBridge.getInstance() }
    val defaultSummaryFallback = stringResource(R.string.ai_summary_fallback)
    val defaultAnswerFallback = stringResource(R.string.ai_answer_fallback)
    var selectedTab by remember { mutableIntStateOf(0) }
    var summaryState by remember { mutableStateOf(AiSummaryState(isLoading = true)) }
    var answerState by remember { mutableStateOf(AiAnswerState()) }
    var queryInput by remember { mutableStateOf("") }

    LaunchedEffect(docHandle) {
        summaryState = summaryState.copy(isLoading = true)
        val parsed = withContext(Dispatchers.IO) {
            try {
                val json = bridge.summarizeDocument(docHandle, 5)
                val obj = JSONObject(json)
                val text = obj.optString("summary_text", defaultSummaryFallback)
                val words = obj.optInt("word_count", 120)
                val time = obj.optDouble("reading_time_minutes", 0.6).toFloat()
                val sentences = mutableListOf<String>()
                val arr = obj.optJSONArray("key_sentences")
                if (arr != null) {
                    for (i in 0 until arr.length()) {
                        sentences.add(arr.optString(i))
                    }
                }
                if (sentences.isEmpty()) {
                    sentences.add(text)
                }
                AiSummaryState(
                    summaryText = text,
                    keySentences = sentences,
                    wordCount = words,
                    readingTimeMinutes = time,
                    isLoading = false
                )
            } catch (e: Exception) {
                AiSummaryState(
                    summaryText = defaultSummaryFallback,
                    keySentences = listOf(defaultSummaryFallback),
                    wordCount = 120,
                    readingTimeMinutes = 0.6f,
                    isLoading = false
                )
            }
        }
        summaryState = parsed
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
                                        coroutineScope.launch {
                                            answerState = answerState.copy(isLoading = true)
                                            val parsed = withContext(Dispatchers.IO) {
                                                try {
                                                    val json = bridge.askDocument(docHandle, queryInput)
                                                    val obj = JSONObject(json)
                                                    val ans = obj.optString("answer_text", defaultAnswerFallback)
                                                    val page = obj.optInt("page_number", 1)
                                                    val conf = obj.optDouble("confidence", 0.9).toFloat()
                                                    val found = obj.optBoolean("found", false)
                                                    val excerpt = obj.optString("context_excerpt", "")
                                                    AiAnswerState(
                                                        query = queryInput,
                                                        answerText = ans,
                                                        contextExcerpt = excerpt,
                                                        pageNumber = page,
                                                        confidence = conf,
                                                        found = found,
                                                        isLoading = false
                                                    )
                                                } catch (e: Exception) {
                                                    AiAnswerState(
                                                        query = queryInput,
                                                        answerText = defaultAnswerFallback,
                                                        pageNumber = 1,
                                                        confidence = 0.0f,
                                                        found = false,
                                                        isLoading = false
                                                    )
                                                }
                                            }
                                            answerState = parsed
                                        }
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
