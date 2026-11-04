package com.rbdofis.app.ui.scanner

import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.pdf.PdfDocument
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.io.File
import java.io.FileOutputStream
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

/**
 * Image enhancement filters for scanned documents.
 */
enum class ScannerFilter {
    ORIGINAL,
    GRAYSCALE,
    BINARIZED
}

/**
 * Pure image processing algorithms for offline, high-contrast document scanning.
 * Adheres strictly to ADR-0003: Zero proprietary closed MLKit/Google SDKs.
 */
object DocumentImageProcessor {

    /**
     * Standard ITU-R BT.601 luminance formula.
     */
    fun computeLuminance(r: Int, g: Int, b: Int): Int {
        return (0.299f * r + 0.587f * g + 0.114f * b).toInt().coerceIn(0, 255)
    }

    /**
     * Converts an RGB bitmap to 8-bit grayscale.
     */
    fun toGrayscale(source: Bitmap): Bitmap {
        val width = source.width
        val height = source.height
        val output = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
        val pixels = IntArray(width * height)
        source.getPixels(pixels, 0, width, 0, 0, width, height)

        for (i in pixels.indices) {
            val pixel = pixels[i]
            val r = (pixel shr 16) and 0xFF
            val g = (pixel shr 8) and 0xFF
            val b = pixel and 0xFF
            val gray = computeLuminance(r, g, b)
            pixels[i] = -0x1000000 or (gray shl 16) or (gray shl 8) or gray
        }

        output.setPixels(pixels, 0, width, 0, 0, width, height)
        return output
    }

    /**
     * Calculates optimal threshold using Otsu's method from an intensity histogram.
     * When a plateau of maximum between-class variance occurs (e.g. between two distinct peaks),
     * the midpoint of the plateau is chosen for maximum noise margin.
     */
    fun calculateOtsuThreshold(histogram: IntArray, totalPixels: Int): Int {
        if (totalPixels <= 0) return 128

        var sum = 0.0
        for (i in 0 until 256) {
            sum += i.toDouble() * histogram[i]
        }

        var sumB = 0.0
        var weightB = 0L
        var maxVariance = -1.0
        var firstBest = 128
        var lastBest = 128

        for (t in 0 until 256) {
            weightB += histogram[t]
            if (weightB == 0L) continue

            val weightF = totalPixels - weightB
            if (weightF == 0L) break

            sumB += t.toDouble() * histogram[t]
            val meanB = sumB / weightB
            val meanF = (sum - sumB) / weightF

            val betweenVariance = weightB.toDouble() * weightF.toDouble() * (meanB - meanF) * (meanB - meanF)
            if (betweenVariance > maxVariance + 1e-5) {
                maxVariance = betweenVariance
                firstBest = t
                lastBest = t
            } else if (kotlin.math.abs(betweenVariance - maxVariance) <= 1e-5) {
                lastBest = t
            }
        }

        return (firstBest + lastBest) / 2
    }

    /**
     * High-contrast document binarization (Otsu thresholding) separating ink from paper.
     */
    fun toBinarized(source: Bitmap): Bitmap {
        val width = source.width
        val height = source.height
        val output = Bitmap.createBitmap(width, height, Bitmap.Config.ARGB_8888)
        val pixels = IntArray(width * height)
        source.getPixels(pixels, 0, width, 0, 0, width, height)

        // 1. Build luminance histogram
        val histogram = IntArray(256)
        val grays = IntArray(pixels.size)
        for (i in pixels.indices) {
            val pixel = pixels[i]
            val r = (pixel shr 16) and 0xFF
            val g = (pixel shr 8) and 0xFF
            val b = pixel and 0xFF
            val gray = computeLuminance(r, g, b)
            grays[i] = gray
            histogram[gray]++
        }

        // 2. Find optimal Otsu threshold
        val threshold = calculateOtsuThreshold(histogram, pixels.size)

        // 3. Binarize: at or below threshold is black text (0xFF000000), above is white paper (0xFFFFFFFF)
        val white = -0x1
        val black = -0x1000000
        for (i in pixels.indices) {
            pixels[i] = if (grays[i] <= threshold) black else white
        }

        output.setPixels(pixels, 0, width, 0, 0, width, height)
        return output
    }
}

/**
 * Compiles scanned bitmaps into standard ISO-compatible PDF documents using Android's native PdfDocument.
 */
object DocumentPdfGenerator {

