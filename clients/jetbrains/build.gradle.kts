import java.io.File

plugins {
    kotlin("jvm") version "2.3.20"
    id("org.jetbrains.intellij.platform") version "2.14.0"
}

group = "com.nimblesite"
version = "0.1.0"

repositories {
    mavenCentral()
    intellijPlatform {
        defaultRepositories()
    }
}

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
}

tasks.named("prepareSandbox") {
    doLast {
        val pluginRoots = pluginSandboxRoots()
        val manifest = rootProject.layout.projectDirectory.file("../../deployment-toolkit.json").asFile
        val lsp = lspBinary()
        pluginRoots.forEach { pluginRoot ->
            copy {
                from(manifest)
                into(pluginRoot)
            }
            val targetDir = pluginRoot.resolve("bin/${hostPlatform()}")
            copy {
                from(lsp)
                into(targetDir)
            }
            targetDir.resolve(lsp.name).setExecutable(true, false)
        }
    }
}

fun pluginSandboxRoots(): List<File> {
    val pluginsDir = layout.buildDirectory.dir("idea-sandbox/plugins").get().asFile
    val roots = pluginsDir.listFiles { file -> file.isDirectory }?.toList().orEmpty()
    if (roots.isEmpty()) throw GradleException("No JetBrains sandbox plugin root found.")
    return roots
}

fun lspBinary(): File {
    val name = if (hostPlatform().startsWith("win32")) "deslop-lsp.exe" else "deslop-lsp"
    val dir = System.getenv("DESLOP_BINARY_DIR")?.let(::file)
        ?: rootProject.layout.projectDirectory.dir("../../target/release").asFile
    val binary = dir.resolve(name)
    if (!binary.isFile) throw GradleException("Missing $binary; build deslop-lsp first.")
    return binary
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
