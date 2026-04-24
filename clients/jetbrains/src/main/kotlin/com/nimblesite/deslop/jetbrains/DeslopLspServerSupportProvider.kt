package com.nimblesite.deslop.jetbrains

import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.platform.lsp.api.LspServerSupportProvider

internal class DeslopLspServerSupportProvider : LspServerSupportProvider {
    override fun fileOpened(
        project: Project,
        file: VirtualFile,
        serverStarter: LspServerSupportProvider.LspServerStarter,
    ) {
        if (DeslopSupportedFiles.includes(file)) {
            serverStarter.ensureServerStarted(DeslopLspServerDescriptor(project))
        }
    }
}
