package com.nimblesite.deslop.jetbrains

import com.intellij.ide.plugins.PluginManagerCore
import com.intellij.openapi.extensions.PluginId
import java.io.File
import java.nio.file.Files
import java.nio.file.Path
import java.util.concurrent.TimeUnit
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive

internal object DeslopBinaryResolver {
    private const val PLUGIN_ID_VALUE = "nimblesite.deslop.jetbrains"

    fun resolveLsp(): DeslopResolvedBinary {
        val pluginRoot = pluginRoot()
        val manifest = DeslopDeploymentManifest.load(pluginRoot.resolve("deployment-toolkit.json"))
        return resolveLsp(manifest, ResolverInputs(pluginRoot = pluginRoot))
    }

    fun resolveLsp(
        manifest: DeslopDeploymentManifest,
        inputs: ResolverInputs,
    ): DeslopResolvedBinary {
        val component = manifest.requiredJetBrainsComponent("deslop-lsp")
        var skippedPath: Candidate? = null
        for (candidate in candidates(component, inputs)) {
            val resolved = verifyCandidate(component, candidate)
            if (resolved != null) return resolved
            skippedPath = candidate
        }
        throw DeslopBinaryResolutionException(missingMessage(component, skippedPath))
    }

    private fun candidates(
        component: ComponentContract,
        inputs: ResolverInputs,
    ): List<Candidate> {
        return listOfNotNull(
            envPathCandidate(component, inputs),
            envDirCandidate(component, inputs),
            bundledCandidate(component, inputs),
            pathCandidate(component, inputs),
        )
    }

    private fun envPathCandidate(
        component: ComponentContract,
        inputs: ResolverInputs,
    ): Candidate? {
        val pathVar = component.pathVar ?: return null
        val configured = inputs.env[pathVar]?.takeIf(String::isNotBlank) ?: return null
        return Candidate("env-path", Path.of(configured), hardFailure = true)
    }

    private fun envDirCandidate(
        component: ComponentContract,
        inputs: ResolverInputs,
    ): Candidate? {
        val dirVar = component.dirVar ?: return null
        val configured = inputs.env[dirVar]?.takeIf(String::isNotBlank) ?: return null
        return Candidate("env-dir", Path.of(configured).resolve(component.binaryName(inputs.platform)), true)
    }

    private fun bundledCandidate(
        component: ComponentContract,
        inputs: ResolverInputs,
    ): Candidate? {
        val pluginRoot = inputs.pluginRoot ?: return null
        return Candidate("bundled", pluginRoot.resolve(component.bundlePath(inputs.platform)), true)
    }

    private fun pathCandidate(
        component: ComponentContract,
        inputs: ResolverInputs,
    ): Candidate? {
        val pathValue = inputs.env["PATH"] ?: inputs.env["Path"] ?: return null
        return pathValue.split(File.pathSeparator)
            .filter(String::isNotBlank)
            .map { Path.of(it).resolve(component.binaryName(inputs.platform)) }
            .firstOrNull(Files::isRegularFile)
            ?.let { Candidate("path", it, hardFailure = false) }
    }

    private fun verifyCandidate(
        component: ComponentContract,
        candidate: Candidate,
    ): DeslopResolvedBinary? {
        if (!Files.isRegularFile(candidate.path)) return handleMissing(component, candidate)
        val probe = VersionProbe.read(candidate.path)
        if (probe.name == component.id && probe.version == component.expectedVersion) {
            return DeslopResolvedBinary(component.id, candidate.path, candidate.source, probe.version)
        }
        if (!candidate.hardFailure) return null
        throw DeslopBinaryResolutionException(mismatchMessage(component, candidate, probe.found()))
    }

    private fun handleMissing(
        component: ComponentContract,
        candidate: Candidate,
    ): DeslopResolvedBinary? {
        if (!candidate.hardFailure) return null
        throw DeslopBinaryResolutionException(
            "Deslop cannot start: ${component.id} ${component.expectedVersion} was not found at " +
                "${candidate.path} from ${candidate.source}.",
        )
    }

    private fun pluginRoot(): Path {
        val plugin = PluginManagerCore.getPlugin(PluginId.getId(PLUGIN_ID_VALUE))
            ?: throw DeslopBinaryResolutionException("Deslop plugin root is unavailable.")
        return plugin.pluginPath
    }
}

internal data class ResolverInputs(
    val pluginRoot: Path? = null,
    val env: Map<String, String> = System.getenv(),
    val platform: String = currentPlatform(),
)

internal data class DeslopResolvedBinary(
    val componentId: String,
    val path: Path,
    val source: String,
    val version: String,
)

internal class DeslopBinaryResolutionException(message: String) : RuntimeException(message)

