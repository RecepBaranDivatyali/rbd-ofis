package com.rbdofis.app

import com.rbdofis.app.ui.scanner.DocumentImageProcessor
import com.rbdofis.app.ui.scanner.ScannerFilter
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class DocumentScannerTest {

    @Test
    fun testComputeLuminanceStandardValues() {
        // Pure White: 0.299 * 255 + 0.587 * 255 + 0.114 * 255 = 255
        assertEquals(255, DocumentImageProcessor.computeLuminance(255, 255, 255))

        // Pure Black
        assertEquals(0, DocumentImageProcessor.computeLuminance(0, 0, 0))

        // Pure Red: 0.299 * 255 = 76.245 -> 76
        assertEquals(76, DocumentImageProcessor.computeLuminance(255, 0, 0))

        // Pure Green: 0.587 * 255 = 149.685 -> 149
        assertEquals(149, DocumentImageProcessor.computeLuminance(0, 255, 0))

        // Pure Blue: 0.114 * 255 = 29.07 -> 29
        assertEquals(29, DocumentImageProcessor.computeLuminance(0, 0, 255))
    }

    @Test
    fun testOtsuThresholdBimodalDistribution() {
        // Create a bimodal histogram simulating text on paper:
        // Cluster 1 (Dark ink): 500 pixels at intensity 40
        // Cluster 2 (Bright paper): 1000 pixels at intensity 220
        val histogram = IntArray(256)
        histogram[40] = 500
        histogram[220] = 1000
        val totalPixels = 1500

        val threshold = DocumentImageProcessor.calculateOtsuThreshold(histogram, totalPixels)

        // Threshold must clearly separate the two peaks: 40 < threshold <= 220
        assertTrue("Threshold ($threshold) should be between 40 and 220", threshold in 41..219)
    }

    @Test
    fun testOtsuThresholdUniformDistribution() {
        val histogram = IntArray(256) { 10 }
        val totalPixels = 2560

        val threshold = DocumentImageProcessor.calculateOtsuThreshold(histogram, totalPixels)
        assertTrue("Threshold for uniform distribution should be valid byte", threshold in 0..255)
    }

    @Test
    fun testOtsuThresholdEmptyOrInvalid() {
        val histogram = IntArray(256)
        val threshold = DocumentImageProcessor.calculateOtsuThreshold(histogram, 0)
        assertEquals(128, threshold)
    }

    @Test
    fun testScannerFilterValues() {
        val filters = ScannerFilter.values()
        assertEquals(3, filters.size)
        assertTrue(filters.contains(ScannerFilter.ORIGINAL))
        assertTrue(filters.contains(ScannerFilter.GRAYSCALE))
        assertTrue(filters.contains(ScannerFilter.BINARIZED))
    }
}
