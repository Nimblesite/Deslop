package com.nimblesite.deslop.jetbrains

import com.intellij.openapi.Disposable
import com.intellij.openapi.project.Project
import com.intellij.openapi.ui.SimpleToolWindowPanel
import com.intellij.openapi.util.Disposer
import com.intellij.ui.jcef.JBCefApp
import com.intellij.ui.jcef.JBCefBrowser
import javax.swing.JComponent

/** Shown in the embedded browser before the first report is rendered. */
private const val PLACEHOLDER_HTML =
    "<html><body style='font-family:sans-serif;padding:1rem;color:#888'>" +
        "<p>No Deslop report yet. Open a supported file " +
        "(<code>.cs</code> <code>.rs</code> <code>.py</code> <code>.dart</code> " +
        "<code>.js</code> <code>.mjs</code> <code>.cjs</code> <code>.jsx</code> " +
        "<code>.ts</code> <code>.tsx</code>) to start " +
        "analysis, then click <b>Refresh</b>.</p></body></html>"

/**
 * The Deslop report surface the tool window hosts. On IDEs with an embedded browser
 * (JCEF) it shows the engine-rendered HTML report; where JCEF is unavailable — e.g.
 * some Android Studio builds, whose runtime ships no Chromium — it hosts the native
 * [DeslopOffendersTreePanel] worst-offenders tree instead, so the report is always
 * reachable. [display] is handed the surface-appropriate payload (HTML or structured
 * JSON) by the single tool-window fetch path, so report rendering is never duplicated
 * (the "do not duplicate the rendering" UI rule).
 */
internal class DeslopReportPanel(
    private val project: Project?,
    parentDisposable: Disposable,
) : SimpleToolWindowPanel(true, true), Disposable {

    private val browser: JBCefBrowser? = if (JBCefApp.isSupported()) JBCefBrowser() else null

    // Built only when there is no embedded browser (its sole access sites are guarded
    // by `browser == null`), so JCEF-capable IDEs never construct the native tree.
    private val tree: DeslopOffendersTreePanel by lazy { DeslopOffendersTreePanel(project) }

    init {
        Disposer.register(parentDisposable, this)
        browser?.let { Disposer.register(this, it) }
        setContent(reportComponent())
    }

    private fun reportComponent(): JComponent {
        val available = browser ?: return tree.component
        available.loadHTML(PLACEHOLDER_HTML)
        return available.component
    }

    /**
     * Shows a report payload: HTML loaded into the embedded browser, or the structured
     * report JSON grouped by the native tree when JCEF is unavailable. The tree's own
     * [DeslopOffendersTreePanel.show] guards a malformed payload, so this stays a plain
     * dispatch.
     */
    fun display(payload: String) {
        val available = browser
        if (available != null) available.loadHTML(payload) else tree.show(payload)
    }

    override fun dispose() = Unit
}
