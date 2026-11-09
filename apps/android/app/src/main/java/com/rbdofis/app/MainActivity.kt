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
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.core.content.FileProvider
import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import androidx.lifecycle.lifecycleScope
import com.rbdofis.app.data.PdfDocumentRepository
import com.rbdofis.app.ui.office.OfficeEditorScreen
import com.rbdofis.app.ui.organizer.PageOrganizerScreen
import com.rbdofis.app.ui.organizer.PageOrganizerViewModel
import com.rbdofis.app.ui.scanner.DocumentScannerScreen
import com.rbdofis.app.ui.scanner.DocumentScannerViewModel
import com.rbdofis.app.ui.theme.RbdOfisTheme
import com.rbdofis.app.ui.viewer.PdfViewerScreen
import com.rbdofis.app.ui.viewer.PdfViewerViewModel
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.File
import java.io.FileOutputStream
import java.util.UUID

enum class AppScreen {
    VIEWER,
    ORGANIZER,
    SCANNER,
    OFFICE_EDITOR
}

sealed class NavigationEvent {
    data class OpenOffice(val filePath: String) : NavigationEvent()
    data class OpenPdf(val filePath: String) : NavigationEvent()
}

class PdfViewModelFactory(
    private val repositoryProvider: () -> PdfDocumentRepository
) : ViewModelProvider.Factory {
    private var repository: PdfDocumentRepository? = null

    fun getRepository(): PdfDocumentRepository {
        return repository ?: repositoryProvider().also { repository = it }
    }

    @Suppress("UNCHECKED_CAST")
    override fun <T : ViewModel> create(modelClass: Class<T>): T {
        val repo = getRepository()
        return when {
            modelClass.isAssignableFrom(PdfViewerViewModel::class.java) -> {
                PdfViewerViewModel(repo) as T
            }
            modelClass.isAssignableFrom(PageOrganizerViewModel::class.java) -> {
                PageOrganizerViewModel(repo) as T
            }
            modelClass.isAssignableFrom(DocumentScannerViewModel::class.java) -> {
                DocumentScannerViewModel() as T
            }
            else -> throw IllegalArgumentException("Unknown ViewModel class: ${modelClass.name}")
        }
    }
}

class MainActivity : ComponentActivity() {

    private lateinit var repository: PdfDocumentRepository
    private lateinit var viewerViewModel: PdfViewerViewModel
    private lateinit var organizerViewModel: PageOrganizerViewModel
    private lateinit var scannerViewModel: DocumentScannerViewModel

    private val navEvents = MutableSharedFlow<NavigationEvent>(extraBufferCapacity = 16)

    // File picker launcher
    private val openDocumentLauncher = registerForActivityResult(
        ActivityResultContracts.OpenDocument()
    ) { uri: Uri? ->
        uri?.let { handleIncomingUri(it) }
    }

