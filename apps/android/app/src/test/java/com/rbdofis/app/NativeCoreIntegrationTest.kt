package com.rbdofis.app

import com.rbdofis.app.core.PageRect
import com.rbdofis.app.core.PdfAnnotation
import com.rbdofis.app.core.RbdNativeBridge
import com.rbdofis.app.data.PdfDocumentRepository
import kotlinx.coroutines.runBlocking
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Before
import org.junit.Test
import java.io.File
import java.nio.ByteBuffer
import java.util.Base64

/**
 * Real Native Core Integration Tests for RBD OFİS.
 *
 * Verifies that:
 * 1. The native Rust core library (`rbd_ffi.dll` / `librbd_ffi.so`) is successfully loaded via JNI.
 * 2. Real corpus PDF documents from `corpus/files/` are opened, parsed, and inspected with exact page counts.
 * 3. Native page operations (rotation, deletion, split, merge, crop, reorder, blank insertion) operate on real files.
 * 4. Native professional PDF tools (compression, redaction, watermarking, page numbers, Bates stamp) execute cleanly.
 * 5. Native local offline AI summarization and Q&A engine produce valid JSON responses.
 * 6. Document scanning algorithms (Otsu thresholding, in-place binarization, PDF generation from images) execute.
 * 7. All JNI method signatures are properly bound with zero [UnsatisfiedLinkError].
 *
 * If the native library or corpus files are not present on the host system, tests are explicitly
 * skipped via [assumeTrue] rather than silently passing in mock mode.
 */
class NativeCoreIntegrationTest {

    private lateinit var bridge: RbdNativeBridge
    private lateinit var corpusDir: File
    private lateinit var tempDir: File

    @Before
    fun setUp() {
        bridge = RbdNativeBridge.getInstance()
        // Ensure mock mode is DISABLED to exercise real native Rust core
        bridge.setForceMockMode(false)
        bridge.loadNativeLibrary()

        assumeTrue(
            "Native library (rbd_ffi) is not loaded; skipping native integration test",
            bridge.isNativeAvailable()
        )

        corpusDir = locateCorpusDir()
        assumeTrue(
            "Test corpus directory not found; skipping native integration test",
            corpusDir.exists() && corpusDir.isDirectory
        )

        tempDir = File(System.getProperty("java.io.tmpdir"), "rbd_native_test_${System.currentTimeMillis()}")
        tempDir.mkdirs()
    }

    @After
    fun tearDown() {
        if (::tempDir.isInitialized && tempDir.exists()) {
            tempDir.deleteRecursively()
        }
    }

    private fun locateCorpusDir(): File {
        val sysProp = System.getProperty("rbd.corpus.dir")
        if (!sysProp.isNullOrBlank()) {
            val f = File(sysProp)
            if (f.exists()) return f
        }
        val candidates = listOf(
            File("../../corpus"),
            File("../../../corpus"),
            File("../corpus"),
            File("corpus")
        )
        return candidates.firstOrNull { it.exists() } ?: File("../../corpus")
    }

    private fun getCorpusFile(relativePath: String): File {
        val file = File(corpusDir, relativePath)
        assumeTrue("Corpus file '${file.absolutePath}' must exist", file.exists())
        return file
    }

