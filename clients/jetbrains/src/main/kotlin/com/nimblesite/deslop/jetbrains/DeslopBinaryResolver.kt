package com.nimblesite.deslop.jetbrains

import com.intellij.ide.plugins.PluginManagerCore
import com.intellij.openapi.extensions.PluginId
import com.intellij.openapi.util.SystemInfoRt
import java.io.File
import java.nio.file.Files
import java.nio.file.Path

internal object DeslopBinaryResolver {
    private const val BINARY_NAME = "deslop-lsp"
    private const val ENV_DIR = "DESLOP_BINARY_DIR"
    private const val PLUGIN_ID_VALUE = "nimblesite.deslop.jetbrains"

    fun command(): String {
        return envBinary()?.toString()
            ?: pathBinary()?.toString()
            ?: bundledBinary()?.toString()
            ?: binaryName()
    }

    private fun envBinary(): Path? {
        val dir = System.getenv(ENV_DIR)?.takeIf(String::isNotBlank) ?: return null
        return Path.of(dir).resolve(binaryName()).takeIf(::isUsable)
    }

    private fun pathBinary(): Path? {
        val path = System.getenv("PATH") ?: System.getenv("Path") ?: return null
        return path.split(File.pathSeparator).asSequence()
            .filter(String::isNotBlank)
            .map { Path.of(it).resolve(binaryName()) }
            .firstOrNull(::isUsable)
    }

    private fun bundledBinary(): Path? {
        val plugin = PluginManagerCore.getPlugin(PluginId.getId(PLUGIN_ID_VALUE)) ?: return null
        return plugin.pluginPath.resolve("bin")
            .resolve(platformDir())
            .resolve(binaryName())
            .takeIf(::isUsable)
    }

    private fun isUsable(path: Path): Boolean {
        return Files.isRegularFile(path) && (SystemInfoRt.isWindows || Files.isExecutable(path))
    }

    private fun binaryName(): String {
        return if (SystemInfoRt.isWindows) "$BINARY_NAME.exe" else BINARY_NAME
    }

    private fun platformDir(): String {
        val arch = if (isArm64()) "arm64" else "x64"
        return when {
            SystemInfoRt.isMac -> "darwin-$arch"
            SystemInfoRt.isLinux -> "linux-$arch"
            SystemInfoRt.isWindows -> "win32-x64"
            else -> "unknown-$arch"
        }
    }

    private fun isArm64(): Boolean {
        return System.getProperty("os.arch").lowercase() in setOf("aarch64", "arm64")
    }
}
