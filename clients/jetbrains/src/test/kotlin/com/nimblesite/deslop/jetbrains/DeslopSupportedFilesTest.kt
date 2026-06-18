package com.nimblesite.deslop.jetbrains

import kotlin.test.Test
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
}
