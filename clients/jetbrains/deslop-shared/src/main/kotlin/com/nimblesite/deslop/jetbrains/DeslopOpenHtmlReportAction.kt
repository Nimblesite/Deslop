package com.nimblesite.deslop.jetbrains

import com.google.gson.JsonElement
import com.google.gson.JsonPrimitive
import com.intellij.openapi.actionSystem.ActionUpdateThread
import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.project.Project

/** `workspace/executeCommand` verb the deslop-lsp server renders the report for. */
const val DESLOP_RENDER_HTML_REPORT_COMMAND: String = "deslop.lsp.renderHtmlReport"

private const val NO_REPORT_MESSAGE =
    "Deslop isn't analysing yet — open a supported file to start the server, then try again."
private const val RENDER_FAILED_MESSAGE = "Deslop failed to render the HTML report."

/**
 * Toolbar/menu action that renders the live report through the LSP and shows it
 * in an embedded-browser tab. The transport (native LSP vs LSP4IJ) differs per
 * plugin variant, so [executeRenderCommand] is abstract while the threading,
 * result coercion, error handling, and display are shared here. Keeping the
 * shared half in one place mirrors the "do not duplicate rendering" rule.
 */
abstract class DeslopOpenHtmlReportAction : AnAction() {

    final override fun getActionUpdateThread(): ActionUpdateThread = ActionUpdateThread.BGT

    final override fun update(event: AnActionEvent) {
        event.presentation.isEnabledAndVisible = event.project != null
    }

    final override fun actionPerformed(event: AnActionEvent) {
        val project = event.project ?: return
        ApplicationManager.getApplication().executeOnPooledThread { renderAndShow(project) }
    }

    private fun renderAndShow(project: Project) {
        val outcome = runCatching { executeRenderCommand(project) }
        val failure = outcome.exceptionOrNull()
        if (failure != null) {
            DeslopStartupNotifier.show(project, failure.message ?: RENDER_FAILED_MESSAGE)
            return
        }
        val html = coerceHtml(outcome.getOrNull())
        if (html.isNullOrEmpty()) DeslopStartupNotifier.info(project, NO_REPORT_MESSAGE)
        else openDeslopHtmlReport(project, html)
    }

    /**
     * Sends [DESLOP_RENDER_HTML_REPORT_COMMAND] to the variant's LSP client and
     * returns the raw `executeCommand` result (or null when no server is
     * running). Runs on a background thread and may block on the response.
     */
    protected abstract fun executeRenderCommand(project: Project): Any?
}

/**
 * Coerces an `executeCommand` result into the HTML string. lsp4j surfaces an
 * `Object`-typed result as a raw [String] or a gson [JsonPrimitive]; anything
 * else means there is no report to show.
 */
private fun coerceHtml(result: Any?): String? = when (result) {
    is String -> result
    is JsonPrimitive -> if (result.isString) result.asString else null
    is JsonElement -> if (result.isJsonPrimitive) result.asString else null
    else -> null
}
