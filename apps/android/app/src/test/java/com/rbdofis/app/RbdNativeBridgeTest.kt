package com.rbdofis.app

import com.rbdofis.app.core.PageRect
import com.rbdofis.app.core.PdfAnnotation
import com.rbdofis.app.core.RbdNativeBridge
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import java.io.File

/**
 * Mock-based fallback unit tests for [RbdNativeBridge].
 * These tests force mock mode via `bridge.setForceMockMode(true)` and verify
 * in-memory fallback/simulation behavior when the native Rust core library is disabled or unavailable.
 * They do NOT assert real native Rust core behavior. Real native behavior is tested in [NativeCoreIntegrationTest].
 */
class RbdNativeBridgeTest {

    private lateinit var bridge: RbdNativeBridge

    @Before
    fun setUp() {
        bridge = RbdNativeBridge.getInstance()
        bridge.setForceMockMode(true)
    }

    @Test
    fun testMockOpenDocumentReturnsValidHandle() {
        val handle = bridge.openDocument("/tmp/sample.pdf")
        assertTrue("Handle should be a positive non-zero value in mock mode", handle > 0L)

        val pageCount = bridge.getPageCount(handle)
        assertTrue("Page count should be positive in mock mode", pageCount > 0)

        bridge.closeDocument(handle)
        assertEquals(0, bridge.getPageCount(handle))
    }

    @Test
    fun testMockOpenDocumentFromBytes() {
        val sampleBytes = "%PDF-1.7 mock content".toByteArray(Charsets.UTF_8)
        val handle = bridge.openDocumentFromBytes(sampleBytes)
        assertTrue(handle > 0L)

        val count = bridge.getPageCount(handle)
        assertEquals(5, count)

        bridge.closeDocument(handle)
    }

    @Test
    fun testMockPageDimensionsAndRotation() {
        val handle = bridge.openDocument("/test/a4.pdf")
        val initialSize = bridge.getPageSize(handle, 0)
        assertEquals(595.28f, initialSize.width, 0.1f)
        assertEquals(841.89f, initialSize.height, 0.1f)

        // Rotate 90 degrees clockwise -> width and height swap in mock mode
        val rotated = bridge.rotatePage(handle, 0, 90)
        assertTrue(rotated)

        val rotatedSize = bridge.getPageSize(handle, 0)
        assertEquals(841.89f, rotatedSize.width, 0.1f)
        assertEquals(595.28f, rotatedSize.height, 0.1f)

        // Rotate another 90 degrees (180 total) -> back to portrait
        bridge.rotatePage(handle, 0, 90)
        val portraitSize = bridge.getPageSize(handle, 0)
        assertEquals(595.28f, portraitSize.width, 0.1f)
        assertEquals(841.89f, portraitSize.height, 0.1f)

        bridge.closeDocument(handle)
    }

    @Test
    fun testMockDeletePage() {
        val handle = bridge.openDocument("/test/multi.pdf")
        val initialCount = bridge.getPageCount(handle)
        assertTrue(initialCount >= 2)

        val success = bridge.deletePage(handle, 0)
        assertTrue(success)
        assertEquals(initialCount - 1, bridge.getPageCount(handle))

        // Deleting invalid page index should return false
        val invalidDelete = bridge.deletePage(handle, 999)
        assertFalse(invalidDelete)

        bridge.closeDocument(handle)
    }

    @Test
    fun testMockReorderPage() {
        val handle = bridge.openDocument("/test/reorder.pdf")
        val count = bridge.getPageCount(handle)
        assertTrue(count >= 3)

        val reordered = bridge.reorderPage(handle, 0, 2)
        assertTrue(reordered)
        assertEquals(count, bridge.getPageCount(handle))

        bridge.closeDocument(handle)
    }

    @Test
    fun testMockSearchText() {
        val handle = bridge.openDocument("/test/search.pdf")
        val results = bridge.searchText(handle, "test")
        assertNotNull(results)
        assertTrue("Search should return matches in mock mode", results.isNotEmpty())

        val emptyResults = bridge.searchText(handle, "")
        assertTrue(emptyResults.isEmpty())

        bridge.closeDocument(handle)
    }

