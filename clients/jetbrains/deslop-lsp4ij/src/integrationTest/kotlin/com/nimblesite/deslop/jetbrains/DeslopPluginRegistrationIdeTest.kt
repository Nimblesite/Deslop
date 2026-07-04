package com.nimblesite.deslop.jetbrains

import com.intellij.ide.plugins.PluginManagerCore
import com.intellij.openapi.actionSystem.ActionManager
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.extensions.PluginId
import com.intellij.openapi.wm.ToolWindowEP
import com.intellij.testFramework.TestApplicationManager
import java.util.concurrent.atomic.AtomicReference
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

// [JETBRAINS-TESTING] / [JETBRAINS-UX] / [DEPLOY-JETBRAINS-PACKAGE]
// Real IDE-level guard for the packaging contract. IPGP installs the ASSEMBLED plugin into a
// fresh headless IntelliJ Platform (the testIde sandbox) and — because the build forces a real
// per-plugin classloader (idea.use.core.classloader.for.plugin.path=false, the production
// behaviour) — the plugin resolves its extension classes through its SHIPPED classloader
// layout, not a flat test classpath. The tool window factory and Tools action live in
// :deslop-shared; if that jar ships under lib/modules/ (a content module behind a child
// classloader) instead of lib/, both extensions silently vanish. This test asserts they
// actually register, and its own source set + classpath deliberately exclude every Deslop
// production jar, so the installed plugin's classloader is the only path to those classes.
private const val PLUGIN_ID: String = "nimblesite.deslop.jetbrains.community"
private const val PLUGIN_NAME: String = "Deslop (Community / Android Studio)"
private const val TOOL_WINDOW_ID: String = "Deslop"
private const val TOOL_WINDOW_FACTORY: String = "com.nimblesite.deslop.jetbrains.DeslopReportToolWindowFactory"
private const val ACTION_ID: String = "Deslop.OpenHtmlReport"
private const val ACTION_TEXT: String = "Deslop: Open HTML Report"
private const val ACTION_CLASS: String = "com.nimblesite.deslop.jetbrains.DeslopRenderReportAction"

internal class DeslopPluginRegistrationIdeTest {
    @Test
    fun assembledPluginRegistersToolWindowAndAction() {
        TestApplicationManager.getInstance()
        runOnEdt {
            val pluginClassLoader = assertPluginLoadedEnabled()
            assertToolWindowRegistered(pluginClassLoader)
            assertOpenReportActionRegistered(pluginClassLoader)
        }
    }

    /** The installed plugin, enabled, exposing the classloader its extension classes load from. */
    private fun assertPluginLoadedEnabled(): ClassLoader {
        val descriptor = PluginManagerCore.getPlugin(PluginId.getId(PLUGIN_ID))
        assertNotNull(descriptor, "the assembled Deslop plugin ($PLUGIN_ID) must be installed in the IDE")
        assertEquals(PLUGIN_NAME, descriptor.name, "the installed plugin must report its shipped display name")
        assertTrue(descriptor.isEnabled, "the Deslop plugin must load enabled, not be disabled for an unmet dependency")
        val classLoader = descriptor.pluginClassLoader
        assertNotNull(classLoader, "the loaded plugin must expose its own classloader")
        return classLoader
    }

    /** The `Deslop` tool window is registered and its factory resolves from the plugin classloader. */
    private fun assertToolWindowRegistered(pluginClassLoader: ClassLoader) {
        val toolWindow = ToolWindowEP.EP_NAME.extensionList.firstOrNull { it.id == TOOL_WINDOW_ID }
        assertNotNull(toolWindow, "a tool window with id '$TOOL_WINDOW_ID' must be registered via com.intellij.toolWindow")
        assertEquals(TOOL_WINDOW_FACTORY, toolWindow.factoryClass, "the tool window must point at the shipped factory class")
        // Teeth: the factory lives in :deslop-shared. It resolves only when that jar sits on the
        // plugin's main classpath (lib/*.jar). Under the broken lib/modules/ layout the plugin
        // classloader cannot see it and this throws — the exact bug the flatten fix cures.
        val factoryClass = pluginClassLoader.loadClass(toolWindow.factoryClass)
        assertEquals(TOOL_WINDOW_FACTORY, factoryClass.name, "the tool window factory must load from the plugin classloader")
    }

    /** The `Deslop.OpenHtmlReport` Tools-menu action is registered with its shipped title. */
    private fun assertOpenReportActionRegistered(pluginClassLoader: ClassLoader) {
        val action = ActionManager.getInstance().getAction(ACTION_ID)
        assertNotNull(action, "the Tools-menu action '$ACTION_ID' must be registered")
        assertEquals(ACTION_TEXT, action.templatePresentation.text, "the action must carry its shipped human-readable title")
        val actionClass = pluginClassLoader.loadClass(ACTION_CLASS)
        assertEquals(ACTION_CLASS, actionClass.name, "the action class must load from the plugin classloader")
    }
}

/** Runs [block] on the EDT (platform services assert it) and rethrows its failure on the test thread. */
private fun runOnEdt(block: () -> Unit) {
    val failure = AtomicReference<Throwable?>()
    ApplicationManager.getApplication().invokeAndWait {
        runCatching(block).exceptionOrNull()?.let(failure::set)
    }
    failure.get()?.let { throw it }
}