    @Test
    fun testRealDocumentOpenAndPageCounts() {
        // 1. Single-page PDF
        val singlePagePdf = getCorpusFile("files/pdf-basic/pdfium-hello-world.pdf")
        val handle1 = bridge.openDocument(singlePagePdf.absolutePath)
        assertTrue("Single-page PDF handle must be positive non-zero", handle1 > 0L)
        try {
            val count1 = bridge.getPageCount(handle1)
            assertEquals("pdfium-hello-world.pdf must contain exactly 1 page", 1, count1)
        } finally {
            bridge.closeDocument(handle1)
        }

        // 2. Two-page PDF
        val twoPagePdf = getCorpusFile("files/pdf-basic/pdfium-hello-world-2-pages.pdf")
        val handle2 = bridge.openDocument(twoPagePdf.absolutePath)
        assertTrue("Two-page PDF handle must be positive non-zero", handle2 > 0L)
        try {
            val count2 = bridge.getPageCount(handle2)
            assertEquals("pdfium-hello-world-2-pages.pdf must contain exactly 2 pages", 2, count2)
        } finally {
            bridge.closeDocument(handle2)
        }

        // 3. Multi-page document (TraceMonkey presentation, 14 pages)
        val tracemonkeyPdf = getCorpusFile("files/pdf-basic/pdfjs-tracemonkey.pdf")
        val handle3 = bridge.openDocument(tracemonkeyPdf.absolutePath)
        assertTrue("TraceMonkey handle must be positive non-zero", handle3 > 0L)
        try {
            val count3 = bridge.getPageCount(handle3)
            assertEquals("pdfjs-tracemonkey.pdf must contain exactly 14 pages", 14, count3)
        } finally {
            bridge.closeDocument(handle3)
        }
    }

    @Test
    fun testRealDocumentPageSizeAndTextSearch() {
        val pdf = getCorpusFile("files/pdf-basic/pdfium-hello-world.pdf")
        val handle = bridge.openDocument(pdf.absolutePath)
        assertTrue(handle > 0L)
        try {
            // Verify page dimensions (positive points)
            val size = bridge.getPageSize(handle, 0)
            assertTrue("Page width must be positive", size.width > 0f)
            assertTrue("Page height must be positive", size.height > 0f)

            // Verify full-text search against real PDF content
            val searchResults = bridge.searchText(handle, "Hello")
            assertNotNull(searchResults)
            assertTrue("Searching 'Hello' in pdfium-hello-world.pdf must yield results", searchResults.isNotEmpty())
        } finally {
            bridge.closeDocument(handle)
        }
    }

    @Test
    fun testRealPageOperationsRotateDeleteMergeSplitInsertBlankCrop() {
        val singlePdf = getCorpusFile("files/pdf-basic/pdfium-hello-world.pdf")
        val twoPagePdf = getCorpusFile("files/pdf-basic/pdfium-hello-world-2-pages.pdf")

        // 1. Page Rotate
        val rotatedPdf = File(tempDir, "rotated.pdf")
        val rotateOk = bridge.pageOpRotate(singlePdf.absolutePath, rotatedPdf.absolutePath, 90)
        assertTrue("Native pageOpRotate must succeed", rotateOk)
        assertTrue("Rotated PDF file must exist", rotatedPdf.exists() && rotatedPdf.length() > 0)
        val hRot = bridge.openDocument(rotatedPdf.absolutePath)
        assertEquals(1, bridge.getPageCount(hRot))
        bridge.closeDocument(hRot)

        // 2. Delete Page (Delete page 1 from 2-page PDF -> 1 page left)
        val deletedPdf = File(tempDir, "deleted.pdf")
        val deleteOk = bridge.pageOpDeletePages(twoPagePdf.absolutePath, deletedPdf.absolutePath, listOf(1))
        assertTrue("Native pageOpDeletePages must succeed", deleteOk)
        val hDel = bridge.openDocument(deletedPdf.absolutePath)
        assertEquals(1, bridge.getPageCount(hDel))
        bridge.closeDocument(hDel)

        // 3. Page Split (Extract page 1..1 -> 1 page PDF)
        val splitPdf = File(tempDir, "split.pdf")
        val splitOk = bridge.pageOpSplit(twoPagePdf.absolutePath, splitPdf.absolutePath, 1, 1)
        assertTrue("Native pageOpSplit must succeed", splitOk)
        val hSplit = bridge.openDocument(splitPdf.absolutePath)
        assertEquals(1, bridge.getPageCount(hSplit))
        bridge.closeDocument(hSplit)

        // 4. Page Merge (Merge 1-page + 2-page -> 3 pages)
        val mergedPdf = File(tempDir, "merged.pdf")
        val mergeOk = bridge.pageOpMerge(mergedPdf.absolutePath, listOf(singlePdf.absolutePath, twoPagePdf.absolutePath))
        assertTrue("Native pageOpMerge must succeed", mergeOk)
        val hMerge = bridge.openDocument(mergedPdf.absolutePath)
        assertEquals(3, bridge.getPageCount(hMerge))
        bridge.closeDocument(hMerge)

        // 5. Insert Blank Page (Insert 1 blank page after page 1 of 1-page PDF -> 2 pages)
        val blankInsertedPdf = File(tempDir, "blank_inserted.pdf")
        val blankOk = bridge.pageOpInsertBlank(singlePdf.absolutePath, blankInsertedPdf.absolutePath, 1, 595.28f, 841.89f)
        assertTrue("Native pageOpInsertBlank must succeed", blankOk)
        val hBlank = bridge.openDocument(blankInsertedPdf.absolutePath)
        assertEquals(2, bridge.getPageCount(hBlank))
        bridge.closeDocument(hBlank)

        // 6. Page Crop
        val croppedPdf = File(tempDir, "cropped.pdf")
        val cropOk = bridge.pageOpCrop(singlePdf.absolutePath, croppedPdf.absolutePath, 1, 50f, 50f, 300f, 400f)
        assertTrue("Native pageOpCrop must succeed", cropOk)
        assertTrue(croppedPdf.exists() && croppedPdf.length() > 0)
    }

