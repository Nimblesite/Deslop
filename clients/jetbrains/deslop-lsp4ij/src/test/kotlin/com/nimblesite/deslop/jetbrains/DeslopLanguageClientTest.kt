package com.nimblesite.deslop.jetbrains

import com.intellij.openapi.project.Project
import org.eclipse.lsp4j.jsonrpc.services.JsonNotification
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull

/**
 * Pins the reactive wiring that makes the JetBrains panel live like the VS Code
 * client. The two halves — the server broadcasting `deslop/reportChanged` and the
 * client refreshing on it — only connect if the client method keeps its
 * `@JsonNotification` binding and the factory keeps overriding client creation.
 * Reflection (no IDE Application) so the guard is fast and deterministic.
 */
internal class DeslopLanguageClientTest {
    @Test
    fun reactiveRefreshIsBoundToTheReportChangedNotification() {
        val handler = DeslopLanguageClient::class.java.getDeclaredMethod("reportChanged", Any::class.java)

        val notification = handler.getAnnotation(JsonNotification::class.java)

        assertNotNull(notification, "the live refresh must be bound to a server notification")
        assertEquals(
            "deslop/reportChanged",
            notification.value,
            "the client must refresh on the exact method deslop-lsp broadcasts",
        )
    }

    @Test
    fun serverFactoryOverridesLanguageClientCreation() {
        // A declared (not inherited) override is what makes LSP4IJ deliver the custom
        // notification to DeslopLanguageClient instead of the base client that drops it.
        val override = DeslopLsp4ijServerFactory::class.java
            .getDeclaredMethod("createLanguageClient", Project::class.java)

        assertEquals(
            "com.redhat.devtools.lsp4ij.client.LanguageClientImpl",
            override.returnType.name,
            "createLanguageClient must be overridden so the reactive client is installed",
        )
    }
}
