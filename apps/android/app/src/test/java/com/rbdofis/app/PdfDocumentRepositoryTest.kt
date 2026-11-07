package com.rbdofis.app

import com.rbdofis.app.core.RbdNativeBridge
import com.rbdofis.app.data.PdfDocumentRepository
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import java.io.File

@OptIn(ExperimentalCoroutinesApi::class)
class PdfDocumentRepositoryTest {

    private lateinit var bridge: RbdNativeBridge
    private lateinit var repository: PdfDocumentRepository
    private val testDispatcher = UnconfinedTestDispatcher()

    @Before
    fun setUp() {
        bridge = RbdNativeBridge.getInstance()
        bridge.setForceMockMode(true)
        repository = PdfDocumentRepository(bridge, testDispatcher)
    }

    @Test
    fun testOpenDocumentFlow() = runTest(testDispatcher) {
        val result = repository.openDocument("/tmp/document.pdf")
        assertTrue(result.isSuccess)

        val state = repository.documentState.value
        assertTrue(state.isOpen)
        assertEquals("document.pdf", state.fileName)
        assertTrue(state.pageCount > 0)
        assertFalse(state.isLoading)

        repository.closeDocument()
        val closedState = repository.documentState.value
        assertFalse(closedState.isOpen)
    }

    @Test
    fun testPageOperationsFlow() = runTest(testDispatcher) {
        repository.openDocument("/tmp/document.pdf")
        val initialPages = repository.getAllPages()
        val initialCount = initialPages.size
        assertTrue(initialCount >= 2)

        // Rotate page
        val rotated = repository.rotatePage(0, 90)
        assertTrue(rotated)
        assertTrue(repository.documentState.value.isModified)

        // Delete page
        val deleted = repository.deletePage(0)
        assertTrue(deleted)
        assertEquals(initialCount - 1, repository.documentState.value.pageCount)

        // Save
        val tempFile = File.createTempFile("repo_test", ".pdf")
        try {
            val saved = repository.saveDocument(tempFile.absolutePath)
            assertTrue(saved)
            assertFalse(repository.documentState.value.isModified)
        } finally {
            tempFile.delete()
            repository.closeDocument()
        }
    }

    @Test
    fun testRepositoryReleaseClosesDocumentAndFreesEngine() = runTest(testDispatcher) {
        val result = repository.openDocument("/tmp/document_release.pdf")
        assertTrue(result.isSuccess)
        assertTrue(repository.documentState.value.isOpen)

        repository.release()
        assertFalse(repository.documentState.value.isOpen)
        assertEquals(0L, bridge.getEngineHandle())
    }
}
