package com.nimblesite.deslop.jetbrains

import com.intellij.openapi.Disposable
import com.intellij.openapi.ui.SimpleToolWindowPanel
import com.intellij.openapi.util.Disposer
import com.intellij.ui.components.JBLabel
import com.intellij.ui.jcef.JBCefApp
import com.intellij.ui.jcef.JBCefBrowser
import javax.swing.JComponent
import javax.swing.SwingConstants

/** Shown in place of the report when the IDE runtime has no embedded browser. */
internal const val JCEF_UNAVAILABLE: String =
    "Deslop cannot show the report here: the embedded browser (JCEF) is unavailable in this IDE runtime."

/** Shown before the first report is rendered. */
private const val PLACEHOLDER_HTML =
    "<html><body style='font-family:sans-serif;padding:1rem;color:#888'>" +
        "<p>No Deslop report yet. Open a supported file " +
        "(<code>.cs</code> <code>.rs</code> <code>.py</code> <code>.dart</code>) to start " +
        "analysis, then click <b>Refresh</b>.</p></body></html>"

/**
 * The single Deslop report surface: the engine-rendered HTML in an embedded JCEF
 * browser. The tool window hosts one of these, and any future report surface reuses
 * it, so report rendering is never duplicated (the "do not duplicate the rendering"
 * UI rule). Renderer output is self-contained (inline CSS, no scripts), so [load]
 * is a one-shot replace.
 */
internal class DeslopReportPanel(parentDisposable: Disposable) :
    SimpleToolWindowPanel(true, true), Disposable {

    private val browser: JBCefBrowser? = if (JBCefApp.isSupported()) JBCefBrowser() else null

    init {
        Disposer.register(parentDisposable, this)
        browser?.let { Disposer.register(this, it) }
        setContent(reportComponent())
    }

    private fun reportComponent(): JComponent {
        val available = browser ?: return JBLabel(JCEF_UNAVAILABLE, SwingConstants.CENTER)
        available.loadHTML(PLACEHOLDER_HTML)
        return available.component
    }

    /** Replaces the displayed report with [html]; a no-op when JCEF is unavailable. */
    fun load(html: String) {
        browser?.loadHTML(html)
    }

    override fun dispose() = Unit
}