internal data class DeslopDeploymentManifest(
    val components: List<ComponentContract>,
    val jetBrainsActivationVerifies: Set<String>,
) {
    fun requiredJetBrainsComponent(id: String): ComponentContract {
        if (id !in jetBrainsActivationVerifies) {
            throw DeslopBinaryResolutionException("JetBrains host does not verify $id.")
        }
        return components.firstOrNull { it.id == id }
            ?: throw DeslopBinaryResolutionException("deployment-toolkit.json is missing $id.")
    }

    companion object {
        fun load(path: Path): DeslopDeploymentManifest {
            val root = Json.parseToJsonElement(Files.readString(path)).jsonObject
            return DeslopDeploymentManifest(parseComponents(root), parseJetBrainsHost(root))
        }

        private fun parseComponents(root: JsonObject): List<ComponentContract> {
            return root["components"]?.jsonArray.orEmpty()
                .mapNotNull { ComponentContract.fromJson(it.jsonObject) }
        }

        private fun parseJetBrainsHost(root: JsonObject): Set<String> {
            return root["hosts"]?.jsonObject
                ?.get("jetbrains")?.jsonObject
                ?.get("activationVerifies")?.jsonArray
                ?.map { it.jsonPrimitive.content }
                ?.toSet()
                .orEmpty()
        }
    }
}

internal data class ComponentContract(
    val id: String,
    val binaryName: String,
    val expectedVersion: String,
    val bundleTemplate: String?,
    val pathVar: String?,
    val dirVar: String?,
) {
    fun binaryName(platform: String): String {
        return if (platform.startsWith("win32")) "$binaryName.exe" else binaryName
    }

    fun bundlePath(platform: String): Path {
        val template = bundleTemplate
            ?: throw DeslopBinaryResolutionException("$id has no bundled path.")
        val value = template
            .replace("\${platform}", platform)
            .replace("\${binaryName}", binaryName)
            .replace("\${exe}", if (platform.startsWith("win32")) ".exe" else "")
        return Path.of(value)
    }

    companion object {
        fun fromJson(json: JsonObject): ComponentContract? {
            val kind = json.stringValue("kind") ?: return null
            if (kind !in setOf("cli", "lsp", "mcp")) return null
            return ComponentContract(
                id = stringAt(json, "id"),
                binaryName = stringAt(json, "binaryName"),
                expectedVersion = stringAt(json, "expectedVersion"),
                bundleTemplate = objectAt(json, "bundled")?.stringValue("bundlePath"),
                pathVar = objectAt(json, "env")?.stringValue("pathVar"),
                dirVar = objectAt(json, "env")?.stringValue("dirVar"),
            )
        }
    }
}

private data class Candidate(
    val source: String,
    val path: Path,
    val hardFailure: Boolean,
)

private data class VersionProbe(
    val name: String?,
    val version: String?,
    val raw: String,
) {
    fun found(): String {
        return if (name != null && version != null) "$name $version" else raw.ifBlank { "not found" }
    }

    companion object {
        fun read(path: Path): VersionProbe {
            return runCatching { readProcess(path) }
                .getOrElse { VersionProbe(null, null, it.message.orEmpty()) }
        }

        private fun readProcess(path: Path): VersionProbe {
            val process = ProcessBuilder(path.toString(), "--version").start()
            if (!process.waitFor(1500, TimeUnit.MILLISECONDS)) {
                process.destroyForcibly()
                return VersionProbe(null, null, "timeout")
            }
            val line = process.inputStream.bufferedReader().readLine().orEmpty()
            if (process.exitValue() != 0) return VersionProbe(null, null, line)
            val parts = line.trim().split(" ")
            return if (parts.size == 2) VersionProbe(parts[0], parts[1], line)
            else VersionProbe(null, null, line)
        }
    }
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

private fun missingMessage(component: ComponentContract, skippedPath: Candidate?): String {
    val suffix = skippedPath?.let { " Last checked: ${it.path} from ${it.source}." }.orEmpty()
    return "No matching ${component.id} ${component.expectedVersion} binary found.$suffix"
}

private fun mismatchMessage(
    component: ComponentContract,
    candidate: Candidate,
    found: String,
): String {
    return "Deslop cannot start: ${component.id} version mismatch. " +
        "Expected ${component.expectedVersion} from deployment-toolkit.json. " +
        "Found $found at ${candidate.path} from ${candidate.source}."
}

private fun stringAt(json: JsonObject, key: String): String {
    return json.stringValue(key)
        ?: throw DeslopBinaryResolutionException("deployment-toolkit.json is missing $key.")
}

private fun objectAt(json: JsonObject, key: String): JsonObject? {
    return json[key]?.jsonObject
}

private fun JsonObject.stringValue(key: String): String? {
    return (this[key] as? JsonPrimitive)?.content
}
