package com.nimblesite.deslop.jetbrains

import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.openapi.project.Project
import java.nio.file.Path

/**
 * The deslop-lsp launch arguments shared by every JetBrains surface: the workspace
 * root only. Embedding/min-node settings are read by the LSP from `.deslop.toml`,
 * never passed as flags (issue #83), so [settings] is intentionally unused but kept
 * for call-site symmetry and future use.
 */
internal fun buildLspParameters(
    workspaceRoot: Path,
    @Suppress("UNUSED_PARAMETER") settings: DeslopLaunchSettings,
): List<String> = listOf(workspaceRoot.toString())

/** The project's workspace root, falling back to the process working directory. */
internal fun deslopWorkspaceRoot(project: Project): Path =
    Path.of(project.basePath ?: System.getProperty("user.dir"))

/**
 * Builds the identical deslop-lsp command line consumed by both the native
 * (ProjectWideLspServerDescriptor) and LSP4IJ (StreamConnectionProvider) surfaces,
 * so the launched process never drifts between IDE families.
 */
fun buildLspCommandLine(
    binary: DeslopResolvedBinary,
    project: Project,
    settings: DeslopLaunchSettings,
): GeneralCommandLine {
    val workspaceRoot = deslopWorkspaceRoot(project)
    return GeneralCommandLine(binary.path.toString())
        .withParameters(buildLspParameters(workspaceRoot, settings))
        .withWorkDirectory(workspaceRoot.toFile())
}
