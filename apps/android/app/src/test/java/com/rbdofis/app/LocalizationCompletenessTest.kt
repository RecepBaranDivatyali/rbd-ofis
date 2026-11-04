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
        val trStringsFile = File(resDir, "values/strings.xml")
        val enStringsFile = File(resDir, "values-en/strings.xml")

        assertTrue("Turkish strings file must exist", trStringsFile.exists())
        assertTrue("English strings file must exist", enStringsFile.exists())

        val trStrings = parseStringResources(trStringsFile)
        val enStrings = parseStringResources(enStringsFile)

        assertFalse("Turkish strings should not be empty", trStrings.isEmpty())
        assertFalse("English strings should not be empty", enStrings.isEmpty())

        // 1. Verify every TR key is in EN
        val missingInEn = trStrings.keys - enStrings.keys
        assertTrue("Keys missing in English: $missingInEn", missingInEn.isEmpty())

        // 2. Verify every EN key is in TR
        val missingInTr = enStrings.keys - trStrings.keys
        assertTrue("Keys missing in Turkish: $missingInTr", missingInTr.isEmpty())

        // 3. Verify identical key set size
        assertEquals("Key counts must match exactly", trStrings.size, enStrings.size)

        // 4. Verify no empty values
        for ((key, value) in trStrings) {
            assertFalse("TR string '$key' must not be blank", value.isBlank())
        }
        for ((key, value) in enStrings) {
            assertFalse("EN string '$key' must not be blank", value.isBlank())
        }
    }
}
