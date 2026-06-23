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
        val panel = DeslopReportPanel(toolWindow.disposable)
        panel.toolbar = buildToolbar(panel).component
        val content = toolWindow.contentManager.factory.createContent(panel, "", false)
        content.isCloseable = false
        toolWindow.contentManager.addContent(content)
        renderInitialReport(project)
    }

    private fun buildToolbar(target: JComponent): ActionToolbar {
        val group = DefaultActionGroup()
        ActionManager.getInstance().getAction(DESLOP_RENDER_REPORT_ACTION_ID)?.let(group::add)
        return ActionManager.getInstance()
            .createActionToolbar(ActionPlaces.TOOLWINDOW_CONTENT, group, true)
            .also { it.targetComponent = target }
    }

    private fun renderInitialReport(project: Project) {
        ApplicationManager.getApplication().executeOnPooledThread {
            val html = runCatching { project.service<DeslopReportRenderer>().render() }.getOrNull()
            if (!html.isNullOrEmpty()) openDeslopHtmlReport(project, html)
        }
    }
}

/**
 * Shows [html] in the Deslop tool window and activates it. Tool window access
 * requires the EDT, so the body runs through `invokeLater`; the project may close
 * before it lands, so a disposed project is skipped.
 */
internal fun openDeslopHtmlReport(project: Project, html: String) {
    ApplicationManager.getApplication().invokeLater {
        if (project.isDisposed) return@invokeLater
        val toolWindow = ToolWindowManager.getInstance(project).getToolWindow(DESLOP_TOOL_WINDOW_ID)
            ?: return@invokeLater
        toolWindow.activate { reportPanel(toolWindow)?.load(html) }
    }
}

private fun reportPanel(toolWindow: ToolWindow): DeslopReportPanel? =
    toolWindow.contentManager.contents.firstNotNullOfOrNull { it.component as? DeslopReportPanel }
