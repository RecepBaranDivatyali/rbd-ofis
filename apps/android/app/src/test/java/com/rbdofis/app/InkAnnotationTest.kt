package com.rbdofis.app

import com.rbdofis.app.core.PageRect
import com.rbdofis.app.core.PdfAnnotation
import com.rbdofis.app.core.RbdNativeBridge
import com.rbdofis.app.ui.viewer.CanvasPoint
import com.rbdofis.app.ui.viewer.InkGeometry
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test

class InkAnnotationTest {

    private lateinit var bridge: RbdNativeBridge

    @Before
    fun setUp() {
        bridge = RbdNativeBridge.getInstance()
        bridge.setForceMockMode(true)
    }

    @Test
    fun testCoordinateNormalizationAndScreenMapping() {
        val width = 1000f
        val height = 2000f
        val screenPoint = CanvasPoint(250f, 1500f)

        // Screen -> Normalized
        val normalized = InkGeometry.toNormalized(screenPoint, width, height)
        assertEquals(0.25f, normalized.first, 0.0001f)
        assertEquals(0.75f, normalized.second, 0.0001f)

        // Normalized -> Screen
        val restored = InkGeometry.toScreenPoint(normalized, width, height)
        assertEquals(250f, restored.x, 0.0001f)
        assertEquals(1500f, restored.y, 0.0001f)

        // Clamp behavior
        val clampedOut = InkGeometry.toNormalized(-50f, 2500f, width, height)
        assertEquals(0f, clampedOut.first, 0.0001f)
        assertEquals(1f, clampedOut.second, 0.0001f)
    }

    @Test
    fun testDistanceToSegmentPerpendicularAndEndpoints() {
        val a = CanvasPoint(0f, 0f)
        val b = CanvasPoint(100f, 0f)

        // Point directly above the midpoint (50, 20) -> distance should be 20
        val pMid = CanvasPoint(50f, 20f)
        val distMid = InkGeometry.distanceToSegment(pMid, a, b)
        assertEquals(20f, distMid, 0.001f)

        // Point before start endpoint (-10, 0) -> distance to A(0,0) is 10
        val pBefore = CanvasPoint(-10f, 0f)
        val distBefore = InkGeometry.distanceToSegment(pBefore, a, b)
        assertEquals(10f, distBefore, 0.001f)

        // Point beyond end endpoint (130, 40) -> distance to B(100,0) is sqrt(30^2 + 40^2) = 50
        val pAfter = CanvasPoint(130f, 40f)
        val distAfter = InkGeometry.distanceToSegment(pAfter, a, b)
        assertEquals(50f, distAfter, 0.001f)
    }

    @Test
    fun testStrokeIntersectsPointEraserHitTest() {
        val stroke = listOf(
            CanvasPoint(10f, 10f),
            CanvasPoint(50f, 10f),
            CanvasPoint(50f, 80f),
            CanvasPoint(100f, 80f)
        )

        val threshold = 15f

        // Point near (30, 15) -> dist to first horizontal segment is 5 <= 15 -> hit
        assertTrue(InkGeometry.strokeIntersectsPoint(stroke, CanvasPoint(30f, 15f), threshold))

        // Point near (55, 45) -> dist to vertical segment is 5 <= 15 -> hit
        assertTrue(InkGeometry.strokeIntersectsPoint(stroke, CanvasPoint(55f, 45f), threshold))

        // Point far away (200, 200) -> not hit
        assertFalse(InkGeometry.strokeIntersectsPoint(stroke, CanvasPoint(200f, 200f), threshold))
    }

    @Test
    fun testComputeBounds() {
        val points = listOf(
            Pair(0.2f, 0.4f),
            Pair(0.1f, 0.8f),
            Pair(0.7f, 0.3f),
            Pair(0.5f, 0.9f)
        )

        val bounds = InkGeometry.computeBounds(points)
        assertEquals(0.1f, bounds.left, 0.0001f)
        assertEquals(0.3f, bounds.top, 0.0001f)
        assertEquals(0.7f, bounds.right, 0.0001f)
        assertEquals(0.9f, bounds.bottom, 0.0001f)
    }

    @Test
    fun testUndoRedoInkAnnotation() {
        val handle = bridge.openDocument("mock://annotation_test.pdf")
        assertTrue(handle > 0L)

        val initialAnnotations = bridge.getAnnotations(handle, 0)
        assertEquals(0, initialAnnotations.size)
        assertFalse(bridge.canUndo(handle))
        assertFalse(bridge.canRedo(handle))

        // Add an ink stroke
        val stroke = PdfAnnotation.InkStroke(
            id = "stroke-1",
            pageIndex = 0,
            bounds = PageRect(0.1f, 0.1f, 0.5f, 0.5f),
            points = listOf(Pair(0.1f, 0.1f), Pair(0.5f, 0.5f)),
            strokeWidth = 3f,
            colorRgba = 0xFF000000L
        )

        val added = bridge.addAnnotation(handle, 0, stroke)
        assertTrue(added)
        assertEquals(1, bridge.getAnnotations(handle, 0).size)
        assertTrue(bridge.canUndo(handle))
        assertFalse(bridge.canRedo(handle))

        // Undo -> Annotation should be removed
        val undone = bridge.undo(handle)
        assertTrue(undone)
        assertEquals(0, bridge.getAnnotations(handle, 0).size)
        assertFalse(bridge.canUndo(handle))
        assertTrue(bridge.canRedo(handle))

        // Redo -> Annotation should be restored
        val redone = bridge.redo(handle)
        assertTrue(redone)
        val restoredList = bridge.getAnnotations(handle, 0)
        assertEquals(1, restoredList.size)
        assertEquals("stroke-1", restoredList[0].id)
        assertTrue(bridge.canUndo(handle))
        assertFalse(bridge.canRedo(handle))

        bridge.closeDocument(handle)
    }

    @Test
    fun testRemoveAnnotationAndUndo() {
        val handle = bridge.openDocument("mock://remove_test.pdf")
        val stroke1 = PdfAnnotation.InkStroke(
            id = "stroke-A",
            pageIndex = 0,
            bounds = PageRect(0f, 0f, 1f, 1f),
            points = listOf(Pair(0f, 0f), Pair(1f, 1f))
        )
        val stroke2 = PdfAnnotation.InkStroke(
            id = "stroke-B",
            pageIndex = 0,
            bounds = PageRect(0.2f, 0.2f, 0.8f, 0.8f),
            points = listOf(Pair(0.2f, 0.2f), Pair(0.8f, 0.8f))
        )

        bridge.addAnnotation(handle, 0, stroke1)
        bridge.addAnnotation(handle, 0, stroke2)
        assertEquals(2, bridge.getAnnotations(handle, 0).size)

        // Remove stroke-A
        val removed = bridge.removeAnnotation(handle, 0, "stroke-A")
        assertTrue(removed)
        val remaining = bridge.getAnnotations(handle, 0)
        assertEquals(1, remaining.size)
        assertEquals("stroke-B", remaining[0].id)

        // Undo the removal
        bridge.undo(handle)
        val afterUndo = bridge.getAnnotations(handle, 0)
        assertEquals(2, afterUndo.size)
        assertTrue(afterUndo.any { it.id == "stroke-A" })
        assertTrue(afterUndo.any { it.id == "stroke-B" })

        bridge.closeDocument(handle)
    }
}
