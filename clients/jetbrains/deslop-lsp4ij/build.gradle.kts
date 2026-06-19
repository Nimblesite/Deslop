// The LSP4IJ plugin: Android Studio and IntelliJ Community, which do not ship the
// native LSP API. It registers deslop-lsp through Red Hat's LSP4IJ client instead.
// Compiled against the unified IntelliJ IDEA base (the IC/IU split ended in 2025.3);
// because plugin.xml depends only on com.intellij.modules.platform + LSP4IJ, the one
// artifact loads in Android Studio, Community, AND Rider/Ultimate. Distinct plugin id
// keeps it separable from the native build.
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
        // LSP4IJ 0.20.1: since-build 242, no upper bound → loads on 261 (2026.1).
        plugin("com.redhat.devtools.lsp4ij", "0.20.1")
    }
}

kotlin {
    jvmToolchain(17)
}

intellijPlatform {
    buildSearchableOptions = false
    pluginConfiguration {
        id = "nimblesite.deslop.jetbrains.community"
        name = "Deslop (Community / Android Studio)"
        version = project.version.toString()
        description =
            "Live duplicate-code analysis for Android Studio and IntelliJ Community, " +
            "bridging the Deslop LSP server through LSP4IJ."
        changeNotes = "First Android Studio / IntelliJ Community build via LSP4IJ."
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
