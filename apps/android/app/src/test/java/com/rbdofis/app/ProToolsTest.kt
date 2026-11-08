package com.rbdofis.app

import com.rbdofis.app.core.CompressionStats
import com.rbdofis.app.core.FormFieldInfo
import com.rbdofis.app.core.RbdNativeBridge
import com.rbdofis.app.core.StampPosition
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import java.io.File

/**
 * Mock-based unit test verifying PDF Professional Tools UI/Presenter integration in simulation mode.
 * Forces mock mode via setForceMockMode(true).
 */
class ProToolsTest {

    private lateinit var bridge: RbdNativeBridge
    private lateinit var tempDir: File

    @Before
    fun setUp() {
        bridge = RbdNativeBridge.getInstance()
        bridge.setForceMockMode(true)
        tempDir = File(System.getProperty("java.io.tmpdir"), "rbd_pro_tools_test_${System.currentTimeMillis()}")
        tempDir.mkdirs()
    }

    @Test
    fun testCompressDocumentStatisticsAndMockOutput() {
        val inFile = File(tempDir, "input_compress.pdf")
        inFile.writeText("%PDF-1.7 sample data for compression testing\n")
        val outFile = File(tempDir, "output_compress.pdf")

        val stats = bridge.compressDocument(
            inputPath = inFile.absolutePath,
            outputPath = outFile.absolutePath,
            removeUnused = true,
            recompress = true,
            stripMetadata = false
        )

        assertNotNull("CompressionStats must not be null", stats)
        assertTrue("Compressed output file must be created", outFile.exists())
        assertTrue("Savings bytes must be non-negative", stats!!.savingsBytes >= 0)
        assertTrue("Savings ratio must be between 0.0 and 1.0", stats.savingsRatio in 0.0f..1.0f)
        assertEquals(stats.savingsRatio * 100f, stats.savingsPercentage, 0.01f)
    }

    @Test
    fun testApplyWatermark() {
        val inFile = File(tempDir, "input_wm.pdf")
        inFile.writeText("%PDF-1.7 test document\n")
        val outFile = File(tempDir, "output_wm.pdf")

        val success = bridge.applyWatermark(
            inputPath = inFile.absolutePath,
            outputPath = outFile.absolutePath,
            text = "CONFIDENTIAL",
            fontSize = 42f,
            rotationDeg = 45f,
            opacity = 0.5f,
            r = 255,
            g = 0,
            b = 0,
            onTop = true
        )

        assertTrue("Watermark application must return true", success)
        assertTrue("Watermarked output file must exist", outFile.exists())
    }

    @Test
    fun testApplyPageNumbers() {
        val inFile = File(tempDir, "input_numbered.pdf")
        inFile.writeText("%PDF-1.7 test document\n")
        val outFile = File(tempDir, "output_numbered.pdf")

        val success = bridge.applyPageNumbers(
            inputPath = inFile.absolutePath,
            outputPath = outFile.absolutePath,
            pattern = "Sayfa {page} / {total}",
            position = StampPosition.BOTTOM_CENTER.value,
            fontSize = 11f,
            margin = 36f
        )

        assertTrue("Page numbering must return true", success)
        assertTrue("Page numbered output file must exist", outFile.exists())
    }

    @Test
    fun testApplyBatesStamp() {
        val inFile = File(tempDir, "input_bates.pdf")
        inFile.writeText("%PDF-1.7 test document\n")
        val outFile = File(tempDir, "output_bates.pdf")

        val success = bridge.applyBatesStamp(
            inputPath = inFile.absolutePath,
            outputPath = outFile.absolutePath,
            prefix = "CASE-",
            startNumber = 100,
            paddingDigits = 6,
            suffix = "-CONF",
            position = StampPosition.BOTTOM_RIGHT.value
        )

        assertTrue("Bates stamping must return true", success)
        assertTrue("Bates stamped output file must exist", outFile.exists())
    }

    @Test
    fun testRedactDocument() {
        val inFile = File(tempDir, "input_redact.pdf")
        inFile.writeText("%PDF-1.7 test document with confidential text\n")
        val outFile = File(tempDir, "output_redact.pdf")

        val redactionsJson = """[{"page_index":0,"rects":[{"x0":50.0,"y0":650.0,"x1":300.0,"y1":750.0}],"fill_color":[0,0,0]}]"""
        val success = bridge.redactDocument(
            inputPath = inFile.absolutePath,
            outputPath = outFile.absolutePath,
            redactionsJson = redactionsJson
        )

        assertTrue("Permanent redaction must return true", success)
        assertTrue("Redacted output file must exist", outFile.exists())
    }

    @Test
    fun testReadAndFillFormFields() {
        val inFile = File(tempDir, "input_form.pdf")
        inFile.writeText("%PDF-1.7 interactive form\n")
        val outFile = File(tempDir, "output_form.pdf")

        // 1. Read fields
        val fields = bridge.readFormFields(inFile.absolutePath)
        assertFalse("Mock fields list must not be empty", fields.isEmpty())
        assertTrue("Should contain sample field", fields.any { it.name == "full_name" })

        // 2. Fill fields
        val valuesJson = """{"full_name":"Mehmet Demir","terms_accepted":"Yes"}"""
        val success = bridge.fillFormFields(
            inputPath = inFile.absolutePath,
            outputPath = outFile.absolutePath,
            fieldValuesJson = valuesJson,
            flatten = true
        )

        assertTrue("Filling form fields must return true", success)
        assertTrue("Filled form output file must exist", outFile.exists())
    }

    @Test
    fun testStampPositionEnum() {
        assertEquals(0, StampPosition.TOP_LEFT.value)
        assertEquals(1, StampPosition.TOP_CENTER.value)
        assertEquals(2, StampPosition.TOP_RIGHT.value)
        assertEquals(3, StampPosition.BOTTOM_LEFT.value)
        assertEquals(4, StampPosition.BOTTOM_CENTER.value)
        assertEquals(5, StampPosition.BOTTOM_RIGHT.value)

        assertEquals(StampPosition.TOP_LEFT, StampPosition.fromInt(0))
        assertEquals(StampPosition.BOTTOM_CENTER, StampPosition.fromInt(4))
        assertEquals(StampPosition.BOTTOM_RIGHT, StampPosition.fromInt(5))
        // Fallback on invalid
        assertEquals(StampPosition.BOTTOM_CENTER, StampPosition.fromInt(99))
    }
}
