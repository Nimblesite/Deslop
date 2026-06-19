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
            runCatching { DeslopBinaryResolver.resolveLsp(ULTIMATE_PLUGIN_ID) }
                .onSuccess { serverStarter.ensureServerStarted(DeslopLspServerDescriptor(project, it)) }
                .onFailure { DeslopStartupNotifier.show(project, it.message.orEmpty()) }
        }
    }

    private companion object {
        /** Plugin id of this native-LSP artifact; the resolver locates its bundled binary by it. */
        const val ULTIMATE_PLUGIN_ID = "nimblesite.deslop.jetbrains"
    }
}
