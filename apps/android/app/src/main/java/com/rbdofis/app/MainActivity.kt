package com.rbdofis.app

import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.provider.OpenableColumns
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.core.content.FileProvider
import androidx.lifecycle.lifecycleScope
import com.rbdofis.app.data.PdfDocumentRepository
import com.rbdofis.app.ui.organizer.PageOrganizerScreen
import com.rbdofis.app.ui.organizer.PageOrganizerViewModel
import com.rbdofis.app.ui.scanner.DocumentScannerScreen
import com.rbdofis.app.ui.scanner.DocumentScannerViewModel
import com.rbdofis.app.ui.theme.RbdOfisTheme
import com.rbdofis.app.ui.viewer.PdfViewerScreen
import com.rbdofis.app.ui.viewer.PdfViewerViewModel
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.File
import java.io.FileOutputStream

enum class AppScreen {
    VIEWER,
    ORGANIZER,
    SCANNER
}

class MainActivity : ComponentActivity() {

    private lateinit var repository: PdfDocumentRepository
    private lateinit var viewerViewModel: PdfViewerViewModel
    private lateinit var organizerViewModel: PageOrganizerViewModel
    private lateinit var scannerViewModel: DocumentScannerViewModel

    // File picker launcher
    private val openDocumentLauncher = registerForActivityResult(
        ActivityResultContracts.OpenDocument()
    ) { uri: Uri? ->
        uri?.let { handleIncomingUri(it) }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        // Initialize Architecture components
        repository = PdfDocumentRepository()
        viewerViewModel = PdfViewerViewModel(repository)
        organizerViewModel = PageOrganizerViewModel(repository)
        scannerViewModel = DocumentScannerViewModel()

        // Process incoming intent (e.g. from file manager or other apps)
        handleIntent(intent)

        setContent {
            RbdOfisTheme {
                MainAppNavHost()
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        handleIntent(intent)
    }

    private fun handleIntent(intent: Intent?) {
        if (intent == null) return
        val action = intent.action
        val data: Uri? = intent.data

        if (Intent.ACTION_VIEW == action && data != null) {
            handleIncomingUri(data)
        }
    }

    private fun handleIncomingUri(uri: Uri) {
        lifecycleScope.launch {
            try {
                when (uri.scheme) {
                    "file" -> {
                        val path = uri.path ?: return@launch
                        repository.openDocument(path)
                    }
                    "content" -> {
                        val displayName = getFileNameFromUri(uri) ?: "document.pdf"
                        val cachedFile = copyContentUriToCache(uri, displayName)
                        if (cachedFile != null) {
                            repository.openDocument(cachedFile.absolutePath)
                        } else {
                            Toast.makeText(this@MainActivity, R.string.error_open_document, Toast.LENGTH_SHORT).show()
                        }
                    }
                    else -> {
                        Toast.makeText(this@MainActivity, R.string.error_open_document, Toast.LENGTH_SHORT).show()
                    }
                }
            } catch (e: Exception) {
                Toast.makeText(this@MainActivity, R.string.error_open_document, Toast.LENGTH_SHORT).show()
            }
        }
    }

    private fun getFileNameFromUri(uri: Uri): String? {
        var name: String? = null
        if (uri.scheme == "content") {
            contentResolver.query(uri, null, null, null, null)?.use { cursor ->
                val nameIndex = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
                if (nameIndex != -1 && cursor.moveToFirst()) {
                    name = cursor.getString(nameIndex)
                }
            }
        }
        return name ?: uri.lastPathSegment
    }

    private suspend fun copyContentUriToCache(uri: Uri, displayName: String): File? = withContext(Dispatchers.IO) {
        try {
            val cacheDir = File(cacheDir, "opened_documents")
            cacheDir.mkdirs()
            val targetFile = File(cacheDir, displayName)

            contentResolver.openInputStream(uri)?.use { inputStream ->
                FileOutputStream(targetFile).use { outputStream ->
                    inputStream.copyTo(outputStream)
                }
            }
            targetFile
        } catch (e: Exception) {
            null
        }
    }

    private fun shareCurrentDocument() {
        val doc = repository.documentState.value
        if (!doc.isOpen || doc.filePath.isBlank()) return

        val file = File(doc.filePath)
        if (!file.exists()) return

        try {
            val contentUri = FileProvider.getUriForFile(
                this,
                "${applicationContext.packageName}.fileprovider",
                file
            )

            val shareIntent = Intent(Intent.ACTION_SEND).apply {
                type = "application/pdf"
                putExtra(Intent.EXTRA_STREAM, contentUri)
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            }
            startActivity(Intent.createChooser(shareIntent, getString(R.string.action_share)))
        } catch (e: Exception) {
            Toast.makeText(this, R.string.error_save_document, Toast.LENGTH_SHORT).show()
        }
    }

    @Composable
    private fun MainAppNavHost() {
        var currentScreen by remember { mutableStateOf(AppScreen.VIEWER) }

        when (currentScreen) {
            AppScreen.VIEWER -> {
                PdfViewerScreen(
                    viewModel = viewerViewModel,
                    onNavigateToOrganizer = { currentScreen = AppScreen.ORGANIZER },
                    onScanDocumentRequested = { currentScreen = AppScreen.SCANNER },
                    onShareRequested = { shareCurrentDocument() },
                    modifier = Modifier.fillMaxSize()
                )
            }
            AppScreen.ORGANIZER -> {
                PageOrganizerScreen(
                    viewModel = organizerViewModel,
                    onNavigateBack = { currentScreen = AppScreen.VIEWER },
                    modifier = Modifier.fillMaxSize()
                )
            }
            AppScreen.SCANNER -> {
                DocumentScannerScreen(
                    viewModel = scannerViewModel,
                    onNavigateBack = { currentScreen = AppScreen.VIEWER },
                    onScanComplete = { scannedPdfPath ->
                        lifecycleScope.launch {
                            repository.openDocument(scannedPdfPath)
                            currentScreen = AppScreen.VIEWER
                        }
                    },
                    modifier = Modifier.fillMaxSize()
                )
            }
        }
    }
}
