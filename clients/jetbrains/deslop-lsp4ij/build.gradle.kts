// The one shipped Deslop JetBrains plugin: registers deslop-lsp through Red Hat's
// LSP4IJ client, which every IDE family ships support for (Android Studio, IntelliJ
// Community, and — with the LSP4IJ plugin installed — Rider / IDEA Ultimate). Because
// plugin.xml depends only on com.intellij.modules.platform + LSP4IJ, this single
// artifact covers every family, so there is no separate native-LSP build to produce.
// Compiled against IntelliJ IDEA Community 2024.3 (build 243) — the same floor the
// since-build below declares — so it loads on, and only references APIs present in,
// every Android Studio from Meerkat (2024.3) onward.
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
    testImplementation(kotlin("test-junit5"))
    // Satisfies the 2024.3 platform's JUnit5 LauncherSessionListener, which
    // hard-references JUnit4 at session open (see the deslop-shared note).
    testRuntimeOnly("junit:junit:4.13.2")

    intellijPlatform {
        intellijIdeaCommunity("2024.3")
        // LSP4IJ 0.20.1: since-build 242, no upper bound → loads on 243 (2024.3) up.
        plugin("com.redhat.devtools.lsp4ij", "0.20.1")
    }
}

kotlin {
    jvmToolchain(17)
}

tasks.test {
    useJUnitPlatform()
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
        changeNotes =
            "<ul>" +
            "<li>Live <b>Deslop</b> tool window (right-hand stripe): the worst-offenders " +
            "report renders from the same engine as the VS Code client and refreshes " +
            "in place as you edit.</li>" +
            "<li>Duplicate regions surface as native diagnostics for C#, Rust, Python, " +
            "Dart, JavaScript, and TypeScript files.</li>" +
            "<li>First Android Studio / IntelliJ Community build via LSP4IJ.</li>" +
            "</ul>"
        ideaVersion {
            // 243 = IntelliJ 2024.3 = Android Studio Meerkat. Android Studio trails
            // the IntelliJ platform by several releases, so the previous 261 (IDEA
            // 2026.1) floor excluded every shipping Android Studio. No upper bound.
            sinceBuild = "243"
            untilBuild = provider { null }
        }
        vendor {
            name = "Nimblesite"
            url = "https://github.com/Nimblesite/Deslop"
        }
    }
}
