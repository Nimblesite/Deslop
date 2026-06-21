package com.nimblesite.deslop.jetbrains.gradle

import java.io.File
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.StandardCopyOption
import org.gradle.api.DefaultTask
import org.gradle.api.GradleException
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.file.RegularFileProperty
import org.gradle.api.provider.Property
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.InputDirectory
import org.gradle.api.tasks.InputFile
import org.gradle.api.tasks.Optional
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.PathSensitive
import org.gradle.api.tasks.PathSensitivity
import org.gradle.api.tasks.TaskAction

/**
 * Stages `shipwright.json` and the `deslop-lsp` binaries into a prepared plugin
 * sandbox. Two modes, both content-tracked for correct up-to-date checking:
 *
 *  - **all-platform** (release): [bundleSource] is laid out as
 *    `<platform>/deslop-lsp[.exe]`; every platform is staged so the published zip
 *    carries the binary for each OS/arch ([JETBRAINS-PACKAGING] offline install).
 *  - **host-only** (local dev): just [hostBinary] for [hostPlatform].
 *
 * Shared by both plugin artifacts through the `deslop-jetbrains-bundling`
 * convention plugin so the bundling logic lives exactly once.
 */
abstract class CopyLspArtifactsToSandbox : DefaultTask() {
    @get:InputFile
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val deploymentManifest: RegularFileProperty

    @get:Optional
    @get:InputDirectory
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val bundleSource: DirectoryProperty

    @get:Optional
    @get:InputFile
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val hostBinary: RegularFileProperty

    @get:Optional
    @get:Input
    abstract val hostPlatform: Property<String>

    @get:OutputDirectory
    abstract val pluginDirectory: DirectoryProperty

    @TaskAction
    fun copyArtifacts() {
        val pluginRoot = pluginDirectory.get().asFile.toPath()
        Files.createDirectories(pluginRoot)
        Files.copy(
            deploymentManifest.get().asFile.toPath(),
            pluginRoot.resolve("shipwright.json"),
            StandardCopyOption.REPLACE_EXISTING,
        )
        val staged = if (bundleSource.isPresent) stageAllPlatforms(pluginRoot) else stageHost(pluginRoot)
        if (staged == 0) throw GradleException("No deslop-lsp binaries staged; build deslop-lsp first.")
    }

    private fun stageAllPlatforms(pluginRoot: Path): Int {
        val platformDirs = bundleSource.get().asFile.listFiles()?.filter(File::isDirectory).orEmpty()
        return platformDirs.sumOf { platformDir ->
            platformDir.listFiles()?.filter(File::isFile).orEmpty()
                .onEach { stageBinary(pluginRoot, platformDir.name, it) }
                .size
        }
    }

    private fun stageHost(pluginRoot: Path): Int {
        val binary = hostBinary.get().asFile
        if (!binary.isFile) throw GradleException("Missing $binary; build deslop-lsp first.")
        stageBinary(pluginRoot, hostPlatform.get(), binary)
        return 1
    }

    private fun stageBinary(pluginRoot: Path, platform: String, binary: File) {
        val targetDir = pluginRoot.resolve("bin").resolve(platform)
        Files.createDirectories(targetDir)
        val target = targetDir.resolve(binary.name)
        Files.copy(binary.toPath(), target, StandardCopyOption.REPLACE_EXISTING)
        target.toFile().setExecutable(true, false)
    }
}
