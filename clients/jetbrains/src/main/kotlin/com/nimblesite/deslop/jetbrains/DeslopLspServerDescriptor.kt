package com.nimblesite.deslop.jetbrains

import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.openapi.components.service
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.platform.lsp.api.ProjectWideLspServerDescriptor
import java.nio.file.Path

internal class DeslopLspServerDescriptor(
    project: Project,
    private val binary: DeslopResolvedBinary,
    private val settings: DeslopLaunchSettings = project.service<DeslopSettings>().launchSettings(),
) :
    ProjectWideLspServerDescriptor(project, "Deslop") {

    override fun isSupportedFile(file: VirtualFile): Boolean {
        return DeslopSupportedFiles.includes(file)
    }

    override fun createCommandLine(): GeneralCommandLine {
        val workspaceRoot = workspaceRoot()
        return GeneralCommandLine(binary.path.toString())
            .withParameters(buildLspParameters(workspaceRoot, settings))
            .withWorkDirectory(workspaceRoot.toFile())
    }

    private fun workspaceRoot(): Path {
        val basePath = project.basePath ?: System.getProperty("user.dir")
        return Path.of(basePath)
    }
}

internal fun buildLspParameters(
    workspaceRoot: Path,
    settings: DeslopLaunchSettings,
): List<String> {
    return listOf(
        workspaceRoot.toString(),
        "--min-nodes",
        settings.minNodes.toString(),
        "--embeddings",
        settings.embeddingMode,
        "--embedding-provider",
        settings.embeddingProvider,
        "--embedding-model",
        settings.embeddingModel,
        "--embedding-endpoint",
        settings.embeddingEndpoint,
    )
}
