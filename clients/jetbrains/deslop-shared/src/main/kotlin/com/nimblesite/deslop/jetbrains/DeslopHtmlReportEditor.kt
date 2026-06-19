package com.nimblesite.deslop.jetbrains

import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.fileEditor.FileEditor
import com.intellij.openapi.fileEditor.FileEditorManager
import com.intellij.openapi.fileEditor.FileEditorPolicy
import com.intellij.openapi.fileEditor.FileEditorProvider
import com.intellij.openapi.fileEditor.FileEditorState
import com.intellij.openapi.fileEditor.FileEditorStateLevel
import com.intellij.openapi.project.DumbAware
import com.intellij.openapi.project.Project
import com.intellij.openapi.util.Disposer
import com.intellij.openapi.util.Key
import com.intellij.openapi.util.UserDataHolderBase
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.testFramework.LightVirtualFile
import com.intellij.ui.jcef.JBCefApp
import com.intellij.ui.jcef.JBCefBrowser
import java.beans.PropertyChangeListener
import java.beans.PropertyChangeSupport
import javax.swing.JComponent

/** Marks an in-memory file as the Deslop HTML report so our editor claims it. */
val DESLOP_HTML_REPORT_KEY: Key<Boolean> = Key.create("deslop.html.report")

private const val REPORT_FILE_NAME = "Deslop Report.html"
private const val JCEF_UNAVAILABLE =
    "Deslop cannot show the HTML report: the embedded browser (JCEF) is unavailable in this IDE runtime."

/**
 * Shows the engine-rendered HTML report in an embedded-browser editor tab.
 * Renderer output is self-contained (inline CSS, no scripts), so this is a
 * one-shot load. Re-running replaces the previous report tab in place.
 * Must run on the EDT — [FileEditorManager.openFile] requires it — so the
 * body is dispatched through `invokeLater`.
 */
fun openDeslopHtmlReport(project: Project, html: String) {
    ApplicationManager.getApplication().invokeLater {
        // The render ran on a pooled thread; the project may have closed before
        // this reaches the EDT, and FileEditorManager throws on a disposed one.
        if (project.isDisposed) return@invokeLater
        if (JBCefApp.isSupported()) replaceReportTab(project, html)
        else DeslopStartupNotifier.show(project, JCEF_UNAVAILABLE)
    }
}

/** Closes any open report tab, then opens a fresh one for the latest snapshot. */
private fun replaceReportTab(project: Project, html: String) {
    val manager = FileEditorManager.getInstance(project)
    manager.openFiles
        .filter { it.getUserData(DESLOP_HTML_REPORT_KEY) == true }
        .forEach(manager::closeFile)
    val file = LightVirtualFile(REPORT_FILE_NAME, html)
    file.putUserData(DESLOP_HTML_REPORT_KEY, true)
    manager.openFile(file, true)
}

/** Routes Deslop report files to the JCEF editor and hides the text editor. */
internal class DeslopHtmlReportEditorProvider : FileEditorProvider, DumbAware {
    override fun accept(project: Project, file: VirtualFile): Boolean =
        file.getUserData(DESLOP_HTML_REPORT_KEY) == true

    override fun createEditor(project: Project, file: VirtualFile): FileEditor =
        DeslopHtmlReportEditor(file, String(file.contentsToByteArray(), file.charset))

    override fun getEditorTypeId(): String = "deslop-html-report"

    override fun getPolicy(): FileEditorPolicy = FileEditorPolicy.HIDE_DEFAULT_EDITOR
}

/** A read-only editor hosting the report HTML in an embedded JCEF browser. */
internal class DeslopHtmlReportEditor(
    private val file: VirtualFile,
    html: String,
) : UserDataHolderBase(), FileEditor {
    private val browser = JBCefBrowser()
    private val propertyChange = PropertyChangeSupport(this)

    init {
        Disposer.register(this, browser)
        browser.loadHTML(html)
    }

    override fun getComponent(): JComponent = browser.component

    override fun getPreferredFocusedComponent(): JComponent = browser.component

    override fun getName(): String = "Deslop Report"

    override fun getFile(): VirtualFile = file

    override fun getState(level: FileEditorStateLevel): FileEditorState = FileEditorState.INSTANCE

    override fun setState(state: FileEditorState) = Unit

    override fun isModified(): Boolean = false

    override fun isValid(): Boolean = true

    override fun addPropertyChangeListener(listener: PropertyChangeListener) =
        propertyChange.addPropertyChangeListener(listener)

    override fun removePropertyChangeListener(listener: PropertyChangeListener) =
        propertyChange.removePropertyChangeListener(listener)

    override fun dispose() = Unit
}
