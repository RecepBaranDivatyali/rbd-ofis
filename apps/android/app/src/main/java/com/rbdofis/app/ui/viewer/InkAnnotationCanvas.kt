package com.rbdofis.app.ui.viewer

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Redo
import androidx.compose.material.icons.automirrored.filled.Undo
import androidx.compose.material.icons.filled.AutoFixNormal
import androidx.compose.material.icons.filled.BorderColor
import androidx.compose.material.icons.filled.Clear
import androidx.compose.material.icons.filled.DeleteSweep
import androidx.compose.material.icons.filled.Edit
import androidx.compose.material.icons.filled.Highlight
import androidx.compose.material.icons.filled.PanTool
import androidx.compose.material3.DividerDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.VerticalDivider
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.PointerType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import com.rbdofis.app.R
import com.rbdofis.app.core.PageRect
import com.rbdofis.app.core.PdfAnnotation
import java.util.UUID
import kotlin.math.max
import kotlin.math.min
import kotlin.math.sqrt

/**
 * Supported tools in ink annotation mode.
 */
enum class InkTool {
    PAN,
    PEN,
    HIGHLIGHTER,
    ERASER
}

/**
 * Predefined ink colors.
 */
object InkPalette {
    val Black = Color(0xFF000000)
    val Blue = Color(0xFF1976D2)
    val Red = Color(0xFFD32F2F)

    // Highlighter colors (semi-transparent alpha ~0.4)
    val YellowHighlight = Color(0x66FFEB3B)
    val GreenHighlight = Color(0x664CAF50)
    val PinkHighlight = Color(0x66E91E63)
}

/**
 * Lightweight pure Kotlin 2D point representation for geometry math and unit tests.
 */
data class CanvasPoint(val x: Float, val y: Float) {
    fun distanceTo(other: CanvasPoint): Float {
        val dx = x - other.x
        val dy = y - other.y
        return sqrt(dx * dx + dy * dy)
    }
}

/**
 * Pure mathematical algorithms for Bézier smoothing, coordinate normalization, and eraser hit testing.
 */
object InkGeometry {

    /**
     * Constructs a smooth [Path] through raw screen points using Quadratic Bézier curves.
     * Intermediate points use midpoints as endpoints and raw points as control handles.
     */
    fun createSmoothedPath(points: List<Offset>): Path {
        val path = Path()
        if (points.isEmpty()) return path

        path.moveTo(points[0].x, points[0].y)
        if (points.size == 1) {
            path.lineTo(points[0].x + 0.1f, points[0].y + 0.1f)
            return path
        }
        if (points.size == 2) {
            path.lineTo(points[1].x, points[1].y)
            return path
        }

        for (i in 1 until points.size - 1) {
            val current = points[i]
            val next = points[i + 1]
            val midX = (current.x + next.x) / 2f
            val midY = (current.y + next.y) / 2f
            path.quadraticTo(current.x, current.y, midX, midY)
        }

        val last = points.last()
        path.lineTo(last.x, last.y)
        return path
    }

    /**
     * Converts screen touch coordinate to normalized [0.0, 1.0] coordinates relative to canvas width and height.
     */
    fun toNormalized(screenX: Float, screenY: Float, width: Float, height: Float): Pair<Float, Float> {
        if (width <= 0f || height <= 0f) return Pair(0f, 0f)
        val nx = (screenX / width).coerceIn(0f, 1f)
        val ny = (screenY / height).coerceIn(0f, 1f)
        return Pair(nx, ny)
    }

    fun toNormalized(point: CanvasPoint, width: Float, height: Float): Pair<Float, Float> {
        return toNormalized(point.x, point.y, width, height)
    }

    fun toNormalized(screenPoint: Offset, size: Size): Pair<Float, Float> {
        return toNormalized(screenPoint.x, screenPoint.y, size.width, size.height)
    }

    /**
     * Converts normalized [0.0, 1.0] coordinate back to screen pixel coordinates.
     */
    fun toScreenPoint(normalized: Pair<Float, Float>, width: Float, height: Float): CanvasPoint {
        return CanvasPoint(normalized.first * width, normalized.second * height)
    }

    fun toScreen(normalized: Pair<Float, Float>, size: Size): Offset {
        return Offset(normalized.first * size.width, normalized.second * size.height)
    }