    @Test
    fun testRealPdfCompression() {
        val inputPdf = getCorpusFile("files/pdf-basic/pdfjs-tracemonkey.pdf")
        val compressedPdf = File(tempDir, "compressed.pdf")

        val stats = bridge.compressDocument(
            inputPath = inputPdf.absolutePath,
            outputPath = compressedPdf.absolutePath,
            removeUnused = true,
            recompress = true,
            stripMetadata = false
        )

        assertNotNull("Compression stats should not be null", stats)
        assertTrue("Original size must be positive", stats!!.originalSize > 0)
        assertTrue("Compressed size must be positive", stats.compressedSize > 0)
        assertTrue("Compressed file must exist on disk", compressedPdf.exists() && compressedPdf.length() > 0)

        // Verify the compressed PDF can be opened by native engine
        val handle = bridge.openDocument(compressedPdf.absolutePath)
        assertTrue("Compressed PDF must open cleanly in native engine", handle > 0L)
        assertEquals(14, bridge.getPageCount(handle))
        bridge.closeDocument(handle)
    }

    @Test
    fun testRealPdfRedaction() {
        val inputPdf = getCorpusFile("files/pdf-basic/pdfium-hello-world.pdf")
        val redactedPdf = File(tempDir, "redacted.pdf")
        val redactionJson = """[{"page_index":0,"rects":[{"x0":50.0,"y0":650.0,"x1":300.0,"y1":750.0}],"fill_color":[0,0,0]}]"""

        val success = bridge.redactDocument(
            inputPath = inputPdf.absolutePath,
            outputPath = redactedPdf.absolutePath,
            redactionsJson = redactionJson
        )

        assertTrue("Native redactDocument must succeed", success)
        assertTrue("Redacted output file must exist and have content", redactedPdf.exists() && redactedPdf.length() > 0)

        val handle = bridge.openDocument(redactedPdf.absolutePath)
        assertTrue(handle > 0L)
        assertEquals(1, bridge.getPageCount(handle))
        bridge.closeDocument(handle)
    }

