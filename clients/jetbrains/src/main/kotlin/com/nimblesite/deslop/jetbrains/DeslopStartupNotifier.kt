package com.nimblesite.deslop.jetbrains

import com.intellij.notification.NotificationGroupManager
import com.intellij.notification.NotificationType
import com.intellij.openapi.project.Project

internal object DeslopStartupNotifier {
    private const val GROUP_ID = "Deslop"

    fun show(project: Project, message: String) {
        val text = message.ifBlank { "Deslop cannot start because deslop-lsp failed verification." }
        NotificationGroupManager.getInstance()
            .getNotificationGroup(GROUP_ID)
            .createNotification(text, NotificationType.ERROR)
            .notify(project)
    }
}
