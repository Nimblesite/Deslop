package com.nimblesite.deslop.jetbrains

import com.intellij.openapi.project.Project
import com.intellij.platform.lsp.api.LspServerManager
import com.intellij.platform.lsp.api.LspServerState
import org.eclipse.lsp4j.ExecuteCommandParams

/**
 * Ultimate/Rider variant of the "Open HTML Report" action. Sends the render
 * command through the native `com.intellij.platform.lsp` client and returns the
 * raw result for the shared base to coerce and display.
 */
internal class DeslopUltimateOpenHtmlReportAction : DeslopOpenHtmlReportAction() {
    override fun executeRenderCommand(project: Project): Any? {
        val server = LspServerManager.getInstance(project)
            .getServersForProvider(DeslopLspServerSupportProvider::class.java)
            .firstOrNull { it.state == LspServerState.Running }
            ?: return null
        return server.sendRequestSync { lsp ->
            lsp.workspaceService.executeCommand(
                ExecuteCommandParams(DESLOP_RENDER_HTML_REPORT_COMMAND, emptyList<Any>()),
            )
        }
    }
}