    suspend fun createPdfFromBitmap(
        bitmap: Bitmap,
        outputDir: File,
        baseName: String = "RBD_Scan"
    ): File = withContext(Dispatchers.IO) {
        outputDir.mkdirs()
        val timeStamp = SimpleDateFormat("yyyyMMdd_HHmmss", Locale.US).format(Date())
        val pdfFile = File(outputDir, "${baseName}_${timeStamp}.pdf")

        val pdfDocument = PdfDocument()
        try {
            // Standard A4 dimensions in typographic points (595 x 842 pt)
            val pageInfo = PdfDocument.PageInfo.Builder(595, 842, 1).create()
            val page = pdfDocument.startPage(pageInfo)
            val canvas = page.canvas

            // Fill page background with white
            canvas.drawColor(Color.WHITE)

            // Fit bitmap preserving aspect ratio
            val pageWidth = 595f
            val pageHeight = 842f
            val margin = 20f

            val availableW = pageWidth - margin * 2f
            val availableH = pageHeight - margin * 2f

            val scale = minOf(availableW / bitmap.width.toFloat(), availableH / bitmap.height.toFloat())
            val drawW = bitmap.width.toFloat() * scale
            val drawH = bitmap.height.toFloat() * scale

            val left = margin + (availableW - drawW) / 2f
            val top = margin + (availableH - drawH) / 2f

            val paint = Paint().apply { isFilterBitmap = true }
            val srcRect = android.graphics.Rect(0, 0, bitmap.width, bitmap.height)
            val dstRect = android.graphics.RectF(left, top, left + drawW, top + drawH)

            canvas.drawBitmap(bitmap, srcRect, dstRect, paint)
            pdfDocument.finishPage(page)

            FileOutputStream(pdfFile).use { out ->
                pdfDocument.writeTo(out)
            }
            pdfFile
        } finally {
            pdfDocument.close()
        }
    }
}

data class ScannerUiState(
    val hasCameraPermission: Boolean = false,
    val isTorchEnabled: Boolean = false,
    val isCapturing: Boolean = false,
    val capturedBitmap: Bitmap? = null,
    val processedBitmap: Bitmap? = null,
    val activeFilter: ScannerFilter = ScannerFilter.BINARIZED,
    val isGeneratingPdf: Boolean = false,
    val savedPdfPath: String? = null,
    val errorMessage: String? = null
)

class DocumentScannerViewModel : ViewModel() {

    private val _uiState = MutableStateFlow(ScannerUiState())
    val uiState: StateFlow<ScannerUiState> = _uiState.asStateFlow()

    fun setCameraPermissionGranted(granted: Boolean) {
        _uiState.value = _uiState.value.copy(hasCameraPermission = granted)
    }

    fun toggleTorch() {
        _uiState.value = _uiState.value.copy(isTorchEnabled = !_uiState.value.isTorchEnabled)
    }

    fun setCapturing(capturing: Boolean) {
        _uiState.value = _uiState.value.copy(isCapturing = capturing)
    }

    fun onPhotoCaptured(bitmap: Bitmap) {
        viewModelScope.launch {
            val processed = applyFilter(bitmap, _uiState.value.activeFilter)
            _uiState.value = _uiState.value.copy(
                capturedBitmap = bitmap,
                processedBitmap = processed,
                isCapturing = false
            )
        }
    }

    fun setFilter(filter: ScannerFilter) {
        val captured = _uiState.value.capturedBitmap ?: return
        viewModelScope.launch {
            val processed = applyFilter(captured, filter)
            _uiState.value = _uiState.value.copy(
                activeFilter = filter,
                processedBitmap = processed
            )
        }
    }

    fun retake() {
        _uiState.value = _uiState.value.copy(
            capturedBitmap = null,
            processedBitmap = null,
            isGeneratingPdf = false,
            savedPdfPath = null,
            errorMessage = null
        )
    }

    fun saveAsPdf(context: Context, onComplete: (String) -> Unit) {
        val bitmapToSave = _uiState.value.processedBitmap ?: _uiState.value.capturedBitmap ?: return

        viewModelScope.launch {
            _uiState.value = _uiState.value.copy(isGeneratingPdf = true, errorMessage = null)
            try {
                val scansDir = File(context.cacheDir, "scans")
                val pdfFile = DocumentPdfGenerator.createPdfFromBitmap(bitmapToSave, scansDir)
                _uiState.value = _uiState.value.copy(
                    isGeneratingPdf = false,
                    savedPdfPath = pdfFile.absolutePath
                )
                onComplete(pdfFile.absolutePath)
            } catch (e: Exception) {
                _uiState.value = _uiState.value.copy(
                    isGeneratingPdf = false,
                    errorMessage = e.localizedMessage
                )
            }
        }
    }

    private suspend fun applyFilter(bitmap: Bitmap, filter: ScannerFilter): Bitmap = withContext(Dispatchers.Default) {
        when (filter) {
            ScannerFilter.ORIGINAL -> bitmap
            ScannerFilter.GRAYSCALE -> DocumentImageProcessor.toGrayscale(bitmap)
            ScannerFilter.BINARIZED -> DocumentImageProcessor.toBinarized(bitmap)
        }
    }
}
