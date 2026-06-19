package com.nimblesite.deslop.jetbrains

import com.intellij.openapi.project.Project
import com.redhat.devtools.lsp4ij.LanguageServerManager
import org.eclipse.lsp4j.ExecuteCommandParams
import java.util.concurrent.TimeUnit

/**
 * Community/Android Studio variant of the "Open HTML Report" action. Sends the
 * render command through the LSP4IJ client and returns the raw result for the
 * shared base to coerce and display. `getLanguageServer` starts the server if
 * needed, so the action works even before a supported file is open.
 */
internal class DeslopCommunityOpenHtmlReportAction : DeslopOpenHtmlReportAction() {
    override fun executeRenderCommand(project: Project): Any? {
        val item = LanguageServerManager.getInstance(project)
            .getLanguageServer(DESLOP_SERVER_ID)
            .get(REQUEST_TIMEOUT_SECONDS, TimeUnit.SECONDS)
            ?: return null
        return item.server.workspaceService
            .executeCommand(ExecuteCommandParams(DESLOP_RENDER_HTML_REPORT_COMMAND, emptyList<Any>()))
            .get(REQUEST_TIMEOUT_SECONDS, TimeUnit.SECONDS)
    }

    private companion object {
        const val DESLOP_SERVER_ID = "deslop"
        const val REQUEST_TIMEOUT_SECONDS = 15L
    }
}
