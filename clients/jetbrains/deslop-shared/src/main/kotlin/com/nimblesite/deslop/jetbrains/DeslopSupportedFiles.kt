package com.nimblesite.deslop.jetbrains

import com.intellij.openapi.vfs.VirtualFile

object DeslopSupportedFiles {
    /** A language's display name and the file extensions that reach it. */
    private data class LanguageInfo(val label: String, val fileExtensions: Set<String>)

    /**
     * Engine language id → display name and file extensions: the single source of
     * truth for [extensions] (which files start the server), for [languageLabel]
     * (an extension's display name), and for [languageName] (the display name of
     * the language id the engine stamps on a cluster). The ids are the parser
     * registry's own ([PIPELINE-LANG-TRAIT]), so a cluster's language is looked up
     * here rather than re-derived from its path. Adding a language here extends
     * analysis start-up and grouping together, so the surfaces cannot drift apart.
     */
    private val languages: Map<String, LanguageInfo> = mapOf(
        "csharp" to LanguageInfo("C#", setOf("cs")),
        "rust" to LanguageInfo("Rust", setOf("rs")),
        "python" to LanguageInfo("Python", setOf("py")),
        "dart" to LanguageInfo("Dart", setOf("dart")),
        "javascript" to LanguageInfo("JavaScript", setOf("js", "mjs", "cjs", "jsx")),
        "typescript" to LanguageInfo("TypeScript", setOf("ts")),
        "tsx" to LanguageInfo("TypeScript", setOf("tsx")),
        "php" to LanguageInfo("PHP", setOf("php")),
        "fsharp" to LanguageInfo("F#", setOf("fs", "fsx")),
        "go" to LanguageInfo("Go", setOf("go")),
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
    val extensions: Set<String> = languages.values.flatMapTo(mutableSetOf(), LanguageInfo::fileExtensions)

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
    fun languageLabel(extension: String?): String {
        val normalised = extension?.lowercase() ?: return OTHER_LANGUAGE
        return languages.values.firstOrNull { normalised in it.fileExtensions }?.label ?: OTHER_LANGUAGE
    }

    /**
     * Human display name for the language id the engine stamped on a cluster
     * ([PIPELINE-LANG-TRAIT]). Unknown or absent ids map to [OTHER_LANGUAGE] so
     * every cluster lands in a group — including one from a report written
     * before the engine carried the field.
     */
    fun languageName(languageId: String?): String =
        languages[languageId?.lowercase()]?.label ?: OTHER_LANGUAGE
}
