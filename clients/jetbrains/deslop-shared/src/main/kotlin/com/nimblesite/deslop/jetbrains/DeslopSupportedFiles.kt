package com.nimblesite.deslop.jetbrains

import com.intellij.openapi.vfs.VirtualFile

object DeslopSupportedFiles {
    /**
     * File extension → human display language name: the single source of truth for
     * both [extensions] (derived from these keys) and the Language grouping-axis label
     * in the worst-offenders tree. Adding a language here extends analysis start-up and
     * grouping together, so the two surfaces cannot drift out of sync.
     */
    private val languageLabels: Map<String, String> = mapOf(
        "cs" to "C#",
        "rs" to "Rust",
        "py" to "Python",
        "dart" to "Dart",
        "js" to "JavaScript",
        "mjs" to "JavaScript",
        "cjs" to "JavaScript",
        "jsx" to "JavaScript",
        "ts" to "TypeScript",
        "tsx" to "TypeScript",
    )

    /** Display name for a path whose extension is not one Deslop analyses. */
    const val OTHER_LANGUAGE: String = "Other"

    /**
     * The file extensions deslop-lsp analyses, derived from [languageLabels] keys so
     * the extension list is defined exactly once. Exposed (not private) so the LSP4IJ
     * `fileNamePatternMapping` glob in the community plugin.xml can be asserted to
     * match exactly — the native surface and the LSP4IJ surface must agree on which
     * files start the server. Keep in lockstep with the languages deslop-lsp parses.
     */
    val extensions: Set<String> = languageLabels.keys

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

    /**
     * Human display name for [extension] (case-insensitive; leading dot already
     * stripped): the Language grouping-axis label in the worst-offenders tree. Unknown
     * or absent extensions map to [OTHER_LANGUAGE] so every cluster lands in a group.
     */
    fun languageLabel(extension: String?): String =
        extension?.lowercase()?.let(languageLabels::get) ?: OTHER_LANGUAGE
}