    /**
     * Calculates the minimum perpendicular distance from test point ([px], [py]) to line segment ([ax], [ay]) -> ([bx], [by]).
     */
    fun distanceToSegment(px: Float, py: Float, ax: Float, ay: Float, bx: Float, by: Float): Float {
        val dx = bx - ax
        val dy = by - ay
        val lengthSq = dx * dx + dy * dy
        if (lengthSq == 0f) {
            val ddx = px - ax
            val ddy = py - ay
            return sqrt(ddx * ddx + ddy * ddy)
        }

        val t = (((px - ax) * dx + (py - ay) * dy) / lengthSq).coerceIn(0f, 1f)
        val projX = ax + t * dx
        val projY = ay + t * dy
        val distDx = px - projX
        val distDy = py - projY
        return sqrt(distDx * distDx + distDy * distDy)
    }

    fun distanceToSegment(p: CanvasPoint, a: CanvasPoint, b: CanvasPoint): Float {
        return distanceToSegment(p.x, p.y, a.x, a.y, b.x, b.y)
    }

    fun distanceToSegment(p: Offset, a: Offset, b: Offset): Float {
        return distanceToSegment(p.x, p.y, a.x, a.y, b.x, b.y)
    }

    /**
     * Checks if a point [p] intersects a polyline stroke within [thresholdPx].
     */
    fun strokeIntersectsPoint(strokePoints: List<CanvasPoint>, p: CanvasPoint, thresholdPx: Float): Boolean {
        if (strokePoints.isEmpty()) return false
        if (strokePoints.size == 1) {
            return strokePoints[0].distanceTo(p) <= thresholdPx
        }
        for (i in 0 until strokePoints.size - 1) {
            val dist = distanceToSegment(p, strokePoints[i], strokePoints[i + 1])
            if (dist <= thresholdPx) return true
        }
        return false
    }

    fun strokeIntersectsOffset(strokePoints: List<Offset>, p: Offset, thresholdPx: Float): Boolean {
        if (strokePoints.isEmpty()) return false
        if (strokePoints.size == 1) {
            val d = strokePoints[0] - p
            return d.getDistance() <= thresholdPx
        }
        for (i in 0 until strokePoints.size - 1) {
            val dist = distanceToSegment(p, strokePoints[i], strokePoints[i + 1])
            if (dist <= thresholdPx) return true
        }
        return false
    }

    /**
     * Computes the bounding rectangle [PageRect] from a list of normalized coordinates.
     */
    fun computeBounds(points: List<Pair<Float, Float>>): PageRect {
        if (points.isEmpty()) return PageRect(0f, 0f, 0f, 0f)
        var minX = points[0].first
        var maxX = points[0].first
        var minY = points[0].second
        var maxY = points[0].second

        for (pt in points) {
            minX = min(minX, pt.first)
            maxX = max(maxX, pt.first)
            minY = min(minY, pt.second)
            maxY = max(maxY, pt.second)
        }
        return PageRect(minX, minY, maxX, maxY)
    }
}

/**
 * Overlay canvas supporting high-performance stylus/touch drawing, Bézier smoothing, and erasing.
 */
