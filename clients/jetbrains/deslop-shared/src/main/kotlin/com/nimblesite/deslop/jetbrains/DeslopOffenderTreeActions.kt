package com.nimblesite.deslop.jetbrains

import com.intellij.openapi.actionSystem.ActionUpdateThread
import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.actionSystem.ToggleAction
import javax.swing.Icon

/**
 * The grouping state the toolbar actions drive, kept behind an interface so the
 * actions are decoupled from the Swing panel (and testable in isolation).
 */
internal interface AxisController {
    /** True when [axis] currently contributes a grouping level. */
    fun isAxisEnabled(axis: Axis): Boolean

    /** Turns [axis]'s grouping level on or off and rebuilds the tree. */
    fun setAxisEnabled(axis: Axis, enabled: Boolean)

    /** The axis the reorder actions act on (the selected group's axis), or null. */
    fun activeAxis(): Axis?

    /** Moves the [activeAxis] one step [delta] in the precedence order and rebuilds. */
    fun moveActiveAxis(delta: Int)
}

/**
 * Toolbar toggle for one grouping [axis]: selecting it adds the axis as a grouping
 * level, deselecting removes it. All three axes off yields a flat worst-first list.
 */
internal class AxisToggleAction(
    private val axis: Axis,
    private val controller: AxisController,
) : ToggleAction(axis.displayName) {
    override fun isSelected(event: AnActionEvent): Boolean = controller.isAxisEnabled(axis)

    override fun setSelected(event: AnActionEvent, state: Boolean) = controller.setAxisEnabled(axis, state)

    override fun getActionUpdateThread(): ActionUpdateThread = ActionUpdateThread.EDT
}

/**
 * Toolbar action that moves the selected group's axis one step [delta] in the order
 * of precedence (negative = outward/earlier, positive = inward/later), changing which
 * axis is outermost. Disabled until a group node is selected.
 */
internal class MoveAxisAction(
    private val delta: Int,
    text: String,
    icon: Icon,
    private val controller: AxisController,
) : AnAction(text, null, icon) {
    override fun actionPerformed(event: AnActionEvent) = controller.moveActiveAxis(delta)

    override fun update(event: AnActionEvent) {
        event.presentation.isEnabled = controller.activeAxis() != null
    }

    override fun getActionUpdateThread(): ActionUpdateThread = ActionUpdateThread.EDT
}
