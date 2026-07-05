package com.nimblesite.deslop.jetbrains

import com.intellij.openapi.actionSystem.ActionUpdateThread
import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.components.service
import com.intellij.openapi.project.Project

private const val NO_REPORT_MESSAGE =
    "Deslop isn't analysing yet — open a supported file to start the server, then try again."
private const val RENDER_FAILED_MESSAGE = "Deslop failed to render the report."

/**
 * The Tools-menu and tool-window-toolbar action that renders the live report through
 * the [DeslopReportRenderer] project service and shows it in the Deslop tool window —
 * the HTML report in the embedded browser, or the native worst-offenders tree where
 * the IDE has no embedded browser. Fetching may block on the LSP, so it runs on a
 * pooled thread; the hop back to the EDT happens inside [showDeslopReport].
 */
internal class DeslopRenderReportAction : AnAction() {

    override fun getActionUpdateThread(): ActionUpdateThread = ActionUpdateThread.BGT

    override fun update(event: AnActionEvent) {
        event.presentation.isEnabledAndVisible = event.project != null
    }

    override fun actionPerformed(event: AnActionEvent) {
        val project = event.project ?: return
        ApplicationManager.getApplication().executeOnPooledThread { renderAndShow(project) }
    }

    private fun renderAndShow(project: Project) {
        val outcome = runCatching { fetchDeslopReportPayload(project.service<DeslopReportRenderer>()) }
        val failure = outcome.exceptionOrNull()
        if (failure != null) {
            DeslopStartupNotifier.show(project, failure.message ?: RENDER_FAILED_MESSAGE)
            return
        }
        val payload = outcome.getOrNull()
        if (payload.isNullOrEmpty()) DeslopStartupNotifier.info(project, NO_REPORT_MESSAGE)
        else showDeslopReport(project, payload, activate = true)
    }
}