@Composable
fun InkAnnotationCanvas(
    pageIndex: Int,
    annotations: List<PdfAnnotation>,
    activeTool: InkTool,
    penColor: Color,
    strokeWidthPx: Float,
    onAnnotationAdded: (PdfAnnotation.InkStroke) -> Unit,
    onAnnotationRemoved: (String) -> Unit,
    modifier: Modifier = Modifier
) {
    var canvasSize by remember { mutableStateOf(Size.Zero) }
    val currentPoints = remember { mutableStateListOf<Offset>() }
    var isStylusInput by remember { mutableStateOf(false) }

    Box(modifier = modifier.fillMaxSize()) {
        Canvas(
            modifier = Modifier
                .fillMaxSize()
                .pointerInput(activeTool, penColor, strokeWidthPx) {
                    if (activeTool == InkTool.PAN) return@pointerInput

                    awaitPointerEventScope {
                        while (true) {
                            val event = awaitPointerEvent()
                            val change = event.changes.firstOrNull() ?: continue
                            isStylusInput = change.type == PointerType.Stylus || change.type == PointerType.Eraser

                            when (activeTool) {
                                InkTool.PEN, InkTool.HIGHLIGHTER -> {
                                    if (change.pressed) {
                                        currentPoints.add(change.position)
                                        change.consume()
                                    } else if (currentPoints.isNotEmpty()) {
                                        // Gesture finished - commit stroke
                                        if (canvasSize.width > 0f && canvasSize.height > 0f) {
                                            val normalizedPoints = currentPoints.map {
                                                InkGeometry.toNormalized(it, canvasSize)
                                            }
                                            val bounds = InkGeometry.computeBounds(normalizedPoints)
                                            val isHighlighter = activeTool == InkTool.HIGHLIGHTER
                                            val stroke = PdfAnnotation.InkStroke(
                                                id = UUID.randomUUID().toString(),
                                                pageIndex = pageIndex,
                                                bounds = bounds,
                                                points = normalizedPoints,
                                                strokeWidth = if (isHighlighter) strokeWidthPx * 3f else strokeWidthPx,
                                                colorRgba = penColor.value.toLong(),
                                                isHighlighter = isHighlighter
                                            )
                                            onAnnotationAdded(stroke)
                                        }
                                        currentPoints.clear()
                                        change.consume()
                                    }
                                }

                                InkTool.ERASER -> {
                                    if (change.pressed && canvasSize.width > 0f && canvasSize.height > 0f) {
                                        val hitPos = change.position
                                        val eraserThreshold = strokeWidthPx.coerceAtLeast(24f)

                                        // Hit test existing ink strokes
                                        val strokeAnnotations = annotations.filterIsInstance<PdfAnnotation.InkStroke>()
                                        for (annot in strokeAnnotations) {
                                            val screenPts = annot.points.map {
                                                InkGeometry.toScreen(it, canvasSize)
                                            }
                                            if (InkGeometry.strokeIntersectsOffset(screenPts, hitPos, eraserThreshold)) {
                                                onAnnotationRemoved(annot.id)
                                                break
                                            }
                                        }
                                        change.consume()
                                    }
                                }

                                else -> Unit
                            }
                        }
                    }
                }
        ) {
            canvasSize = size

            // 1. Draw existing ink strokes
            val inkAnnotations = annotations.filterIsInstance<PdfAnnotation.InkStroke>()
            for (stroke in inkAnnotations) {
                if (stroke.points.isEmpty()) continue
                val screenPts = stroke.points.map { InkGeometry.toScreen(it, size) }
                val path = InkGeometry.createSmoothedPath(screenPts)
                val color = Color(stroke.colorRgba.toULong())

                drawPath(
                    path = path,
                    color = color,
                    style = Stroke(
                        width = stroke.strokeWidth,
                        cap = StrokeCap.Round,
                        join = StrokeJoin.Round
                    )
                )
            }

            // 2. Draw active in-progress stroke
            if (currentPoints.isNotEmpty()) {
                val path = InkGeometry.createSmoothedPath(currentPoints)
                val effectiveWidth = if (activeTool == InkTool.HIGHLIGHTER) strokeWidthPx * 3f else strokeWidthPx
                drawPath(
                    path = path,
                    color = penColor,
                    style = Stroke(
                        width = effectiveWidth,
                        cap = StrokeCap.Round,
                        join = StrokeJoin.Round
                    )
                )
            }
        }
    }
}

/**
 * Mobile-optimized bottom/floating toolbar for ink annotations.
 */
