package com.nimblesite.deslop.jetbrains

import com.intellij.ide.BrowserUtil
import com.intellij.openapi.diagnostic.Logger
import com.intellij.openapi.util.io.FileUtil
import com.intellij.ui.components.JBLabel
import java.nio.file.Path
import javax.swing.BoxLayout
import javax.swing.JButton
import javax.swing.JComponent
import javax.swing.JPanel
import javax.swing.SwingConstants

/** Label of the no-JCEF fallback's "open externally" control. */
internal const val OPEN_REPORT_EXTERNALLY_LABEL: String = "Open the Deslop report in your browser"

/**
 * The report surface for IDE runtimes without an embedded browser (JCEF) — notably
 * some Android Studio builds, where `JBCefApp.isSupported()` is false. The engine
 * report is a self-contained HTML document (inline CSS, no scripts), so instead of a
 * dead end the fallback retains the latest report and opens it in the system browser
 * on demand. Reactive refreshes only replace the retained HTML; they never launch a
 * browser tab, so a live edit cannot flood the desktop — only an explicit click opens
 * the report. Kept separate from [DeslopReportPanel] so the panel's job stays "pick a
 * surface and forward the HTML" and this file owns the external-open concern.
 */
internal class DeslopExternalReportFallback {
    private var latestHtml: String? = null
    private val openButton = JButton(OPEN_REPORT_EXTERNALLY_LABEL).apply {
        isEnabled = false
        addActionListener { openLatestReport() }
    }

    /** The Swing component [DeslopReportPanel] installs when JCEF is unavailable. */
    val component: JComponent = buildComponent()

    /** Retains [html] as the report to open and enables the open control. */
    fun retain(html: String) {
        latestHtml = html
        openButton.isEnabled = true
    }

    private fun buildComponent(): JComponent {
        val panel = JPanel()
        panel.layout = BoxLayout(panel, BoxLayout.Y_AXIS)
        panel.add(JBLabel(JCEF_UNAVAILABLE, SwingConstants.CENTER))
        panel.add(openButton)
        return panel
    }

    private fun openLatestReport() {
        val html = latestHtml ?: return
        runCatching { BrowserUtil.browse(writeBrowsableReport(html).toFile()) }
            .onFailure { LOG.warn("failed to open the Deslop report in the external browser", it) }
    }

    private companion object {
        val LOG = Logger.getInstance(DeslopExternalReportFallback::class.java)
    }
}

/**
 * Writes a self-contained Deslop report [html] to a temporary, auto-deleted HTML file
 * the system browser can open, returning its path. Separated from the browser launch
 * so the no-JCEF fallback's materialization is provable without spawning a browser.
 */
internal fun writeBrowsableReport(html: String): Path {
    val file = FileUtil.createTempFile("deslop-report", ".html", true)
    file.writeText(html, Charsets.UTF_8)
    return file.toPath()
}
