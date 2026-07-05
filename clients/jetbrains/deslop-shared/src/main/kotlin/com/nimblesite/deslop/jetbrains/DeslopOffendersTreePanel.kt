package com.nimblesite.deslop.jetbrains

import com.intellij.icons.AllIcons
import com.intellij.openapi.actionSystem.ActionManager
import com.intellij.openapi.actionSystem.ActionPlaces
import com.intellij.openapi.actionSystem.ActionToolbar
import com.intellij.openapi.actionSystem.DefaultActionGroup
import com.intellij.openapi.diagnostic.Logger
import com.intellij.openapi.fileEditor.FileEditorManager
import com.intellij.openapi.fileEditor.OpenFileDescriptor
import com.intellij.openapi.project.Project
import com.intellij.openapi.ui.SimpleToolWindowPanel
import com.intellij.openapi.vfs.LocalFileSystem
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.ui.components.JBScrollPane
import com.intellij.ui.treeStructure.Tree
import java.awt.event.KeyAdapter
import java.awt.event.KeyEvent
import java.awt.event.MouseAdapter
import java.awt.event.MouseEvent
import java.nio.file.Path
import javax.swing.JComponent
import javax.swing.tree.DefaultMutableTreeNode
import javax.swing.tree.DefaultTreeModel

/**
 * Native Swing "worst offenders" grouping tree for the JetBrains tool window — no
 * JCEF, no HTML, so it renders where Android Studio's runtime has no embedded browser.
 * A configurable multi-axis tree (clone type → language → folder by default) built by
 * the pure [DeslopOffenderGrouping] seam and mirrored into a [Tree]. The toolbar
 * toggles each axis and reorders their precedence; double-click / Enter opens the
 * source. Implements [AxisController] so the toolbar actions drive it directly.
 */
internal class DeslopOffendersTreePanel(private val project: Project?) : AxisController {
    private val axisOrder: MutableList<Axis> = Axis.entries.toMutableList()
    private val enabledAxes: MutableSet<Axis> = Axis.entries.toMutableSet()
    private var selectedAxis: Axis? = null

    // Last successfully-parsed clusters. Re-grouping (axis toggle/reorder) works off
    // this cache, so it never re-parses; a malformed [show] leaves it untouched.
    private var clusters: List<OffenderCluster> = emptyList()

    private val rootNode = DefaultMutableTreeNode()
    private val treeModel = DefaultTreeModel(rootNode)
    private val tree = Tree(treeModel)
    private val toolWindowPanel = SimpleToolWindowPanel(true, true)

    /** The whole panel: the grouping toolbar above the scrollable tree. */
    val component: JComponent get() = toolWindowPanel

    init {
        tree.isRootVisible = false
        tree.showsRootHandles = true
        tree.cellRenderer = DeslopOffenderTreeCellRenderer()
        installInteractions()
        toolWindowPanel.toolbar = buildToolbar().component
        toolWindowPanel.setContent(JBScrollPane(tree))
        rebuild()
    }

    /**
     * Parses [reportJson] and rebuilds the grouped tree with the current axis config.
     * A malformed payload is logged and ignored — the last-good tree stays on screen —
     * rather than clearing the view or throwing onto the EDT.
     */
    fun show(reportJson: String) {
        val parsed = runCatching { DeslopOffenderGrouping.parse(reportJson) }
            .onFailure { LOG.warn("ignored a malformed Deslop report payload", it) }
            .getOrNull() ?: return
        clusters = parsed
        rebuild()
    }

    override fun isAxisEnabled(axis: Axis): Boolean = axis in enabledAxes

    override fun setAxisEnabled(axis: Axis, enabled: Boolean) {
        if (enabled) enabledAxes.add(axis) else enabledAxes.remove(axis)
        rebuild()
    }

    override fun activeAxis(): Axis? = selectedAxis

    override fun moveActiveAxis(delta: Int) {
        val axis = selectedAxis ?: return
        val index = axisOrder.indexOf(axis)
        val target = index + delta
        if (target !in axisOrder.indices) return
        axisOrder.removeAt(index)
        axisOrder.add(target, axis)
        rebuild()
    }

    private fun rebuild() {
        val effectiveAxes = axisOrder.filter(enabledAxes::contains)
        val nodes = DeslopOffenderGrouping.group(clusters, effectiveAxes)
        rootNode.removeAllChildren()
        nodes.forEach { rootNode.add(toTreeNode(it)) }
        treeModel.reload()
    }

    private fun toTreeNode(node: OffenderNode): DefaultMutableTreeNode {
        val treeNode = DefaultMutableTreeNode(node)
        node.children.forEach { treeNode.add(toTreeNode(it)) }
        return treeNode
    }

    private fun buildToolbar(): ActionToolbar {
        val group = DefaultActionGroup()
        Axis.entries.forEach { group.add(AxisToggleAction(it, this)) }
        group.addSeparator()
        group.add(MoveAxisAction(-1, "Group Earlier", AllIcons.Actions.MoveUp, this))
        group.add(MoveAxisAction(1, "Group Later", AllIcons.Actions.MoveDown, this))
        return ActionManager.getInstance()
            .createActionToolbar(ActionPlaces.TOOLWINDOW_CONTENT, group, true)
            .also { it.targetComponent = tree }
    }

    private fun installInteractions() {
        tree.addTreeSelectionListener { selectedAxis = (selectedUserObject() as? GroupNode)?.axis }
        tree.addMouseListener(object : MouseAdapter() {
            override fun mouseClicked(event: MouseEvent) {
                if (event.clickCount == 2) openSelected()
            }
        })
        tree.addKeyListener(object : KeyAdapter() {
            override fun keyPressed(event: KeyEvent) {
                if (event.keyCode == KeyEvent.VK_ENTER) openSelected()
            }
        })
    }

    private fun selectedUserObject(): Any? =
        (tree.lastSelectedPathComponent as? DefaultMutableTreeNode)?.userObject

    private fun openSelected() = when (val node = selectedUserObject()) {
        is OccurrenceNode -> openOccurrence(node.occurrence)
        is ClusterNode -> openOccurrence(node.cluster.firstOccurrence)
        else -> Unit
    }

    private fun openOccurrence(occurrence: OffenderOccurrence) {
        val activeProject = project ?: return
        val file = resolveFile(activeProject, occurrence.path) ?: return
        val line = (occurrence.startLine - 1).coerceAtLeast(0)
        FileEditorManager.getInstance(activeProject).openTextEditor(OpenFileDescriptor(activeProject, file, line, 0), true)
    }

    private fun resolveFile(activeProject: Project, path: String): VirtualFile? {
        val fileSystem = LocalFileSystem.getInstance()
        val candidate = Path.of(path)
        if (candidate.isAbsolute) return fileSystem.findFileByNioFile(candidate)
        val base = activeProject.basePath ?: return null
        return fileSystem.findFileByNioFile(Path.of(base).resolve(path))
    }

    private companion object {
        val LOG = Logger.getInstance(DeslopOffendersTreePanel::class.java)
    }
}