@Composable
fun InkToolBar(
    activeTool: InkTool,
    onToolSelected: (InkTool) -> Unit,
    activeColor: Color,
    onColorSelected: (Color) -> Unit,
    strokeWidthPx: Float,
    onStrokeWidthSelected: (Float) -> Unit,
    canUndo: Boolean,
    onUndo: () -> Unit,
    canRedo: Boolean,
    onRedo: () -> Unit,
    onClearAll: () -> Unit,
    modifier: Modifier = Modifier
) {
    Surface(
        modifier = modifier
            .fillMaxWidth()
            .padding(8.dp),
        shape = RoundedCornerShape(16.dp),
        tonalElevation = 6.dp,
        shadowElevation = 8.dp
    ) {
        Column(
            modifier = Modifier.padding(8.dp),
            verticalArrangement = Arrangement.spacedBy(6.dp)
        ) {
            // Row 1: Tool Selection & History (Undo / Redo / Clear)
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                // Navigation mode
                IconButton(
                    onClick = { onToolSelected(InkTool.PAN) }
                ) {
                    Icon(
                        imageVector = Icons.Default.PanTool,
                        contentDescription = stringResource(R.string.tool_pan),
                        tint = if (activeTool == InkTool.PAN) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }

                // Pen mode
                IconButton(
                    onClick = {
                        onToolSelected(InkTool.PEN)
                        onColorSelected(InkPalette.Black)
                    }
                ) {
                    Icon(
                        imageVector = Icons.Default.Edit,
                        contentDescription = stringResource(R.string.tool_pen),
                        tint = if (activeTool == InkTool.PEN) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }

                // Highlighter mode
                IconButton(
                    onClick = {
                        onToolSelected(InkTool.HIGHLIGHTER)
                        onColorSelected(InkPalette.YellowHighlight)
                    }
                ) {
                    Icon(
                        imageVector = Icons.Default.Highlight,
                        contentDescription = stringResource(R.string.tool_highlighter),
                        tint = if (activeTool == InkTool.HIGHLIGHTER) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }

                // Eraser mode
                IconButton(
                    onClick = { onToolSelected(InkTool.ERASER) }
                ) {
                    Icon(
                        imageVector = Icons.Default.AutoFixNormal,
                        contentDescription = stringResource(R.string.tool_eraser),
                        tint = if (activeTool == InkTool.ERASER) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant
                    )
                }

                VerticalDivider(modifier = Modifier.height(24.dp))

                // Undo
                IconButton(
                    onClick = onUndo,
                    enabled = canUndo
                ) {
                    Icon(
                        imageVector = Icons.AutoMirrored.Filled.Undo,
                        contentDescription = stringResource(R.string.action_undo)
                    )
                }

                // Redo
                IconButton(
                    onClick = onRedo,
                    enabled = canRedo
                ) {
                    Icon(
                        imageVector = Icons.AutoMirrored.Filled.Redo,
                        contentDescription = stringResource(R.string.action_redo)
                    )
                }

                // Clear all
                IconButton(
                    onClick = onClearAll
                ) {
                    Icon(
                        imageVector = Icons.Default.DeleteSweep,
                        contentDescription = stringResource(R.string.action_clear_canvas),
                        tint = MaterialTheme.colorScheme.error
                    )
                }
            }

            // Row 2: Color Chips & Stroke Widths (shown only in Pen or Highlighter mode)
            AnimatedVisibility(
                visible = activeTool == InkTool.PEN || activeTool == InkTool.HIGHLIGHTER,
                enter = fadeIn(),
                exit = fadeOut()
            ) {
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .padding(horizontal = 8.dp),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    // Colors
                    val colors = if (activeTool == InkTool.PEN) {
                        listOf(InkPalette.Black, InkPalette.Blue, InkPalette.Red)
                    } else {
                        listOf(InkPalette.YellowHighlight, InkPalette.GreenHighlight, InkPalette.PinkHighlight)
                    }

                    Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                        for (c in colors) {
                            val isSelected = activeColor == c
                            Box(
                                modifier = Modifier
                                    .size(28.dp)
                                    .clip(CircleShape)
                                    .background(c)
                                    .border(
                                        width = if (isSelected) 3.dp else 1.dp,
                                        color = if (isSelected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.outline,
                                        shape = CircleShape
                                    )
                                    .clickable { onColorSelected(c) }
                            )
                        }
                    }

                    // Stroke Width Indicators (Thin 2dp, Medium 4dp, Thick 8dp)
                    val widths = listOf(2f, 4f, 8f)
                    Row(
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                        verticalAlignment = Alignment.CenterVertically
                    ) {
                        for (w in widths) {
                            val isSelected = strokeWidthPx == w
                            Box(
                                modifier = Modifier
                                    .size(28.dp)
                                    .clip(RoundedCornerShape(6.dp))
                                    .background(if (isSelected) MaterialTheme.colorScheme.primaryContainer else Color.Transparent)
                                    .clickable { onStrokeWidthSelected(w) },
                                contentAlignment = Alignment.Center
                            ) {
                                Box(
                                    modifier = Modifier
                                        .size((w * 2.5f).dp)
                                        .clip(CircleShape)
                                        .background(if (isSelected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant)
                                )
                            }
                        }
                    }
                }
            }
        }
    }
}
