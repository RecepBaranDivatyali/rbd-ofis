package com.rbdofis.app

import com.rbdofis.app.core.ImageExportFormat
import com.rbdofis.app.core.ImageToPdfPageSize
import com.rbdofis.app.core.RbdNativeBridge
import com.rbdofis.app.data.PdfDocumentRepository
import kotlinx.coroutines.runBlocking
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import java.io.File

class DocumentConverterTest {

    private lateinit var bridge: RbdNativeBridge
    private lateinit var repository: PdfDocumentRepository
    private lateinit var tempDir: File

    @Before
    fun setUp() {
        bridge = RbdNativeBridge.getInstance()
        bridge.setForceMockMode(true)
        repository = PdfDocumentRepository(bridge)
        tempDir = File(System.getProperty("java.io.tmpdir"), "rbd_converter_test_${System.currentTimeMillis()}")
        tempDir.mkdirs()
    }

    @After
    fun tearDown() {
        tempDir.deleteRecursively()
    }

    @Test
    fun testPdfToImagesPngDefault() {
        val inFile = File(tempDir, "sample.pdf").apply { writeText("%PDF-1.7 mock data\n") }
        val outDir = File(tempDir, "images_out_png").apply { mkdirs() }

        val images = bridge.pdfToImages(
            inPath = inFile.absolutePath,
            outDir = outDir.absolutePath,
            dpi = 150,
            format = ImageExportFormat.PNG,
            pageRange = null
        )

        assertNotNull("Images list must not be null", images)
        assertTrue("At least one image must be generated", images!!.isNotEmpty())
        for (path in images) {
            val f = File(path)
            assertTrue("Generated image must exist: $path", f.exists())
            assertTrue("Generated image must have .png extension", path.endsWith(".png"))
        }
    }

    @Test
    fun testPdfToImagesJpegWithPageRange() {
        val inFile = File(tempDir, "sample.pdf").apply { writeText("%PDF-1.7 mock data\n") }
        val outDir = File(tempDir, "images_out_jpg").apply { mkdirs() }
        val targetPages = listOf(0, 2)

        val images = bridge.pdfToImages(
            inPath = inFile.absolutePath,
            outDir = outDir.absolutePath,
            dpi = 300,
            format = ImageExportFormat.JPEG,
            pageRange = targetPages
        )

        assertNotNull("Images list must not be null", images)
        assertEquals("Image count should match pageRange count", targetPages.size, images!!.size)
        for (path in images) {
            val f = File(path)
            assertTrue("Generated JPEG must exist: $path", f.exists())
            assertTrue("Generated image must have .jpg extension", path.endsWith(".jpg"))
        }
    }

    @Test
    fun testImagesToPdfSuccess() {
        val img1 = File(tempDir, "img1.png").apply { writeText("mock img 1") }
        val img2 = File(tempDir, "img2.jpg").apply { writeText("mock img 2") }
        val outPdf = File(tempDir, "combined.pdf")

        val success = bridge.imagesToPdf(
            imagePaths = listOf(img1.absolutePath, img2.absolutePath),
            pageSize = ImageToPdfPageSize.A4,
            marginPt = 10f,
            outPath = outPdf.absolutePath
        )

        assertTrue("imagesToPdf should return true", success)
        assertTrue("Combined PDF file must exist", outPdf.exists())
        assertTrue("PDF file must not be empty", outPdf.length() > 0)
    }

    @Test
    fun testImagesToPdfEmptyListReturnsFalse() {
        val outPdf = File(tempDir, "empty_combined.pdf")

        val success = bridge.imagesToPdf(
            imagePaths = emptyList(),
            pageSize = ImageToPdfPageSize.LETTER,
            marginPt = 5f,
            outPath = outPdf.absolutePath
        )

        assertFalse("imagesToPdf should return false for empty list", success)
        assertFalse("No PDF should be created for empty image list", outPdf.exists())
    }

