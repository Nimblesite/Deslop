package com.nimblesite.deslop.jetbrains

import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.util.Disposer
import com.intellij.testFramework.TestApplicationManager
import com.intellij.ui.jcef.JBCefApp
import java.awt.Container
import java.util.concurrent.atomic.AtomicReference
import javax.swing.JLabel
import kotlin.test.Test
import kotlin.test.assertTrue

/** A populated report fragment — the exact shape `render_html` emits for a clone. */
private const val REPORT_HTML: String =
    "<!doctype html><html data-theme=\"dark\"><body class=\"report-shell\">" +
        "<article class=\"cluster-card\"><h3 class=\"cluster-card__title\">clones in 2 places</h3>" +
        "</article></body></html>"

/**
 * Launch coverage for [DeslopReportPanel] — the surface the **Deslop** tool window
 * hosts on the right-hand stripe — against a real (headless) IDE Application. It
 * proves the panel builds, accepts the engine's report HTML without error (the
 * render call every code path makes: first open, the explicit action, and the
 * reactive `deslop/reportChanged` refresh), and degrades to a readable message when
 * the IDE runtime has no embedded browser, instead of showing nothing.
 *
 * Application-only (no project) on purpose: a project fixture would run background
 * file indexing whose SVG parser is absent from this slim test classpath, which is
 * unrelated to the panel. The server→client reactive leg is covered by deslop-lsp's
 * notification tests plus the compile-time binding of `DeslopLanguageClient`.
 */
internal class DeslopReportPanelTest {
    @Test
    fun panelAcceptsReportHtmlAndDegradesWithoutBrowser() {
        TestApplicationManager.getInstance()
        runOnEdt {
            val disposable = Disposer.newDisposable("deslop-report-panel-test")
            try {
                val panel = DeslopReportPanel(disposable)

                panel.load(REPORT_HTML)

                if (!JBCefApp.isSupported()) {
                    assertTrue(
                        descendantTexts(panel).any { it.contains(JCEF_UNAVAILABLE) },
                        "without an embedded browser the panel must show the unavailable fallback",
                    )
                }
            } finally {
                Disposer.dispose(disposable)
            }
        }
    }
}

/** Runs [block] on the EDT (Swing/JCEF require it) and rethrows its failure on the test thread. */
private fun runOnEdt(block: () -> Unit) {
    val failure = AtomicReference<Throwable?>()
    ApplicationManager.getApplication().invokeAndWait {
        runCatching(block).exceptionOrNull()?.let(failure::set)
    }
    failure.get()?.let { throw it }
}

/** All [JLabel] texts in [root]'s component tree, so the fallback message is assertable. */
private fun descendantTexts(root: Container): List<String> = buildList {
    for (child in root.components) {
        if (child is JLabel) child.text?.let(::add)
        if (child is Container) addAll(descendantTexts(child))
    }
}
