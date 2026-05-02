package com.nimblesite.deslop.jetbrains

import java.nio.file.Path
import kotlin.test.Test
import kotlin.test.assertContains
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse

internal class DeslopSettingsTest {
    private val legacyLspStartupFlags = listOf(
        "--min-nodes",
        "--embeddings",
        "--embedding-provider",
        "--embedding-model",
        "--embedding-endpoint",
    )

    @Test
    fun defaultsKeepFreshInstallEmbeddingsOff() {
        val settings = DeslopSettingsValidator.snapshot(DeslopSettingsState())

        assertEquals(30, settings.minNodes)
        assertEquals("ollama", settings.embeddingProvider)
        assertEquals("nomic-embed-text", settings.embeddingModel)
        assertEquals("http://127.0.0.1:11434", settings.embeddingEndpoint)
        assertEquals("off", settings.embeddingMode)
        assertEquals(true, settings.incremental)
    }

    @Test
    fun validationRejectsInvalidValues() {
        val state = DeslopSettingsState(
            minNodes = 0,
            embeddingProvider = "bogus",
            embeddingModel = " ",
            embeddingEndpoint = "localhost:11434",
            embeddingMode = "maybe",
        )

        val error = assertFailsWith<DeslopSettingsException> {
            DeslopSettingsValidator.snapshot(state)
        }
        val message = error.message.orEmpty()
        assertContains(message, "deslop.minNodes")
        assertContains(message, "deslop.embedding.provider")
        assertContains(message, "deslop.embedding.model")
        assertContains(message, "deslop.embedding.endpoint")
        assertContains(message, "deslop.embedding.mode")
    }

    @Test
    fun descriptorArgumentsKeepIssue83LegacyStartupFlagsOut() {
        val args = buildLspParameters(
            Path.of("/workspace"),
            DeslopLaunchSettings(
                minNodes = 42,
                embeddingProvider = "stub",
                embeddingModel = "blake3-stub",
                embeddingEndpoint = "https://ollama.example.test",
                embeddingMode = "auto",
                incremental = false,
            ),
        )

        assertEquals(listOf("/workspace"), args)
        for (flag in legacyLspStartupFlags) {
            assertFalse(args.contains(flag), "issue #83: JetBrains must not pass legacy $flag in $args")
        }
    }
}