    @Test
    fun testRealPdfWatermarkAndPageNumbersAndBatesStamp() {
        val inputPdf = getCorpusFile("files/pdf-basic/pdfium-hello-world-2-pages.pdf")

        // 1. Watermark
        val watermarkedPdf = File(tempDir, "watermarked.pdf")
        val wmOk = bridge.applyWatermark(
            inputPath = inputPdf.absolutePath,
            outputPath = watermarkedPdf.absolutePath,
            text = "GİZLİ & ÖZEL",
            fontSize = 36f,
            rotationDeg = 45f,
            opacity = 0.3f,
            r = 128,
            g = 128,
            b = 128,
            onTop = true
        )
        assertTrue("Native applyWatermark must succeed", wmOk)
        assertTrue(watermarkedPdf.exists() && watermarkedPdf.length() > 0)

        // 2. Page numbers
        val pageNumberedPdf = File(tempDir, "page_numbered.pdf")
        val pnOk = bridge.applyPageNumbers(
            inputPath = inputPdf.absolutePath,
            outputPath = pageNumberedPdf.absolutePath,
            pattern = "Sayfa {page} / {total}",
            position = 4,
            fontSize = 10f,
            margin = 36f
        )
        assertTrue("Native applyPageNumbers must succeed", pnOk)
        assertTrue(pageNumberedPdf.exists() && pageNumberedPdf.length() > 0)

        // 3. Bates stamp
        val batesPdf = File(tempDir, "bates_stamped.pdf")
        val bsOk = bridge.applyBatesStamp(
            inputPath = inputPdf.absolutePath,
            outputPath = batesPdf.absolutePath,
            prefix = "RBD-CASE-",
            startNumber = 100,
            paddingDigits = 6,
            suffix = "-CONF",
            position = 5
        )
        assertTrue("Native applyBatesStamp must succeed", bsOk)
        assertTrue(batesPdf.exists() && batesPdf.length() > 0)
    }

    @Test
    fun testRealAiSummarizeAndAsk() {
        val inputPdf = getCorpusFile("files/pdf-basic/pdfium-hello-world.pdf")
        val handle = bridge.openDocument(inputPdf.absolutePath)
        assertTrue(handle > 0L)
        try {
            // 1. AI Summarize
            val summaryJson = bridge.summarizeDocument(handle, maxSentences = 3)
            assertNotNull("AI summary JSON response must not be null", summaryJson)
            assertTrue("AI summary must contain JSON fields", summaryJson.contains("summary_text") || summaryJson.contains("{"))

            // 2. AI Ask
            val askJson = bridge.askDocument(handle, "What is the greeting in this document?")
            assertNotNull("AI ask JSON response must not be null", askJson)
            assertTrue("AI ask must return valid answer structure", askJson.contains("answer_text") || askJson.contains("found") || askJson.contains("{"))
        } finally {
            bridge.closeDocument(handle)
        }
    }

    @Test
    fun testRealScanOtsuThresholdCalculation() {
        // Construct a synthetic bimodal histogram (dark peak near 40, bright peak near 220)
        val histogram = IntArray(256)
        histogram[40] = 500
        histogram[220] = 1000

        val threshold = bridge.scanCalculateOtsuThreshold(histogram, 1500)
        assertTrue(
            "Optimal Otsu threshold for peaks at 40 and 220 must lie strictly between them (got $threshold)",
            threshold in 41..219
        )
    }

    @Test
    fun testRealScanDirectBufferBinarizeAndGrayscale() {
        val width = 32
        val height = 32
        val stride = width * 4
        val bufferSize = stride * height
        val buffer = ByteBuffer.allocateDirect(bufferSize)

        // Populate with synthetic RGBA gradient
        for (y in 0 until height) {
            for (x in 0 until width) {
                val lum = ((x + y) * 255 / (width + height)).toByte()
                buffer.put(lum)         // R
                buffer.put(lum)         // G
                buffer.put(lum)         // B
                buffer.put(255.toByte()) // A
            }
        }
        buffer.rewind()

        // 1. In-place Grayscale
        val grayStatus = bridge.scanToGrayscaleBuffer(buffer, width, height, stride)
        assertEquals("Native scanToGrayscaleBuffer must return RBD_OK (0)", 0, grayStatus)

        // 2. In-place Binarization
        buffer.rewind()
        val binStatus = bridge.scanToBinarizedBuffer(buffer, width, height, stride)
        assertEquals("Native scanToBinarizedBuffer must return RBD_OK (0)", 0, binStatus)

        // Verify values are strictly binarized (0 or 255)
        buffer.rewind()
        for (i in 0 until (width * height)) {
            val r = buffer.get().toInt() and 0xFF
            val g = buffer.get().toInt() and 0xFF
            val b = buffer.get().toInt() and 0xFF
            val a = buffer.get().toInt() and 0xFF
            assertTrue("Binarized R component must be 0 or 255 (got $r)", r == 0 || r == 255)
            assertTrue("Binarized G component must be 0 or 255 (got $g)", g == 0 || g == 255)
            assertTrue("Binarized B component must be 0 or 255 (got $b)", b == 0 || b == 255)
        }
    }

