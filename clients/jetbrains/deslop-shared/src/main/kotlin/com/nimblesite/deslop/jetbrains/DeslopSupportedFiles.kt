package com.nimblesite.deslop.jetbrains

import com.intellij.openapi.vfs.VirtualFile

object DeslopSupportedFiles {
    /**
     * The file extensions deslop-lsp analyses. Exposed (not private) so the LSP4IJ
     * `fileNamePatternMapping` glob in the community plugin.xml can be asserted to
     * match exactly — the native surface and the LSP4IJ surface must agree on which
     * files start the server. Keep in lockstep with the languages deslop-lsp parses.
     */
    val extensions = setOf("cs", "rs", "py", "dart")

    fun includes(file: VirtualFile): Boolean =
        !file.isDirectory && supportsExtension(file.extension)

    /**
     * True when [extension] (case-insensitive, leading dot already stripped by
     * [VirtualFile.getExtension]) maps to a language the Deslop LSP analyses. Pure,
     * so the supported-language contract is unit-testable without a VirtualFile
     * fixture, mirroring the [buildLspParameters] helper.
     */
    fun supportsExtension(extension: String?): Boolean =
        extension?.lowercase() in extensions
}
