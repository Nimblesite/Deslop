package com.nimblesite.deslop.jetbrains

import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.openapi.components.service
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.platform.lsp.api.ProjectWideLspServerDescriptor

internal class DeslopLspServerDescriptor(
    project: Project,
    private val binary: DeslopResolvedBinary,
    private val settings: DeslopLaunchSettings = project.service<DeslopSettings>().launchSettings(),
) : ProjectWideLspServerDescriptor(project, "Deslop") {

    override fun isSupportedFile(file: VirtualFile): Boolean = DeslopSupportedFiles.includes(file)

    override fun createCommandLine(): GeneralCommandLine = buildLspCommandLine(binary, project, settings)
}
