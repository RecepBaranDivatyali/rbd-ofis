package com.rbdofis.app

import com.rbdofis.app.core.RbdNativeBridge
import org.junit.Assert.*
import org.junit.Test
import java.io.File

class DocumentAiTest {

    private val bridge = RbdNativeBridge.getInstance()

    @Test
    fun testSummarizeDocument_ReturnsJson() {
        val json = bridge.summarizeDocument(100L, 5)
        assertNotNull(json)
        assertTrue(json.contains("summary_text"))
        assertTrue(json.contains("key_sentences"))
        assertTrue(json.contains("word_count"))
    }

    @Test
    fun testAskDocument_ReturnsAnswerWithCitation() {
        val json = bridge.askDocument(100L, "Gizlilik politikası nedir?")
        assertNotNull(json)
        assertTrue(json.contains("query"))
        assertTrue(json.contains("answer_text"))
        assertTrue(json.contains("page_number"))
        assertTrue(json.contains("confidence"))
    }

    @Test
    fun testReflowPageText_ReturnsParagraphList() {
        val json = bridge.reflowPageText(100L, 0)
        assertNotNull(json)
        assertTrue(json.contains("bbox"))
        assertTrue(json.contains("text"))
    }

    @Test
    fun testReplaceParagraphText_CreatesFile() {
        val tempIn = File.createTempFile("rbd_ai_in", ".pdf")
        val tempOut = File.createTempFile("rbd_ai_out", ".pdf")
        try {
            tempIn.writeText("%PDF-1.7 initial content")
            val success = bridge.replaceParagraphText(
                docHandle = 100L,
                inputPath = tempIn.absolutePath,
                pageIndex = 0,
                x0 = 72f,
                y0 = 600f,
                x1 = 400f,
                y1 = 650f,
                newText = "Yeni akışkan metin",
                fontSize = 12f,
                outputPath = tempOut.absolutePath
            )
            assertTrue(success)
            assertTrue(tempOut.exists())
            assertTrue(tempOut.length() > 0)
        } finally {
            tempIn.delete()
            tempOut.delete()
        }
    }
}
