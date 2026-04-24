package com.nimblesite.deslop.jetbrains

import com.intellij.openapi.vfs.VirtualFile

internal object DeslopSupportedFiles {
    private val extensions = setOf("cs", "rs", "py")

    fun includes(file: VirtualFile): Boolean {
        val extension = file.extension?.lowercase()
        return !file.isDirectory && extension in extensions
    }
}
