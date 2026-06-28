package com.nimblesite.deslop.jetbrains

import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.openapi.components.service
import com.intellij.openapi.project.Project
import com.redhat.devtools.lsp4ij.LanguageServerFactory
import com.redhat.devtools.lsp4ij.client.LanguageClientImpl
import com.redhat.devtools.lsp4ij.server.OSProcessStreamConnectionProvider
import com.redhat.devtools.lsp4ij.server.StreamConnectionProvider

/**
 * Registers deslop-lsp with LSP4IJ, the LSP client that ships in Android Studio and
 * IntelliJ Community (which lack the native `com.intellij.platform.lsp` API). It
 * reuses the shared binary resolver, settings, and command-line builder verbatim,
 * so the launched process matches the native [DeslopLspServerDescriptor] exactly.
 */
internal class DeslopLsp4ijServerFactory : LanguageServerFactory {
    override fun createConnectionProvider(project: Project): StreamConnectionProvider {
        val binary = resolveOrNotify(project) { DeslopBinaryResolver.resolveLsp(COMMUNITY_PLUGIN_ID) }
        val settings = resolveOrNotify(project) { project.service<DeslopSettings>().launchSettings() }
        return DeslopConnectionProvider(buildLspCommandLine(binary, project, settings))
    }

    /**
     * Returns the [DeslopLanguageClient] so the tool window refreshes live on the
     * server's `deslop/reportChanged` notification. Without this override LSP4IJ
     * uses a base client that drops the custom notification, leaving the panel
     * render-once.
     */
    override fun createLanguageClient(project: Project): LanguageClientImpl = DeslopLanguageClient(project)

    private fun <T> resolveOrNotify(project: Project, block: () -> T): T =
        runCatching(block)
            .onFailure { DeslopStartupNotifier.show(project, it.message.orEmpty()) }
            .getOrThrow()

    private companion object {
        /** Plugin id of this LSP4IJ artifact; the resolver locates its bundled binary by it. */
        const val COMMUNITY_PLUGIN_ID = "nimblesite.deslop.jetbrains.community"
    }
}

private class DeslopConnectionProvider(commandLine: GeneralCommandLine) :
    OSProcessStreamConnectionProvider() {
    init {
        super.setCommandLine(commandLine)
    }
}
