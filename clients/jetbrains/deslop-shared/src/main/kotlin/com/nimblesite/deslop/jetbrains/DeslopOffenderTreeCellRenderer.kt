package com.nimblesite.deslop.jetbrains

import com.intellij.icons.AllIcons
import com.intellij.ui.ColoredTreeCellRenderer
import com.intellij.ui.SimpleTextAttributes
import javax.swing.JTree
import javax.swing.tree.DefaultMutableTreeNode

/**
 * Renders the worst-offenders tree: bold group headings with a grey count, plain
 * cluster leaves, and grey occurrence rows, each with a role icon. Reuses the
 * [OffenderNode] labels so the human-readable text is defined once in the model.
 */
internal class DeslopOffenderTreeCellRenderer : ColoredTreeCellRenderer() {
    override fun customizeCellRenderer(
        tree: JTree,
        value: Any?,
        selected: Boolean,
        expanded: Boolean,
        leaf: Boolean,
        row: Int,
        hasFocus: Boolean,
    ) {
        render((value as? DefaultMutableTreeNode)?.userObject)
    }

    private fun render(node: Any?) = when (node) {
        is GroupNode -> renderGroup(node)
        is ClusterNode -> renderCluster(node)
        is OccurrenceNode -> renderOccurrence(node)
        else -> Unit
    }

    private fun renderGroup(node: GroupNode) {
        icon = AllIcons.Nodes.Folder
        append(node.value, SimpleTextAttributes.REGULAR_BOLD_ATTRIBUTES)
        append(" (${node.clusterCount})", SimpleTextAttributes.GRAYED_ATTRIBUTES)
    }

    private fun renderCluster(node: ClusterNode) {
        icon = AllIcons.Actions.Copy
        append(node.label, SimpleTextAttributes.REGULAR_ATTRIBUTES)
    }

    private fun renderOccurrence(node: OccurrenceNode) {
        icon = AllIcons.General.Locate
        append(node.label, SimpleTextAttributes.GRAYED_ATTRIBUTES)
    }
}