    @Test
    fun testMockAnnotations() {
        val handle = bridge.openDocument("/test/annot.pdf")
        val annotation = PdfAnnotation.Highlight(
            id = "highlight-1",
            pageIndex = 0,
            bounds = PageRect(100f, 150f, 300f, 170f)
        )

        val added = bridge.addAnnotation(handle, 0, annotation)
        assertTrue(added)

        val annotations = bridge.getAnnotations(handle, 0)
        assertEquals(1, annotations.size)
        assertEquals("highlight-1", annotations[0].id)

        bridge.closeDocument(handle)
    }

    @Test
    fun testMockSaveDocument() {
        val handle = bridge.openDocument("/test/save.pdf")
        val tempFile = File.createTempFile("rbd_test_save_mock", ".pdf")
        try {
            val saved = bridge.saveDocument(handle, tempFile.absolutePath)
            assertTrue(saved)
            assertTrue(tempFile.exists())
            assertTrue(tempFile.length() > 0)
        } finally {
            tempFile.delete()
            bridge.closeDocument(handle)
        }
    }

    @Test
    fun testMockFileBasedPageOperations() {
        val tempIn = File.createTempFile("rbd_op_in_mock", ".pdf")
        val tempOut = File.createTempFile("rbd_op_out_mock", ".pdf")
        try {
            tempIn.writeText("%PDF-1.7 input mock")

            assertTrue(bridge.pageOpRotate(tempIn.absolutePath, tempOut.absolutePath, 90))
            assertTrue(bridge.pageOpDeletePages(tempIn.absolutePath, tempOut.absolutePath, listOf(1)))
            assertTrue(bridge.pageOpMerge(tempOut.absolutePath, listOf(tempIn.absolutePath)))
            assertTrue(bridge.pageOpSplit(tempIn.absolutePath, tempOut.absolutePath, 1, 1))
            assertTrue(bridge.pageOpReorder(tempIn.absolutePath, tempOut.absolutePath, listOf(1)))
            assertTrue(bridge.pageOpCrop(tempIn.absolutePath, tempOut.absolutePath, 1, 0f, 0f, 100f, 100f))
            assertTrue(bridge.pageOpInsertBlank(tempIn.absolutePath, tempOut.absolutePath, 1, 595f, 842f))
            assertTrue(bridge.pageOpDecrypt(tempIn.absolutePath, tempOut.absolutePath, "secret"))
        } finally {
            tempIn.delete()
            tempOut.delete()
        }
    }

    @Test
    fun testMockEngineLifecycle() {
        bridge.freeEngine()
        assertEquals(0L, bridge.getEngineHandle())
    }

    @Test
    fun testMockNumericAnnotationRemoval() {
        val handle = bridge.openDocument("/test/annot_num.pdf")
        val added = bridge.addAnnotationJson(handle, """{"id":999,"page_index":0,"kind":"Highlight","rect":{"x0":0,"y0":0,"x1":10,"y1":10},"color_rgba":[255,0,0,255],"contents":""}""")
        assertTrue(added)
        val removed = bridge.removeAnnotation(handle, 999L)
        assertTrue(removed)
        bridge.closeDocument(handle)
    }

    @Test
    fun testMockScanBridgeOperations() {
        val histogram = IntArray(256)
        histogram[40] = 500
        histogram[220] = 1000
        val threshold = bridge.scanCalculateOtsuThreshold(histogram, 1500)
        assertTrue(threshold in 41..219)

        val tempOut = File.createTempFile("rbd_scan_out_mock", ".pdf")
        try {
            val generated = bridge.scanGeneratePdf(listOf("/mock/img1.png"), tempOut.absolutePath)
            assertTrue(generated)
            assertTrue(tempOut.exists())
        } finally {
            tempOut.delete()
        }
    }
}
