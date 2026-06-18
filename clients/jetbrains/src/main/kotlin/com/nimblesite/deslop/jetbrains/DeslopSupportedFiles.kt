package com.nimblesite.deslop.jetbrains

import com.intellij.openapi.vfs.VirtualFile

internal object DeslopSupportedFiles {
    private val extensions = setOf("cs", "rs", "py", "dart")

    fun includes(file: VirtualFile): Boolean =
        !file.isDirectory && supportsExtension(file.extension)

    /**
     * True when [extension] (case-insensitive, leading dot already stripped by
     * [VirtualFile.getExtension]) maps to a language the Deslop LSP analyses.
     *
     * Extracted as a pure predicate so the supported-language contract is
     * unit-testable without an IntelliJ VirtualFile fixture, mirroring the
     * [buildLspParameters] helper. Keep this set in lockstep with the languages
     * deslop-lsp actually parses — a file type the LSP analyses but this set
     * omits silently denies analysis in JetBrains IDEs.
     */
    fun supportsExtension(extension: String?): Boolean =
        extension?.lowercase() in extensions
}
