package com.nimblesite.deslop.jetbrains

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

internal class DeslopSupportedFilesTest {
    /**
     * deslop-lsp ships Dart as a v1 language (tree-sitter-dart), and Android
     * Studio / Flutter developers live in .dart files. The JetBrains bridge must
     * start the LSP for them exactly as the VS Code client already does. Before
     * the fix this set was {cs, rs, py}, so opening a .dart file in a JetBrains
     * IDE silently produced no analysis.
     */
    @Test
    fun dartFilesAreAnalysed() {
        assertTrue(DeslopSupportedFiles.supportsExtension("dart"), "deslop-lsp analyses Dart")
        assertTrue(DeslopSupportedFiles.supportsExtension("DART"), "extension match is case-insensitive")
    }

    @Test
    fun everyShippingLanguageIsAnalysed() {
        for (extension in listOf("cs", "rs", "py", "dart")) {
            assertTrue(
                DeslopSupportedFiles.supportsExtension(extension),
                "deslop-lsp parses .$extension, so the JetBrains bridge must start for it",
            )
        }
    }

    @Test
    fun extensionsOutsideTheShippingSetStayDormant() {
        for (extension in listOf("ts", "js", "go", "kt", "txt", "md", null)) {
            assertFalse(
                DeslopSupportedFiles.supportsExtension(extension),
                ".$extension is not a shipping Deslop language; the bridge must not start",
            )
        }
    }

    /**
     * The LSP4IJ community plugin decides which files start the server via the
     * `fileNamePatternMapping` glob in its plugin.xml, NOT via [DeslopSupportedFiles].
     * If the two drift, Android Studio silently analyses the wrong set. This pins
     * the glob to the canonical extension set so a new language cannot be added in
     * one place and forgotten in the other.
     */
    @Test
    fun lsp4ijFilePatternsMatchTheSupportedSet() {
        assertEquals(
            DeslopSupportedFiles.extensions,
            lsp4ijFilePatternExtensions(),
            "LSP4IJ fileNamePatternMapping must cover exactly the languages deslop-lsp analyses",
        )
    }

    private fun lsp4ijFilePatternExtensions(): Set<String> {
        val patterns = PluginDescriptor
            .attributeValues(PluginDescriptor.lsp4ij(), "fileNamePatternMapping", "patterns")
            .single()
        return patterns.split(";")
            .map { it.trim().removePrefix("*.").lowercase() }
            .filter { it.isNotEmpty() }
            .toSet()
    }
}
