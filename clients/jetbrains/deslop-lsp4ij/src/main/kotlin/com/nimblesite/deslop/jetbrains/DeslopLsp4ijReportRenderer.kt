package com.nimblesite.deslop.jetbrains

import com.google.gson.JsonElement
import com.google.gson.JsonPrimitive
import com.intellij.openapi.project.Project
import com.redhat.devtools.lsp4ij.LanguageServerManager
import org.eclipse.lsp4j.ExecuteCommandParams
import java.util.concurrent.TimeUnit

/** `workspace/executeCommand` verb the deslop-lsp server renders the report for. */
private const val RENDER_HTML_REPORT_COMMAND = "deslop.lsp.renderHtmlReport"

/**
 * LSP4IJ implementation of [DeslopReportRenderer]. Sends the render command through
 * the LSP4IJ client; `getLanguageServer` starts the server if needed, so the report
 * renders even before a supported file is open. Returns null when no server is
 * available. Registered as a project service in plugin.xml.
 */
internal class DeslopLsp4ijReportRenderer(private val project: Project) : DeslopReportRenderer {
    override fun render(): String? {
        val item = LanguageServerManager.getInstance(project)
            .getLanguageServer(DESLOP_SERVER_ID)
            .get(REQUEST_TIMEOUT_SECONDS, TimeUnit.SECONDS)
            ?: return null
        val result = item.server.workspaceService
            .executeCommand(ExecuteCommandParams(RENDER_HTML_REPORT_COMMAND, emptyList<Any>()))
            .get(REQUEST_TIMEOUT_SECONDS, TimeUnit.SECONDS)
        return coerceHtml(result)
    }

    private companion object {
        const val DESLOP_SERVER_ID = "deslop"
        const val REQUEST_TIMEOUT_SECONDS = 15L
    }
}

/**
 * Coerces an `executeCommand` result into the HTML string. lsp4j surfaces an
 * `Object`-typed result as a raw [String] or a gson [JsonPrimitive]; anything else
 * means there is no report to show.
 */
private fun coerceHtml(result: Any?): String? = when (result) {
    is String -> result
    is JsonPrimitive -> if (result.isString) result.asString else null
    is JsonElement -> if (result.isJsonPrimitive) result.asString else null
    else -> null
}
