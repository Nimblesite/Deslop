package com.nimblesite.deslop.jetbrains

import java.nio.file.Files
import java.nio.file.Path
import kotlin.io.path.createDirectories
import kotlin.io.path.writeText
import kotlin.test.Test
import kotlin.test.assertContains
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

internal class DeslopBinaryResolverTest {
    private val platform = testPlatform()

    @Test
    fun envPathMismatchBlocksStartup() {
        val root = tempRoot()
        val envBin = script(root.resolve("env/deslop-lsp"), "deslop-lsp", "9.9.9")
        val error = assertFailsWith<DeslopBinaryResolutionException> {
            resolve(root, mapOf("DESLOP_LSP_PATH" to envBin.toString()))
        }
        assertContains(error.message.orEmpty(), "Found deslop-lsp 9.9.9")
        assertContains(error.message.orEmpty(), "env-path")
    }

    @Test
    fun envDirectoryMismatchBlocksStartup() {
        val root = tempRoot()
        script(root.resolve("env/deslop-lsp"), "deslop-lsp", "9.9.9")
        val error = assertFailsWith<DeslopBinaryResolutionException> {
            resolve(root, mapOf("DESLOP_BINARY_DIR" to root.resolve("env").toString()))
        }
        assertContains(error.message.orEmpty(), "env-dir")
    }

    @Test
    fun stalePathBinaryDoesNotOverrideBundledBinary() {
        val root = tempRoot()
        script(root.resolve("path/deslop-lsp"), "deslop-lsp", "9.9.9")
        bundled(root, "deslop-lsp", "0.1.0")
        val resolved = resolve(root, mapOf("PATH" to root.resolve("path").toString()))
        assertEquals("bundled", resolved.source)
        assertEquals("0.1.0", resolved.version)
    }

    @Test
    fun bundledBinaryStartsSuccessfully() {
        val root = tempRoot()
        bundled(root, "deslop-lsp", "0.1.0")
        val resolved = resolve(root, emptyMap())
        assertEquals(root.resolve("plugin/bin/$platform/deslop-lsp"), resolved.path)
    }

    @Test
    fun missingBundledBinaryReportsPathAndSource() {
        val root = tempRoot()
        val error = assertFailsWith<DeslopBinaryResolutionException> {
            resolve(root, emptyMap())
        }
        assertContains(error.message.orEmpty(), "was not found")
        assertContains(error.message.orEmpty(), "bundled")
    }

    @Test
    fun componentNameMismatchReportsExpectedAndFound() {
        val root = tempRoot()
        bundled(root, "deslop", "0.1.0")
        val error = assertFailsWith<DeslopBinaryResolutionException> {
            resolve(root, emptyMap())
        }
        assertContains(error.message.orEmpty(), "Expected 0.1.0")
        assertContains(error.message.orEmpty(), "Found deslop 0.1.0")
    }

    private fun resolve(
        root: Path,
        env: Map<String, String>,
    ): DeslopResolvedBinary {
        return DeslopBinaryResolver.resolveLsp(
            manifest(),
            ResolverInputs(root.resolve("plugin"), env, platform),
        )
    }

    private fun bundled(root: Path, name: String, version: String): Path {
        return script(root.resolve("plugin/bin/$platform/deslop-lsp"), name, version)
    }

    private fun script(path: Path, name: String, version: String): Path {
        path.parent.createDirectories()
        path.writeText("#!/bin/sh\necho '$name $version'\n")
        path.toFile().setExecutable(true)
        return path
    }

    private fun tempRoot(): Path {
        return Files.createTempDirectory("deslop-jetbrains-resolver-")
    }

    private fun manifest(): DeslopDeploymentManifest {
        return DeslopDeploymentManifest(
            components = listOf(
                ComponentContract(
                    id = "deslop-lsp",
                    binaryName = "deslop-lsp",
                    expectedVersion = "0.1.0",
                    bundleTemplate = "bin/\${platform}/\${binaryName}\${exe}",
                    pathVar = "DESLOP_LSP_PATH",
                    dirVar = "DESLOP_BINARY_DIR",
                ),
            ),
            jetBrainsActivationVerifies = setOf("deslop-lsp"),
        )
    }
}

private fun testPlatform(): String {
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
