// The native-LSP plugin: IntelliJ IDEA Ultimate / Rider, using the platform's
// built-in com.intellij.platform.lsp.api. Behaviour is unchanged from the original
// single-module plugin — the only difference is that its non-surface code now comes
// from :deslop-shared. Keep plugin id nimblesite.deslop.jetbrains so the existing
// Marketplace listing and the bundled plugin-root lookup keep working.
plugins {
    kotlin("jvm")
    id("org.jetbrains.intellij.platform")
    id("deslop-jetbrains-bundling")
}

group = "com.nimblesite"
// Lock-step with the Rust binaries / VSIX: the release passes -PdeslopVersion=<tag>.
version = providers.gradleProperty("deslopVersion").getOrElse("0.1.0")

dependencies {
    implementation(project(":deslop-shared"))

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
        changeNotes = "Adds Dart support; shares the analysis bridge with the Android Studio / Community build."
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