    // Scoped Storage (SAF) Create Document launcher for PDF export
    private val exportDocumentLauncher = registerForActivityResult(
        ActivityResultContracts.CreateDocument("application/pdf")
    ) { uri: Uri? ->
        uri?.let { exportCurrentDocumentToUri(it) }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        // Bind ViewModels to Activity scope via ViewModelProvider and custom Factory
        // If activity is recreated on configuration change, ViewModelStore retains existing ViewModels
        val factory = PdfViewModelFactory { PdfDocumentRepository() }
        viewerViewModel = ViewModelProvider(this, factory)[PdfViewerViewModel::class.java]
        // Preserve and share the exact repository instance attached to viewerViewModel
        repository = viewerViewModel.repository
        organizerViewModel = ViewModelProvider(this, PdfViewModelFactory { repository })[PageOrganizerViewModel::class.java]
        scannerViewModel = ViewModelProvider(this, factory)[DocumentScannerViewModel::class.java]

        // Process incoming intent only on initial creation
        if (savedInstanceState == null) {
            handleIntent(intent)
        }

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
            handleIncomingUri(data, intent.type)
        }
    }

    private fun isOfficeFile(uri: Uri, mimeType: String?): Boolean {
        val type = mimeType ?: contentResolver.getType(uri)
        if (type != null) {
            val lower = type.lowercase()
            if (lower.contains("officedocument") ||
                lower.contains("msword") ||
                lower.contains("ms-excel") ||
                lower.contains("ms-powerpoint") ||
                lower.contains("wordprocessingml") ||
                lower.contains("spreadsheetml") ||
                lower.contains("presentationml")
            ) {
                return true
            }
        }
        val name = (getFileNameFromUri(uri) ?: uri.path ?: "").lowercase()
        return name.endsWith(".docx") || name.endsWith(".xlsx") || name.endsWith(".pptx") ||
               name.endsWith(".doc") || name.endsWith(".xls") || name.endsWith(".ppt")
    }

    private fun handleIncomingUri(uri: Uri, mimeType: String? = null) {
        val isOffice = isOfficeFile(uri, mimeType)
        lifecycleScope.launch {
            try {
                val localPath = when (uri.scheme) {
                    "file" -> uri.path
                    "content" -> {
                        val displayName = getFileNameFromUri(uri) ?: if (isOffice) "document.docx" else "document.pdf"
                        copyContentUriToCache(uri, displayName)?.absolutePath
                    }
                    else -> null
                }

                if (localPath == null) {
                    Toast.makeText(this@MainActivity, R.string.error_open_document, Toast.LENGTH_SHORT).show()
                    return@launch
                }

                if (isOffice) {
                    navEvents.emit(NavigationEvent.OpenOffice(localPath))
                } else {
                    val result = repository.openDocument(localPath)
                    if (result.isSuccess) {
                        navEvents.emit(NavigationEvent.OpenPdf(localPath))
                    } else {
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
            val rootCacheDir = File(cacheDir, "opened_documents")
            rootCacheDir.mkdirs()
            // Prevent files with the same name from overwriting each other by isolating each in a unique subfolder
            val uniqueSubdir = File(rootCacheDir, "${System.currentTimeMillis()}_${UUID.randomUUID().toString().take(6)}")
            uniqueSubdir.mkdirs()
            val targetFile = File(uniqueSubdir, displayName)

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

    private fun exportCurrentDocument() {
        val doc = repository.documentState.value
        if (!doc.isOpen) return
        val defaultName = if (doc.fileName.isNotBlank()) {
            if (doc.fileName.endsWith(".pdf", ignoreCase = true)) doc.fileName else "${doc.fileName}.pdf"
        } else {
            "document_export.pdf"
        }
        exportDocumentLauncher.launch(defaultName)
    }

    private fun exportCurrentDocumentToUri(destinationUri: Uri) {
        val doc = repository.documentState.value
        if (!doc.isOpen || doc.filePath.isBlank()) return

        lifecycleScope.launch(Dispatchers.IO) {
            try {
                val sourceFile = File(doc.filePath)
                if (!sourceFile.exists()) {
                    withContext(Dispatchers.Main) {
                        Toast.makeText(this@MainActivity, R.string.error_save_document, Toast.LENGTH_SHORT).show()
                    }
                    return@launch
                }

                contentResolver.openOutputStream(destinationUri)?.use { outputStream ->
                    sourceFile.inputStream().use { inputStream ->
                        inputStream.copyTo(outputStream)
                    }
                }
                withContext(Dispatchers.Main) {
                    Toast.makeText(this@MainActivity, R.string.office_save_success, Toast.LENGTH_SHORT).show()
                }
            } catch (e: Exception) {
                withContext(Dispatchers.Main) {
                    Toast.makeText(this@MainActivity, R.string.error_save_document, Toast.LENGTH_SHORT).show()
                }
            }
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
        var currentScreen by rememberSaveable { mutableStateOf(AppScreen.VIEWER) }
        var officeFilePath by rememberSaveable { mutableStateOf("") }

        LaunchedEffect(Unit) {
            navEvents.collect { event ->
                when (event) {
                    is NavigationEvent.OpenOffice -> {
                        officeFilePath = event.filePath
                        currentScreen = AppScreen.OFFICE_EDITOR
                    }
                    is NavigationEvent.OpenPdf -> {
                        currentScreen = AppScreen.VIEWER
                    }
                }
            }
        }

        when (currentScreen) {
            AppScreen.VIEWER -> {
                PdfViewerScreen(
                    viewModel = viewerViewModel,
                    onNavigateToOrganizer = { currentScreen = AppScreen.ORGANIZER },
                    onScanDocumentRequested = { currentScreen = AppScreen.SCANNER },
                    onShareRequested = { shareCurrentDocument() },
                    onExportRequested = { exportCurrentDocument() },
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
            AppScreen.OFFICE_EDITOR -> {
                OfficeEditorScreen(
                    filePath = officeFilePath,
                    onNavigateBack = { currentScreen = AppScreen.VIEWER },
                    onPdfGenerated = { generatedPdfPath ->
                        lifecycleScope.launch {
                            repository.openDocument(generatedPdfPath)
                            currentScreen = AppScreen.VIEWER
                        }
                    },
                    modifier = Modifier.fillMaxSize()
                )
            }
        }
    }
}
