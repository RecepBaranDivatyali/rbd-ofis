package com.rbdofis.app

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.w3c.dom.Element
import java.io.File
import javax.xml.parsers.DocumentBuilderFactory

class LocalizationCompletenessTest {

    private fun findProjectResDir(): File {
        val candidates = listOf(
            File("src/main/res"),
            File("app/src/main/res"),
            File("apps/android/app/src/main/res")
        )
        for (dir in candidates) {
            if (dir.exists() && dir.isDirectory) {
                return dir
            }
        }
        throw IllegalStateException("Resource directory not found in candidates: $candidates")
    }

    private fun parseStringResources(file: File): Map<String, String> {
        assertTrue("Resource file must exist: ${file.absolutePath}", file.exists())
        val factory = DocumentBuilderFactory.newInstance()
        val builder = factory.newDocumentBuilder()
        val doc = builder.parse(file)
        val stringNodes = doc.getElementsByTagName("string")

        val map = mutableMapOf<String, String>()
        for (i in 0 until stringNodes.length) {
            val element = stringNodes.item(i) as Element
            val name = element.getAttribute("name")
            val content = element.textContent
            map[name] = content
        }
        return map
    }

    @Test
    fun testAllStringsLocalizedInTurkishAndEnglish() {
        val resDir = findProjectResDir()
        val defaultStringsFile = File(resDir, "values/strings.xml")
        val trStringsFile = File(resDir, "values-tr/strings.xml")
        val enStringsFile = File(resDir, "values-en/strings.xml")

        assertTrue("Default strings file must exist", defaultStringsFile.exists())
        assertTrue("Turkish (values-tr) strings file must exist", trStringsFile.exists())
        assertTrue("English (values-en) strings file must exist", enStringsFile.exists())

        val defaultStrings = parseStringResources(defaultStringsFile)
        val trStrings = parseStringResources(trStringsFile)
        val enStrings = parseStringResources(enStringsFile)

        assertFalse("Default strings should not be empty", defaultStrings.isEmpty())
        assertFalse("Turkish strings should not be empty", trStrings.isEmpty())
        assertFalse("English strings should not be empty", enStrings.isEmpty())

        // 1. Verify strict 3-way key parity: values == values-tr == values-en
        val missingInEnFromDefault = defaultStrings.keys - enStrings.keys
        assertTrue("Keys missing in English: $missingInEnFromDefault", missingInEnFromDefault.isEmpty())

        val missingInDefaultFromEn = enStrings.keys - defaultStrings.keys
        assertTrue("Keys missing in Default: $missingInDefaultFromEn", missingInDefaultFromEn.isEmpty())

        val missingInTrFromDefault = defaultStrings.keys - trStrings.keys
        assertTrue("Keys missing in values-tr: $missingInTrFromDefault", missingInTrFromDefault.isEmpty())

        val missingInDefaultFromTr = trStrings.keys - defaultStrings.keys
        assertTrue("Keys missing in Default from values-tr: $missingInDefaultFromTr", missingInDefaultFromTr.isEmpty())

        // 2. Verify identical key set size
        assertEquals("values and values-en key counts must match exactly", defaultStrings.size, enStrings.size)
        assertEquals("values and values-tr key counts must match exactly", defaultStrings.size, trStrings.size)

        // 3. Verify no empty values in all 3 catalogs
        for ((key, value) in defaultStrings) {
            assertFalse("Default string '$key' must not be blank", value.isBlank())
        }
        for ((key, value) in trStrings) {
            assertFalse("TR string '$key' must not be blank", value.isBlank())
        }
        for ((key, value) in enStrings) {
            assertFalse("EN string '$key' must not be blank", value.isBlank())
        }
    }

    @Test
    fun testPhase8AiAndOfficeKeysPresent() {
        val resDir = findProjectResDir()
        val defaultStrings = parseStringResources(File(resDir, "values/strings.xml"))
        val trStrings = parseStringResources(File(resDir, "values-tr/strings.xml"))
        val enStrings = parseStringResources(File(resDir, "values-en/strings.xml"))

        val requiredPhase8Keys = listOf(
            "ai_title",
            "ai_badge_offline",
            "ai_tab_summary",
            "ai_tab_ask",
            "ai_word_count",
            "ai_reading_time",
            "ai_section_summary",
            "ai_section_key_sentences",
            "ai_query_placeholder",
            "ai_page_indicator",
            "ai_confidence_score",
            "ai_summary_fallback",
            "ai_answer_fallback",
            "office_badge_offline",
            "office_editor_placeholder"
        )

        for (key in requiredPhase8Keys) {
            assertTrue("Default strings missing required key '$key'", defaultStrings.containsKey(key))
            assertTrue("Turkish strings missing required key '$key'", trStrings.containsKey(key))
            assertTrue("English strings missing required key '$key'", enStrings.containsKey(key))
        }
    }
}
