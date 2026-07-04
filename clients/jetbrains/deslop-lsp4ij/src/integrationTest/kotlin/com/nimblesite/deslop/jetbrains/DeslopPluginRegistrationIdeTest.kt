package com.nimblesite.deslop.jetbrains

import com.intellij.ide.plugins.PluginManagerCore
import com.intellij.openapi.actionSystem.ActionManager
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.extensions.PluginId
import com.intellij.openapi.wm.ToolWindowEP
import com.intellij.testFramework.TestApplicationManager
import com.redhat.devtools.lsp4ij.LanguageServersRegistry
import java.util.concurrent.atomic.AtomicReference
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

// [JETBRAINS-TESTING] / [JETBRAINS-UX] / [DEPLOY-JETBRAINS-PACKAGE]
// Real IDE-level guard for the packaging contract. IPGP installs the ASSEMBLED plugin into
// a fresh headless IntelliJ Platform (the testIde sandbox), so the plugin loads through its
// SHIPPED classloader layout, not from a flat test classpath. The tool window factory and
// Tools action live in :deslop-shared; if that jar ships under lib/modules/ (a content
// module behind a child classloader) instead of lib/, both extensions silently vanish. This
// test asserts they actually register — and its own source set deliberately excludes the
// plugin's production code, so the only path to those classes is the plugin classloader.
private const val PLUGIN_ID: String = "nimblesite.deslop.jetbrains.community"
private const val PLUGIN_NAME: String = "Deslop (Community / Android Studio)"
private const val TOOL_WINDOW_ID: String = "Deslop"
private const val TOOL_WINDOW_FACTORY: String = "com.nimblesite.deslop.jetbrains.DeslopReportToolWindowFactory"
private const val ACTION_ID: String = "Deslop.OpenHtmlReport"
private const val ACTION_TEXT: String = "Deslop: Open HTML Report"
private const val ACTION_CLASS: String = "com.nimblesite.deslop.jetbrains.DeslopRenderReportAction"
private const val SERVER_ID: String = "deslop"
private const val SERVER_NAME: String = "Deslop"

internal class DeslopPluginRegistrationIdeTest {
    @Test
    fun assembledPluginRegistersToolWindowActionAndServer() {
        TestApplicationManager.getInstance()
        runOnEdt {
            val pluginClassLoader = assertPluginLoadedFromMainClasspath()
            assertToolWindowRegistered(pluginClassLoader)
            assertOpenReportActionRegistered(pluginClassLoader)
            assertDeslopLanguageServerRegistered()
        }
    }

    /** The installed plugin, enabled, exposing the main classloader its extensions load from. */
    private fun assertPluginLoadedFromMainClasspath(): ClassLoader {
        val descriptor = PluginManagerCore.getPlugin(PluginId.getId(PLUGIN_ID))
        assertNotNull(descriptor, "the assembled Deslop plugin ($PLUGIN_ID) must be installed in the IDE")
        assertEquals(PLUGIN_NAME, descriptor.name, "the installed plugin must report its shipped display name")
        assertTrue(descriptor.isEnabled, "the Deslop plugin must load enabled, not be disabled for an unmet dependency")
        val classLoader = descriptor.pluginClassLoader
        assertNotNull(classLoader, "the loaded plugin must expose its main classloader")
        return classLoader
    }

    /** The `Deslop` tool window is registered and its factory resolves from the main classloader. */
    private fun assertToolWindowRegistered(pluginClassLoader: ClassLoader) {
        val toolWindow = ToolWindowEP.EP_NAME.extensionList.firstOrNull { it.id == TOOL_WINDOW_ID }
        assertNotNull(toolWindow, "a tool window with id '$TOOL_WINDOW_ID' must be registered via com.intellij.toolWindow")
        assertEquals(TOOL_WINDOW_FACTORY, toolWindow.factoryClass, "the tool window must point at the shipped factory class")
        // Teeth: the factory lives in :deslop-shared. It resolves only when that jar sits on
        // the MAIN plugin classpath (lib/*.jar). Under the broken lib/modules/ layout the
        // plugin classloader cannot see it and this throws — the exact bug the fix cures.
        val factoryClass = pluginClassLoader.loadClass(toolWindow.factoryClass)
        assertEquals(TOOL_WINDOW_FACTORY, factoryClass.name, "the tool window factory must load from the main plugin classloader")
    }

    /** The `Deslop.OpenHtmlReport` Tools-menu action is registered with its shipped title. */
    private fun assertOpenReportActionRegistered(pluginClassLoader: ClassLoader) {
        val action = ActionManager.getInstance().getAction(ACTION_ID)
        assertNotNull(action, "the Tools-menu action '$ACTION_ID' must be registered")
        assertEquals(ACTION_TEXT, action.templatePresentation.text, "the action must carry its shipped human-readable title")
        val actionClass = pluginClassLoader.loadClass(ACTION_CLASS)
        assertEquals(ACTION_CLASS, actionClass.name, "the action class must load from the main plugin classloader")
    }

    /** Bonus: the LSP4IJ `deslop` server (top-level jar) stays registered — LSP is unaffected. */
    private fun assertDeslopLanguageServerRegistered() {
        val definition = LanguageServersRegistry.getInstance().getServerDefinition(SERVER_ID)
        assertNotNull(definition, "the LSP4IJ server '$SERVER_ID' must be registered")
        assertEquals(SERVER_ID, definition.id, "the registered server must keep its wire id")
        assertEquals(SERVER_NAME, definition.displayName, "the registered server must keep its human-readable name")
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
