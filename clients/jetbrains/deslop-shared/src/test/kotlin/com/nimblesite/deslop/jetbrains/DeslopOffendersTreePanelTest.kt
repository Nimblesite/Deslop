package com.nimblesite.deslop.jetbrains

import com.intellij.openapi.application.ApplicationManager
import com.intellij.testFramework.TestApplicationManager
import java.awt.Container
import java.util.concurrent.atomic.AtomicReference
import javax.swing.JTree
import javax.swing.tree.DefaultMutableTreeNode
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlin.test.fail

/**
 * Launch coverage for [DeslopOffendersTreePanel] — the native Swing worst-offenders
 * tree the JetBrains tool window hosts where JCEF is unavailable — against a real
 * (headless) IDE Application. It proves the panel builds its toolbar + tree, that
 * [DeslopOffendersTreePanel.show] drives the pure grouping seam into a real
 * [DefaultTreeModel], and that the default clone-type-first grouping surfaces
 * worst-first groups under a hidden synthetic root. Application-only (no project),
 * because navigation resolution needs no project when [DeslopOffendersTreePanel] is
 * constructed with `null` — the grouping structure is what this asserts.
 */
internal class DeslopOffendersTreePanelTest {
    @Test
    fun panelShowsCloneTypeFirstGroupingUnderHiddenRoot() {
        TestApplicationManager.getInstance()
        runOnEdt {
            val panel = DeslopOffendersTreePanel(null)

            panel.show(OFFENDERS_FIXTURE_JSON)

            val tree = findTree(panel.component) ?: fail("the panel must contain a tree")
            assertFalse(tree.isRootVisible, "the synthetic root is hidden; the groups are the visible top level")
            assertEquals(
                listOf("Identical (2)", "Nearly identical (1)", "Structural only (1)"),
                topLevelLabels(tree),
                "the default clone-type-first grouping surfaces worst-first groups through Swing",
            )
            assertEquals(
                "Dart (1)",
                firstGrandchildLabel(tree),
                "with all axes enabled, language nests under the worst clone-type group",
            )
        }
    }

    /**
     * A malformed report payload (client/LSP version skew, a truncated response) must
     * not crash the EDT — not on arrival, and not later when a toolbar toggle rebuilds
     * the tree. The tree keeps its last-good content instead of clearing or throwing.
     * Regression for the guard that only covered the first render: the axis actions
     * re-grouped the stored payload with no protection, so a poisoned payload surfaced
     * as an uncaught IDE error the moment the user toggled an axis.
     */
    @Test
    fun malformedPayloadKeepsLastGoodTreeAcrossAxisToggle() {
        TestApplicationManager.getInstance()
        runOnEdt {
            val panel = DeslopOffendersTreePanel(null)
            panel.show(OFFENDERS_FIXTURE_JSON)
            val tree = findTree(panel.component) ?: fail("the panel must contain a tree")
            val lastGood = topLevelLabels(tree)
            assertTrue(lastGood.isNotEmpty(), "the valid report must populate the tree")

            panel.show("{ not valid json")

            assertEquals(lastGood, topLevelLabels(tree), "a malformed payload leaves the last-good tree intact")

            panel.setAxisEnabled(Axis.LANGUAGE, false)

            assertTrue(
                topLevelLabels(tree).isNotEmpty(),
                "toggling an axis after a bad payload rebuilds from last-good data, never crashes the EDT",
            )
        }
    }
}

/** Labels of the model root's direct children — the visible top-level group rows. */
private fun topLevelLabels(tree: JTree): List<String> {
    val root = tree.model.root as DefaultMutableTreeNode
    return (0 until root.childCount).map { labelOf(root.getChildAt(it)) }
}

/** Label of the first child of the first top-level group — proves the second axis nests. */
private fun firstGrandchildLabel(tree: JTree): String {
    val firstGroup = (tree.model.root as DefaultMutableTreeNode).getChildAt(0) as DefaultMutableTreeNode
    return labelOf(firstGroup.getChildAt(0))
}

/** The [OffenderNode] label carried by a tree node's user object. */
private fun labelOf(node: Any): String = ((node as DefaultMutableTreeNode).userObject as OffenderNode).label

/** Depth-first search for the first [JTree] in [container]'s component subtree. */
private fun findTree(container: Container): JTree? {
    for (child in container.components) {
        if (child is JTree) return child
        if (child is Container) findTree(child)?.let { return it }
    }
    return null
}

/** Runs [block] on the EDT (Swing requires it) and rethrows its failure on the test thread. */
private fun runOnEdt(block: () -> Unit) {
    val failure = AtomicReference<Throwable?>()
    ApplicationManager.getApplication().invokeAndWait {
        runCatching(block).exceptionOrNull()?.let(failure::set)
    }
    failure.get()?.let { throw it }
}
