package com.nimblesite.deslop.jetbrains

import com.intellij.openapi.actionSystem.ActionManager
import com.intellij.openapi.actionSystem.ActionPlaces
import com.intellij.openapi.actionSystem.ActionToolbar
import com.intellij.openapi.actionSystem.DefaultActionGroup
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.components.service
import com.intellij.openapi.project.DumbAware
import com.intellij.openapi.project.Project
import com.intellij.openapi.wm.ToolWindow
import com.intellij.openapi.wm.ToolWindowFactory
import com.intellij.openapi.wm.ToolWindowManager
import com.intellij.ui.jcef.JBCefApp
import javax.swing.JComponent

/** Tool window id; matches the `<toolWindow id="Deslop">` registration in plugin.xml. */
internal const val DESLOP_TOOL_WINDOW_ID: String = "Deslop"

/** Action id shared by the Tools menu and the tool window's Refresh toolbar button. */
internal const val DESLOP_RENDER_REPORT_ACTION_ID: String = "Deslop.OpenHtmlReport"

/**
 * Registers the **Deslop** tool window: a [DeslopReportPanel] showing the live
 * worst-offenders report with a toolbar Refresh action. On first open it renders
 * the report once (best effort), so the panel is populated without an extra click.
 */
internal class DeslopReportToolWindowFactory : ToolWindowFactory, DumbAware {
    override fun createToolWindowContent(project: Project, toolWindow: ToolWindow) {
        installDeslopReportPanel(project, toolWindow)
        refreshDeslopReport(project, activate = false)
    }
}

/**
 * Builds the [DeslopReportPanel] with its Refresh toolbar and adds it to
 * [toolWindow] as the single, pinned (non-closeable) content; returns the panel.
 * Separated from the best-effort initial render so the deterministic panel wiring
 * is provable in a platform test without spawning the off-EDT render.
 */
internal fun installDeslopReportPanel(project: Project, toolWindow: ToolWindow): DeslopReportPanel {
    val panel = DeslopReportPanel(project, toolWindow.disposable)
    panel.toolbar = buildReportToolbar(panel).component
    val content = toolWindow.contentManager.factory.createContent(panel, "", false)
    content.isCloseable = false
    toolWindow.contentManager.addContent(content)
    return panel
}

private fun buildReportToolbar(target: JComponent): ActionToolbar {
    val group = DefaultActionGroup()
    ActionManager.getInstance().getAction(DESLOP_RENDER_REPORT_ACTION_ID)?.let(group::add)
    return ActionManager.getInstance()
        .createActionToolbar(ActionPlaces.TOOLWINDOW_CONTENT, group, true)
        .also { it.targetComponent = target }
}

/**
 * Fetches the report payload the tool-window surface renders: structured JSON for the
 * native worst-offenders tree (IDEs without an embedded browser) or the self-contained
 * HTML document for the JCEF browser. Keying both the fetch and the [DeslopReportPanel]
 * surface off `JBCefApp.isSupported()` keeps them in lock-step. May block on the LSP,
 * so call it off the EDT. Null when no server is running or it produced no report.
 */
internal fun fetchDeslopReportPayload(renderer: DeslopReportRenderer): String? =
    if (JBCefApp.isSupported()) renderer.render() else renderer.reportJson()

/**
 * Refreshes the report off the EDT through the [DeslopReportRenderer] service and shows
 * it in the tool window, foregrounding it when [activate]. The single place the render
 * service is consumed for a best-effort refresh, so first-open and the reactive
 * `deslop/reportChanged` refresh never duplicate the pooled-fetch dance. Fetch failures
 * are swallowed here (best effort); the user-invoked action reports them.
 */
internal fun refreshDeslopReport(project: Project, activate: Boolean) {
    ApplicationManager.getApplication().executeOnPooledThread {
        val service = project.service<DeslopReportRenderer>()
        val payload = runCatching { fetchDeslopReportPayload(service) }.getOrNull()
        if (!payload.isNullOrEmpty()) showDeslopReport(project, payload, activate)
    }
}

/**
 * Shows [payload] in the Deslop tool window's panel on the EDT, bringing the tool
 * window to the foreground when [activate] (the explicit action and first-open render,
 * where surfacing the panel is the point) and refreshing in place otherwise (the
 * reactive path, so a live edit never yanks the tool window forward).
 */
internal fun showDeslopReport(project: Project, payload: String, activate: Boolean) {
    withDeslopToolWindow(project) { toolWindow ->
        if (activate) toolWindow.activate { reportPanel(toolWindow)?.display(payload) }
        else reportPanel(toolWindow)?.display(payload)
    }
}

/**
 * Reactive entry point for the server's `deslop/reportChanged` notification:
 * re-render off the EDT and refresh the open tool window in place. This is the UI
 * leg of the live loop (watcher → scheduler → broadcast → UI) for every JetBrains
 * IDE family, mirroring the VS Code client's live refresh. Public because the
 * LSP4IJ language client in the surface module drives it from the notification.
 */
fun refreshDeslopReportInPlace(project: Project) = refreshDeslopReport(project, activate = false)

/**
 * Runs [action] against the Deslop tool window on the EDT. Tool window access
 * requires the EDT, so the body runs through `invokeLater`; the project may close
 * before it lands, so a disposed project (or an unregistered tool window) is skipped.
 */
private fun withDeslopToolWindow(project: Project, action: (ToolWindow) -> Unit) {
    ApplicationManager.getApplication().invokeLater {
        if (project.isDisposed) return@invokeLater
        val toolWindow = ToolWindowManager.getInstance(project).getToolWindow(DESLOP_TOOL_WINDOW_ID)
            ?: return@invokeLater
        action(toolWindow)
    }
}

private fun reportPanel(toolWindow: ToolWindow): DeslopReportPanel? =
    toolWindow.contentManager.contents.firstNotNullOfOrNull { it.component as? DeslopReportPanel }
