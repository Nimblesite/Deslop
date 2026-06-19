package com.nimblesite.deslop.jetbrains

import java.nio.file.Files
import java.nio.file.Path
import kotlin.io.path.copyTo
import kotlin.io.path.createDirectories
import kotlin.io.path.exists
import kotlin.test.Test
import kotlin.test.assertContains
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

/**
 * End-to-end proof that the JetBrains resolver accepts the *real* released
 * deslop-lsp binary, and rejects it the moment the manifest contract drifts.
 *
 * Shell-stub tests in [DeslopBinaryResolverTest] exercise the resolver state
 * machine, but a stub never proves the actual production binary's --version
 * output matches what the resolver parses. This test wires the two ends
 * together: it copies the real release binary into a synthetic plugin root
 * and runs the resolver against it. If the binary's --version contract ever
 * drifts (line ending, prefix, version string), this test fails immediately.
 *
 * Activated via the `deslop.lsp.realBinary` system property which is set by
 * the Gradle task only when DESLOP_LSP_REAL_BINARY env var is exported. CI
 * exports it after the Rust release build; the suite is skipped otherwise so
 * a fresh checkout with no Rust artifacts can still run the rest of the
 * resolver tests.
 */
internal class DeslopRealBinaryContractTest {
    private val realBinary: Path? = System.getProperty("deslop.lsp.realBinary")
        ?.takeIf(String::isNotBlank)
        ?.let(Path::of)
        ?.takeIf(Path::exists)

    private val platform = currentPlatform()

    @Test
    fun realBinaryResolvesAgainstRepoManifest() {
        val binary = realBinary ?: return
        val pluginRoot = stageBundledBinary(binary)
        val manifest = loadRepoManifest()
        val expectedVersion = manifest.requiredJetBrainsComponent("deslop-lsp").expectedVersion

        val resolved = DeslopBinaryResolver.resolveLsp(
            manifest,
            ResolverInputs(pluginRoot = pluginRoot, env = emptyMap(), platform = platform),
        )

        assertEquals("deslop-lsp", resolved.componentId)
        assertEquals(expectedVersion, resolved.version)
        assertEquals("bundled", resolved.source)
    }

    @Test
    fun realBinaryIsRejectedWhenManifestExpectsDifferentVersion() {
        val binary = realBinary ?: return
        val pluginRoot = stageBundledBinary(binary)
        val actualVersion = loadRepoManifest().requiredJetBrainsComponent("deslop-lsp").expectedVersion
        val drifted = manifestExpectingVersion("9.9.9")

        val error = assertFailsWith<DeslopBinaryResolutionException> {
            DeslopBinaryResolver.resolveLsp(
                drifted,
                ResolverInputs(pluginRoot = pluginRoot, env = emptyMap(), platform = platform),
            )
        }
        val message = error.message.orEmpty()
        assertContains(message, "Expected 9.9.9")
        assertContains(message, "Found deslop-lsp $actualVersion")
        assertContains(message, "bundled")
    }

    private fun stageBundledBinary(source: Path): Path {
        val root = Files.createTempDirectory("deslop-real-binary-")
        val pluginRoot = root.resolve("plugin")
        val target = pluginRoot.resolve("bin/$platform/deslop-lsp")
        target.parent.createDirectories()
        source.copyTo(target)
        target.toFile().setExecutable(true, false)
        return pluginRoot
    }

    private fun loadRepoManifest(): DeslopDeploymentManifest {
        val repoRoot = System.getProperty("deslop.repoRoot")
            ?: error("deslop.repoRoot system property must be set by the Gradle test task")
        return DeslopDeploymentManifest.load(Path.of(repoRoot, "shipwright.json"))
    }

    private fun manifestExpectingVersion(version: String): DeslopDeploymentManifest {
        return DeslopDeploymentManifest(
            components = listOf(
                ComponentContract(
                    id = "deslop-lsp",
                    binaryName = "deslop-lsp",
                    expectedVersion = version,
                    bundleTemplate = "bin/\${platform}/\${binaryName}\${exe}",
                    pathVar = "DESLOP_LSP_PATH",
                    dirVar = "DESLOP_BINARY_DIR",
                ),
            ),
            jetBrainsActivationVerifies = setOf("deslop-lsp"),
        )
    }

    private fun currentPlatform(): String {
        val arch = if (System.getProperty("os.arch").lowercase() in setOf("aarch64", "arm64")) {
            "arm64"
        } else {
            "x64"
        }
        val name = System.getProperty("os.name").lowercase()
        return when {
            name.contains("mac") -> "darwin-$arch"
            name.contains("linux") -> "linux-$arch"
            name.contains("windows") -> "win32-x64"
            else -> "unknown-$arch"
        }
    }
}
