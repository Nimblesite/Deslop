package com.nimblesite.deslop.jetbrains

import com.intellij.openapi.project.Project
import com.redhat.devtools.lsp4ij.client.LanguageClientImpl
import org.eclipse.lsp4j.jsonrpc.services.JsonNotification

/**
 * Method deslop-lsp pushes whenever a live analysis pass changes the visible
 * report (the file watcher and the cache-seed commit both broadcast it). Mirror of
 * the server-side `deslop/reportChanged` constant; LSP4IJ routes it here by name.
 */
private const val REPORT_CHANGED_NOTIFICATION: String = "deslop/reportChanged"

/**
 * The Deslop LSP4IJ language client. Subclasses LSP4IJ's [LanguageClientImpl] for
 * one reason: to handle deslop-lsp's custom `deslop/reportChanged` notification so
 * the **Deslop** tool window refreshes in place on every server-driven change. This
 * is the UI leg of the live loop (watcher → scheduler → broadcast → UI) — without
 * it the JetBrains panel would be render-once, unlike the live VS Code client. All
 * standard LSP client behaviour is inherited unchanged.
 */
internal class DeslopLanguageClient(project: Project) : LanguageClientImpl(project) {
    /**
     * Handles `deslop/reportChanged`. The payload (generation + change summary) is
     * only a signal to re-render; the fresh HTML is fetched through the render
     * service, so the parameter is intentionally ignored. [project] is the inherited
     * [LanguageClientImpl.getProject].
     */
    @JsonNotification(REPORT_CHANGED_NOTIFICATION)
    fun reportChanged(@Suppress("UNUSED_PARAMETER") params: Any?) {
        refreshDeslopReportInPlace(project)
    }
}