    @Test
    fun testImagesToPdfCustomPageSizes() {
        val img = File(tempDir, "single_img.png").apply { writeText("mock") }
        val letterPdf = File(tempDir, "letter.pdf")
        val fitPdf = File(tempDir, "fit.pdf")

        val letterSuccess = bridge.imagesToPdf(
            imagePaths = listOf(img.absolutePath),
            pageSize = ImageToPdfPageSize.LETTER,
            marginPt = 0f,
            outPath = letterPdf.absolutePath
        )
        val fitSuccess = bridge.imagesToPdf(
            imagePaths = listOf(img.absolutePath),
            pageSize = ImageToPdfPageSize.FIT_IMAGE,
            marginPt = 15f,
            outPath = fitPdf.absolutePath
        )

        assertTrue("Letter conversion must succeed", letterSuccess)
        assertTrue("Fit image conversion must succeed", fitSuccess)
        assertTrue(letterPdf.exists())
        assertTrue(fitPdf.exists())
    }

    @Test
    fun testMarkdownToPdfSuccess() {
        val outPdf = File(tempDir, "md_output.pdf")
        val mdContent = """
            # Document Title
            
            This is a paragraph converted from **Markdown** to PDF.
            
            ## Features
            - Item 1
            - Item 2
        """.trimIndent()

        val success = bridge.markdownToPdf(
            markdownText = mdContent,
            outPath = outPdf.absolutePath
        )

        assertTrue("markdownToPdf should succeed", success)
        assertTrue("Markdown output PDF must exist", outPdf.exists())
        assertTrue("Markdown output PDF must not be empty", outPdf.length() > 0)
    }

    @Test
    fun testMarkdownToPdfEmptyReturnsFalse() {
        val outPdf = File(tempDir, "empty_md.pdf")

        val success = bridge.markdownToPdf(
            markdownText = "   ",
            outPath = outPdf.absolutePath
        )

        assertFalse("markdownToPdf with blank text should return false", success)
        assertFalse("No output file should be generated for empty markdown", outPdf.exists())
    }

    @Test
    fun testRepositoryConverterIntegration() = runBlocking {
        val mockPdf = File(tempDir, "repo_input.pdf").apply { writeText("%PDF-1.7 mock data\n") }
        repository.openDocument(mockPdf.absolutePath)

        val outDir = File(tempDir, "repo_images_out").apply { mkdirs() }
        val pdfToImagesResult = repository.convertPdfToImages(
            outDir = outDir.absolutePath,
            dpi = 150,
            format = ImageExportFormat.PNG
        )

        assertTrue("Repository convertPdfToImages should succeed", pdfToImagesResult.isSuccess)
        val imagePaths = pdfToImagesResult.getOrThrow()
        assertTrue("Should return image paths", imagePaths.isNotEmpty())

        val imagesToPdfOut = File(tempDir, "repo_from_images.pdf")
        val imagesToPdfResult = repository.convertImagesToPdf(
            imagePaths = imagePaths,
            pageSize = ImageToPdfPageSize.A4,
            marginPt = 10f,
            outPath = imagesToPdfOut.absolutePath
        )
        assertTrue("Repository convertImagesToPdf should succeed", imagesToPdfResult.isSuccess)
        assertTrue(imagesToPdfOut.exists())

        val mdToPdfOut = File(tempDir, "repo_from_md.pdf")
        val mdToPdfResult = repository.convertMarkdownToPdf(
            markdownText = "# Hello from Repository Test",
            outPath = mdToPdfOut.absolutePath
        )
        assertTrue("Repository convertMarkdownToPdf should succeed", mdToPdfResult.isSuccess)
        assertTrue(mdToPdfOut.exists())
    }

    @Test
    fun testEnumValues() {
        assertEquals(0, ImageExportFormat.PNG.value)
        assertEquals(1, ImageExportFormat.JPEG.value)

        assertEquals(0, ImageToPdfPageSize.A4.value)
        assertEquals(1, ImageToPdfPageSize.LETTER.value)
        assertEquals(2, ImageToPdfPageSize.FIT_IMAGE.value)
    }
}
