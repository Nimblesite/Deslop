package com.nimblesite.deslop.jetbrains

import com.google.gson.JsonElement
import com.google.gson.JsonPrimitive
import com.intellij.openapi.project.Project
import com.redhat.devtools.lsp4ij.LanguageServerManager
import org.eclipse.lsp4j.ExecuteCommandParams
import java.util.concurrent.TimeUnit

/** `workspace/executeCommand` verb the deslop-lsp server renders the HTML report for. */
private const val RENDER_HTML_REPORT_COMMAND = "deslop.lsp.renderHtmlReport"

/** `workspace/executeCommand` verb returning the structured report as a JSON string. */
private const val REPORT_JSON_COMMAND = "deslop.lsp.reportJson"

/**
 * LSP4IJ implementation of [DeslopReportRenderer]. Sends report commands through
 * the LSP4IJ client; `getLanguageServer` starts the server if needed, so a report
 * is produced even before a supported file is open. Returns null when no server is
 * available. Registered as a project service in plugin.xml.
 */
internal class DeslopLsp4ijReportRenderer(private val project: Project) : DeslopReportRenderer {
    override fun render(): String? = executeStringCommand(RENDER_HTML_REPORT_COMMAND)

    override fun reportJson(): String? = executeStringCommand(REPORT_JSON_COMMAND)

    /**
     * Runs a report `executeCommand` verb that returns a string result (HTML or
     * JSON), starting the server if needed. Null when no server is available.
     */
    private fun executeStringCommand(command: String): String? {
        val item = LanguageServerManager.getInstance(project)
            .getLanguageServer(DESLOP_SERVER_ID)
            .get(REQUEST_TIMEOUT_SECONDS, TimeUnit.SECONDS)
            ?: return null
        val result = item.server.workspaceService
            .executeCommand(ExecuteCommandParams(command, emptyList<Any>()))
            .get(REQUEST_TIMEOUT_SECONDS, TimeUnit.SECONDS)
        return coerceString(result)
    }

    private companion object {
        const val DESLOP_SERVER_ID = "deslop"
        const val REQUEST_TIMEOUT_SECONDS = 15L
    }
}

/**
 * Coerces an `executeCommand` result into its string payload. lsp4j surfaces an
 * `Object`-typed result as a raw [String] or a gson [JsonPrimitive]; anything else
 * means there is no report to show.
 */
private fun coerceString(result: Any?): String? = when (result) {
    is String -> result
    is JsonPrimitive -> if (result.isString) result.asString else null
    is JsonElement -> if (result.isJsonPrimitive) result.asString else null
    else -> null
}
