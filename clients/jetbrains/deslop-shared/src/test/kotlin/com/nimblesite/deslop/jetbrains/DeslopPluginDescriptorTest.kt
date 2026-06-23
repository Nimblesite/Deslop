package com.nimblesite.deslop.jetbrains

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

/**
 * Pins the IDE-visible surfaces declared in the shipped plugin.xml. When the
 * JetBrains plugin "shows nothing", the cause is almost always a registration that
 * silently went missing — a dropped tool window, an unbound render service, or a
 * renamed action class. These assertions fail fast on exactly that class of
 * regression, the same way the VS Code suite pins its views and commands.
 */
internal class DeslopPluginDescriptorTest {
    private val document = PluginDescriptor.lsp4ij()

    @Test
    fun registersTheDeslopReportToolWindow() {
        assertEquals(
            listOf("Deslop"),
            PluginDescriptor.attributeValues(document, "toolWindow", "id"),
            "the Deslop tool window is the plugin's visible report panel",
        )
        assertTrue(
            PluginDescriptor.attributeValues(document, "toolWindow", "factoryClass")
                .single().endsWith(".DeslopReportToolWindowFactory"),
            "the tool window must be built by DeslopReportToolWindowFactory",
        )
    }

    @Test
    fun bindsTheReportRendererService() {
        assertTrue(
            PluginDescriptor.attributeValues(document, "projectService", "serviceInterface")
                .single().endsWith(".DeslopReportRenderer"),
            "the shared report UI renders through the DeslopReportRenderer seam",
        )
        assertTrue(
            PluginDescriptor.attributeValues(document, "projectService", "serviceImplementation")
                .single().endsWith(".DeslopLsp4ijReportRenderer"),
            "LSP4IJ provides the render implementation",
        )
    }

    @Test
    fun registersTheOpenReportAction() {
        assertEquals(
            listOf("Deslop.OpenHtmlReport"),
            PluginDescriptor.attributeValues(document, "action", "id"),
            "the Tools menu and tool window toolbar share one report action",
        )
        assertTrue(
            PluginDescriptor.attributeValues(document, "action", "class")
                .single().endsWith(".DeslopRenderReportAction"),
            "the action is DeslopRenderReportAction",
        )
    }

    @Test
    fun registersTheLsp4ijServerFactory() {
        assertEquals(
            listOf("deslop"),
            PluginDescriptor.attributeValues(document, "server", "id"),
            "LSP4IJ starts the deslop server",
        )
        assertTrue(
            PluginDescriptor.attributeValues(document, "server", "factoryClass")
                .single().endsWith(".DeslopLsp4ijServerFactory"),
            "the server is launched through DeslopLsp4ijServerFactory",
        )
    }
}
