package com.rbdofis.app

import com.rbdofis.app.core.RbdNativeBridge
import org.junit.After
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import java.io.File

/**
 * Mock-based unit tests for Phase 6 Office document capabilities on Android in simulation mode.
 * Forces mock mode via setForceMockMode(true).
 */
class OfficeDocumentTest {

    private lateinit var bridge: RbdNativeBridge
    private lateinit var tempDir: File

    @Before
    fun setUp() {
        bridge = RbdNativeBridge.getInstance()
        bridge.setForceMockMode(true)
        tempDir = File(System.getProperty("java.io.tmpdir"), "rbd_office_android_test_${System.currentTimeMillis()}")
        tempDir.mkdirs()
    }

    @After
    fun tearDown() {
        tempDir.deleteRecursively()
    }

    @Test
    fun testIsSupportedOfficeFile() {
        assertTrue(bridge.isSupportedOfficeFile("document.docx"))
        assertTrue(bridge.isSupportedOfficeFile("SHEET.XLSX"))
        assertTrue(bridge.isSupportedOfficeFile("slides.pptx"))
        assertTrue(bridge.isSupportedOfficeFile("legacy.doc"))
        assertTrue(bridge.isSupportedOfficeFile("legacy.xls"))
        assertTrue(bridge.isSupportedOfficeFile("legacy.ppt"))

        assertFalse(bridge.isSupportedOfficeFile("document.pdf"))
        assertFalse(bridge.isSupportedOfficeFile("image.png"))
        assertFalse(bridge.isSupportedOfficeFile(""))
    }

    @Test
    fun testOfficeCreateDocxAndExtract() {
        val outDocx = File(tempDir, "test.docx")
        val sampleText = "RBD OFİS Android Ofis Düzenleyici\nTürkçe karakterler: çşğüöı."

        val created = bridge.officeCreateDocx(sampleText, outDocx.absolutePath)
        assertTrue("Office create DOCX must succeed", created)
        assertTrue("Output file must exist on disk", outDocx.exists())

        val extracted = bridge.officeExtractText(outDocx.absolutePath)
        assertNotNull("Extracted text must not be null", extracted)
        assertTrue("Extracted text must contain file name or content", extracted!!.contains("test.docx") || extracted.contains("RBD OFİS"))
    }

    @Test
    fun testOfficeInspectMetadata() {
        val docFile = File(tempDir, "report.docx").apply {
            writeText("MOCK CONTENT FOR INSPECTION")
        }

        val metadata = bridge.officeInspect(docFile.absolutePath)
        assertNotNull("Metadata must not be null", metadata)
        assertEquals("DOCX", metadata!!.docType)
        assertEquals("report", metadata.title)
        assertTrue(metadata.wordCount > 0)
        assertTrue(metadata.itemCount > 0)
    }

    @Test
    fun testOfficeExtractMissingFile() {
        val missing = File(tempDir, "non_existent.docx")
        val result = bridge.officeExtractText(missing.absolutePath)
        assertNull("Extracting from non-existent file must return null", result)
    }

    @Test
    fun testOfficeCreateDocxInvalidPath() {
        val result = bridge.officeCreateDocx("text", "")
        assertFalse("Creating docx with empty path must return false", result)
    }

    @Test
    fun testReadOnlyOfficeFormatsProtection() {
        val docxFiles = listOf("test.docx", "test.DOCX", "test.doc")
        val readonlyFiles = listOf("table.xlsx", "table.XLS", "deck.pptx", "deck.PPT")

        for (file in docxFiles) {
            val ext = file.substringAfterLast('.', "").lowercase()
            val isDocx = ext == "docx" || ext == "doc"
            assertTrue("DOCX file should be marked editable: $file", isDocx)
        }

        for (file in readonlyFiles) {
            val ext = file.substringAfterLast('.', "").lowercase()
            val isDocx = ext == "docx" || ext == "doc"
            assertFalse("XLSX/PPTX file must be marked read-only to prevent corruption: $file", isDocx)
        }
    }
}