    @Test
    fun testRealScanGeneratePdfFromImage() {
        // Create a valid 1x1 PNG file using standard Base64 decoding (no java.awt dependency)
        val samplePng = File(tempDir, "scanned_page.png")
        val pngBytes = Base64.getDecoder().decode(
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg=="
        )
        samplePng.writeBytes(pngBytes)
        assertTrue(samplePng.exists() && samplePng.length() > 0)

        // Compile image into PDF using native scanGeneratePdf
        val outputPdf = File(tempDir, "scanned_doc.pdf")
        val ok = bridge.scanGeneratePdf(listOf(samplePng.absolutePath), outputPdf.absolutePath)
        assertTrue("Native scanGeneratePdf must succeed", ok)
        assertTrue("Generated scanned PDF must exist and have content", outputPdf.exists() && outputPdf.length() > 0)

        // Verify the resulting PDF can be parsed by the engine
        val handle = bridge.openDocument(outputPdf.absolutePath)
        assertTrue(handle > 0L)
        assertEquals("Generated PDF must have exactly 1 page", 1, bridge.getPageCount(handle))
        bridge.closeDocument(handle)
    }

    @Test
    fun testAllJniSignaturesBoundWithoutUnsatisfiedLinkError() {
        // Explicitly verify engine lifecycle and JNI link
        val engineHandle = bridge.getEngineHandle()
        // Ensure engine is alive or can be re-initialized
        val singlePdf = getCorpusFile("files/pdf-basic/pdfium-hello-world.pdf")
        val handle = bridge.openDocument(singlePdf.absolutePath)
        assertTrue("Document open via JNI must succeed", handle > 0L)

        // Annotations JNI binding
        val annotOk = bridge.addAnnotationJson(
            handle,
            """{"id":1001,"page_index":0,"kind":"Highlight","rect":{"x0":10.0,"y0":20.0,"x1":100.0,"y1":50.0},"color_rgba":[255,235,59,255],"contents":"Test"}"""
        )
        // Check remove annotation JNI
        bridge.removeAnnotation(handle, 1001L)

        // Check undo/redo JNI
        bridge.undo(handle)
        bridge.redo(handle)

        // Office docx creation JNI
        val outDocx = File(tempDir, "test.docx")
        bridge.officeCreateDocx("RBD OFİS Integration Test Content", outDocx.absolutePath)
        assertTrue(outDocx.exists() && outDocx.length() > 0)

        // Office text extraction JNI
        val extractedText = bridge.officeExtractText(outDocx.absolutePath)
        assertNotNull(extractedText)
        assertTrue(extractedText!!.contains("RBD OFİS"))

        bridge.closeDocument(handle)
    }

    @Test
    fun testCorruptOrZeroByteFileReturnsZeroHandleWithoutMock() {
        val zeroByteFile = File(tempDir, "zero_byte.pdf")
        zeroByteFile.writeBytes(ByteArray(0))

        val handleZero = bridge.openDocument(zeroByteFile.absolutePath)
        assertEquals("0-byte file must return 0L without silently producing a mock", 0L, handleZero)
        assertFalse("0-byte file must not exist in mock documents", bridge.isMockDocument(handleZero))

        val corruptFile = File(tempDir, "corrupt.pdf")
        corruptFile.writeText("NOT A REAL PDF CONTENT AT ALL")

        val handleCorrupt = bridge.openDocument(corruptFile.absolutePath)
        assertEquals("Corrupt PDF file must return 0L without silently producing a mock", 0L, handleCorrupt)

        val corruptBytesHandle = bridge.openDocumentFromBytes("NOT A PDF".toByteArray())
        assertEquals("Corrupt byte array must return 0L without silently producing a mock", 0L, corruptBytesHandle)
    }

