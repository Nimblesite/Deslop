package com.nimblesite.deslop.jetbrains

import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.platform.lsp.api.ProjectWideLspServerDescriptor
import java.nio.file.Path

internal class DeslopLspServerDescriptor(
    project: Project,
    private val binary: DeslopResolvedBinary,
) :
    ProjectWideLspServerDescriptor(project, "Deslop") {

    override fun isSupportedFile(file: VirtualFile): Boolean {
        return DeslopSupportedFiles.includes(file)
    }

    override fun createCommandLine(): GeneralCommandLine {
        val workspaceRoot = workspaceRoot()
        return GeneralCommandLine(binary.path.toString())
            .withParameters(workspaceRoot.toString(), "--min-nodes", "30", "--embeddings", "off")
            .withWorkDirectory(workspaceRoot.toFile())
    }

    private fun workspaceRoot(): Path {
        val basePath = project.basePath ?: System.getProperty("user.dir")
        return Path.of(basePath)
    }
}
