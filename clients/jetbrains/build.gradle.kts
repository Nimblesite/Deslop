import java.io.File
import java.nio.file.Files
import java.nio.file.StandardCopyOption

import org.gradle.api.DefaultTask
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.file.RegularFileProperty
import org.gradle.api.provider.Property
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.InputFile
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.PathSensitive
import org.gradle.api.tasks.PathSensitivity
import org.gradle.api.tasks.TaskAction
import org.jetbrains.intellij.platform.gradle.tasks.BuildPluginTask
import org.jetbrains.intellij.platform.gradle.tasks.PrepareSandboxTask

plugins {
    kotlin("jvm") version "2.3.20"
    id("org.jetbrains.intellij.platform")
}

group = "com.nimblesite"
version = "0.1.0"

dependencies {
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.9.0")
    testImplementation(kotlin("test-junit5"))

    intellijPlatform {
        intellijIdea("2026.1")
    }
}

kotlin {
    jvmToolchain(17)
}

intellijPlatform {
    buildSearchableOptions = false
    pluginConfiguration {
        id = "nimblesite.deslop.jetbrains"
        name = "Deslop"
        version = project.version.toString()
        description = "Live duplicate-code analysis for JetBrains IDEs through the Deslop LSP server."
        changeNotes = "Initial Rider-first LSP bridge."
        ideaVersion {
            sinceBuild = "261"
            untilBuild = provider { null }
        }
        vendor {
            name = "Nimblesite"
            url = "https://github.com/Nimblesite/Deslop"
        }
    }
}

tasks.test {
    useJUnitPlatform()
    // The real-binary contract test (DeslopRealBinaryContractTest) is opt-in
    // via this system property. It depends on a release build of deslop-lsp
    // existing at target/release/. CI sets DESLOP_LSP_REAL_BINARY to the
    // built path; locally `make jetbrains-real-binary-test` runs it.
    System.getenv("DESLOP_LSP_REAL_BINARY")?.let { systemProperty("deslop.lsp.realBinary", it) }
}

val hostPlatformName = hostPlatform()
val lspBinaryName = if (hostPlatformName.startsWith("win32")) "deslop-lsp.exe" else "deslop-lsp"
val binaryDirectory = System.getenv("DESLOP_BINARY_DIR")?.let(::File)
    ?: rootProject.layout.projectDirectory.dir("../../target/release").asFile
val lspBinaryFile = binaryDirectory.resolve(lspBinaryName)
val deploymentManifestFile = rootProject.layout.projectDirectory
    .file("../../deployment-toolkit.json")
    .asFile

abstract class CopyLspArtifactsToSandbox : DefaultTask() {
    @get:InputFile
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val deploymentManifest: RegularFileProperty

    @get:InputFile
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val lspBinary: RegularFileProperty

    @get:OutputDirectory
    abstract val pluginDirectory: DirectoryProperty

    @get:Input
    abstract val hostPlatform: Property<String>

    @TaskAction
    fun copyArtifacts() {
        val binaryFile = lspBinary.get().asFile
        if (!binaryFile.isFile) throw GradleException("Missing $binaryFile; build deslop-lsp first.")

        val pluginRoot = pluginDirectory.get().asFile.toPath()
        val targetDir = pluginRoot.resolve("bin").resolve(hostPlatform.get())
        val targetBinary = targetDir.resolve(binaryFile.name)
        Files.createDirectories(pluginRoot)
        Files.createDirectories(targetDir)
        Files.copy(
            deploymentManifest.get().asFile.toPath(),
            pluginRoot.resolve("deployment-toolkit.json"),
            StandardCopyOption.REPLACE_EXISTING,
        )
        Files.copy(binaryFile.toPath(), targetBinary, StandardCopyOption.REPLACE_EXISTING)
        targetBinary.toFile().setExecutable(true, false)
    }
}

val prepareSandbox = tasks.named<PrepareSandboxTask>("prepareSandbox")
val copyLspArtifactsToSandbox = tasks.register<CopyLspArtifactsToSandbox>("copyLspArtifactsToSandbox") {
    dependsOn(prepareSandbox)
    deploymentManifest.set(deploymentManifestFile)
    lspBinary.set(lspBinaryFile)
    pluginDirectory.set(prepareSandbox.flatMap { it.pluginDirectory })
    hostPlatform.set(hostPlatformName)
}

tasks.named<BuildPluginTask>("buildPlugin") {
    dependsOn(copyLspArtifactsToSandbox)
    eachFile {
        if (!isDirectory && path.contains("/bin/")) {
            permissions { unix("rwxr-xr-x") }
        }
    }
}

fun hostPlatform(): String {
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