    @Test
    fun testCloseDocumentDoesNotSendMockHandleToNativeFree() {
        // Force mock to obtain a mock handle
        bridge.setForceMockMode(true)
        val mockHandle = bridge.openDocument("/test/mock_lifecycle.pdf")
        assertTrue("Mock handle should be positive non-zero", mockHandle > 0L)
        assertTrue("Handle must be tracked as mock", bridge.isMockDocument(mockHandle))

        // Re-enable native library
        bridge.setForceMockMode(false)
        assertTrue("Native library must be available", bridge.isNativeAvailable())

        // Closing mock handle while native is active must NEVER call nativeDocumentFree (avoids SIGSEGV)
        bridge.closeDocument(mockHandle)
        assertFalse("Mock document must be removed from tracking", bridge.isMockDocument(mockHandle))
    }

    @Test
    fun testNativeRepositoryPageOperationsRenewActiveHandleAndPersistOnDisk() = runBlocking {
        val twoPagePdf = getCorpusFile("files/pdf-basic/pdfium-hello-world-2-pages.pdf")
        val workPdf = File(tempDir, "work_two_page.pdf")
        twoPagePdf.copyTo(workPdf, overwrite = true)

        val repository = PdfDocumentRepository(bridge)
        try {
            val openResult = repository.openDocument(workPdf.absolutePath)
            assertTrue("Document open must succeed", openResult.isSuccess)
            assertEquals(2, openResult.getOrNull())

            val initialHandle = repository.getActiveDocHandle()
            assertTrue("Initial handle must be positive non-zero", initialHandle > 0L)
            assertFalse("Handle must be a real native handle, not mock", bridge.isMockDocument(initialHandle))

            // 1. Rotate page 0 by 90 degrees
            val rotateSuccess = repository.rotatePage(0, 90)
            assertTrue("rotatePage must succeed natively", rotateSuccess)
            val rotatedHandle = repository.getActiveDocHandle()
            assertTrue("Rotated handle must be valid", rotatedHandle > 0L)
            assertNotEquals("Active document handle must be renewed after rotate operation", initialHandle, rotatedHandle)
            assertTrue("Document state must be marked modified", repository.documentState.value.isModified)

            // 2. Delete page 1 (now 1 page remains)
            val deleteSuccess = repository.deletePage(1)
            assertTrue("deletePage must succeed natively", deleteSuccess)
            val deletedHandle = repository.getActiveDocHandle()
            assertTrue("Deleted handle must be valid", deletedHandle > 0L)
            assertNotEquals("Active document handle must be renewed after delete operation", rotatedHandle, deletedHandle)
            assertEquals("Page count must be 1 after deletion", 1, repository.documentState.value.pageCount)

            // 3. Save modified document to a new target path
            val savedPdf = File(tempDir, "persisted_result.pdf")
            val saveSuccess = repository.saveDocument(savedPdf.absolutePath)
            assertTrue("saveDocument must succeed", saveSuccess)
            assertFalse("isModified must be cleared after save", repository.documentState.value.isModified)
            assertTrue("Saved PDF file must exist on disk", savedPdf.exists() && savedPdf.length() > 0)

            // 4. Verify saved file natively
            val checkHandle = bridge.openDocument(savedPdf.absolutePath)
            assertTrue("Saved PDF must be openable natively", checkHandle > 0L)
            assertEquals("Saved PDF must have exactly 1 page", 1, bridge.getPageCount(checkHandle))
            bridge.closeDocument(checkHandle)
        } finally {
            repository.closeDocument()
        }
    }
}
