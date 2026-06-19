package com.nimblesite.deslop.jetbrains

import com.intellij.notification.NotificationGroupManager
import com.intellij.notification.NotificationType
import com.intellij.openapi.project.Project

object DeslopStartupNotifier {
    private const val GROUP_ID = "Deslop"

    fun show(project: Project, message: String) {
        post(project, message.ifBlank { "Deslop cannot start because deslop-lsp failed verification." }, NotificationType.ERROR)
    }

    /** Posts a non-error balloon — used for benign states like "no report yet". */
    fun info(project: Project, message: String) {
        post(project, message, NotificationType.INFORMATION)
    }

    private fun post(project: Project, message: String, type: NotificationType) {
        // A notification can be requested from a background thread after the
        // project has been closed (e.g. an in-flight LSP render); the platform
        // services throw against a disposed project, so drop it silently.
        if (project.isDisposed) return
        NotificationGroupManager.getInstance()
            .getNotificationGroup(GROUP_ID)
            .createNotification(message, type)
            .notify(project)
    }
}
