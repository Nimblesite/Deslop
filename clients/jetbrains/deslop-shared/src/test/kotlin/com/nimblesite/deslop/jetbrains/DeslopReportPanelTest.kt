package com.nimblesite.deslop.jetbrains

import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.util.Disposer
import com.intellij.testFramework.TestApplicationManager
import com.intellij.ui.jcef.JBCefApp
import java.awt.Container
import java.util.concurrent.atomic.AtomicReference
import javax.swing.JTree
import kotlin.test.Test
import kotlin.test.assertTrue

/** A populated HTML report fragment — the shape `render_html` emits for a clone. */
private const val REPORT_HTML: String =
    "<!doctype html><html data-theme=\"dark\"><body class=\"report-shell\">" +
        "<article class=\"cluster-card\"><h3 class=\"cluster-card__title\">clones in 2 places</h3>" +
        "</article></body></html>"

/** A populated structured report — the shape `deslop.lsp.reportJson` emits for the native tree. */
private const val REPORT_JSON: String =
    "{\"clusters\":[{\"id\":\"a1\",\"weight\":1000.0,\"size\":2,\"bucket\":\"identical\"," +
        "\"occurrences\":[" +
        "{\"path\":\"lib/api/client.dart\",\"start_line\":10,\"end_line\":20,\"hidden\":false}," +
        "{\"path\":\"lib/api/server.dart\",\"start_line\":5,\"end_line\":15,\"hidden\":false}]}]}"

/**
 * Launch coverage for [DeslopReportPanel] — the surface the **Deslop** tool window
 * hosts on the right-hand stripe — against a real (headless) IDE Application. It
 * proves the panel builds and accepts the engine's report for whichever surface the
 * runtime supports (the render call every code path makes: first open, the explicit
 * action, and the reactive `deslop/reportChanged` refresh), and that where the IDE has
 * no embedded browser it degrades to the native worst-offenders tree rather than a
 * dead end.
 *
 * Application-only (no project) on purpose: a project fixture would run background
 * file indexing whose SVG parser is absent from this slim test classpath, which is
 * unrelated to the panel. The server→client reactive leg is covered by deslop-lsp's
 * notification tests plus the compile-time binding of `DeslopLanguageClient`; the
 * grouping tree itself is covered by [DeslopOffenderGroupingTest] and
 * [DeslopOffendersTreePanelTest].
 */
internal class DeslopReportPanelTest {
    @Test
    fun panelAcceptsSurfacePayloadWithoutError() {
        onPanel { panel -> panel.display(payloadForRuntime()) }
    }

    @Test
    fun panelHostsNativeTreeWhenBrowserUnavailable() {
        onPanel { panel ->
            panel.display(payloadForRuntime())
            if (!JBCefApp.isSupported()) {
                assertTrue(
                    containsTree(panel),
                    "without an embedded browser the panel must host the native worst-offenders tree, not a dead end",
                )
            }
        }
    }
}

/** Builds a [DeslopReportPanel] on the EDT, runs [block] against it, then disposes it. */
private fun onPanel(block: (DeslopReportPanel) -> Unit) {
    TestApplicationManager.getInstance()
    runOnEdt {
        val disposable = Disposer.newDisposable("deslop-report-panel-test")
        try {
            block(DeslopReportPanel(null, disposable))
        } finally {
            Disposer.dispose(disposable)
        }
    }
}

/** The payload the current runtime's panel surface expects: HTML for JCEF, JSON otherwise. */
private fun payloadForRuntime(): String = if (JBCefApp.isSupported()) REPORT_HTML else REPORT_JSON

/** Runs [block] on the EDT (Swing/JCEF require it) and rethrows its failure on the test thread. */
private fun runOnEdt(block: () -> Unit) {
    val failure = AtomicReference<Throwable?>()
    ApplicationManager.getApplication().invokeAndWait {
        runCatching(block).exceptionOrNull()?.let(failure::set)
    }
    failure.get()?.let { throw it }
}

/** True when [root]'s component tree contains a [JTree] — the native worst-offenders surface. */
private fun containsTree(root: Container): Boolean =
    root.components.any { it is JTree || (it is Container && containsTree(it)) }
