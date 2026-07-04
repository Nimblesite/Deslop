// The one shipped Deslop JetBrains plugin: registers deslop-lsp through Red Hat's
// LSP4IJ client, which every IDE family ships support for (Android Studio, IntelliJ
// Community, and — with the LSP4IJ plugin installed — Rider / IDEA Ultimate). Because
// plugin.xml depends only on com.intellij.modules.platform + LSP4IJ, this single
// artifact covers every family, so there is no separate native-LSP build to produce.
// Compiled against IntelliJ IDEA Community 2024.3 (build 243) — the same floor the
// since-build below declares — so it loads on, and only references APIs present in,
// every Android Studio from Meerkat (2024.3) onward.
import org.jetbrains.intellij.platform.gradle.IntelliJPlatformType
import org.jetbrains.intellij.platform.gradle.TestFrameworkType

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
        // Headless IDE Application fixture (TestApplicationManager) for the IDE-level
        // integration test; also puts the platform test framework on the test classpaths.
        testFramework(TestFrameworkType.Platform)
    }
}

// [JETBRAINS-TESTING] Real IDE-level registration test. Its own source set carries NO
// dependency on the plugin's production code (:deslop-shared, main output), so the tool
// window factory and Tools action reach it ONLY through the installed plugin's classloader.
// A flat test classpath carrying those classes would mask the lib/modules/ packaging bug —
// exactly the false-green the task warns against — so they are deliberately absent here.
// The source set compiles against the IntelliJ Platform + test framework + LSP4IJ only.
val integrationTest: SourceSet = sourceSets.create("integrationTest")

// intellijPlatformClasspath = the extracted IDE product jars (ApplicationManager,
// ToolWindowEP, ActionManager, LSP4IJ); intellijPlatformTestClasspath = the platform test
// framework (TestApplicationManager). Main gets the former, `test` the latter, so a custom
// source set needs both to compile the IDE-level assertions.
listOf(
    integrationTest.compileClasspathConfigurationName,
    integrationTest.runtimeClasspathConfigurationName,
).forEach { configurationName ->
    configurations[configurationName].extendsFrom(
        configurations["intellijPlatformClasspath"],
        configurations["intellijPlatformTestClasspath"],
    )
}

dependencies {
    add(integrationTest.implementationConfigurationName, kotlin("test-junit5"))
    // Same JUnit4 shim the sibling modules pin: the 2024.3 platform's JUnit5
    // LauncherSessionListener hard-references JUnit4 at session open.
    add(integrationTest.runtimeOnlyConfigurationName, "junit:junit:4.13.2")
}

kotlin {
    jvmToolchain(17)
}

tasks.test {
    useJUnitPlatform()
}

// [JETBRAINS-TESTING] IPGP 2.14 testIde: installs the ASSEMBLED plugin (its own
// prepareSandbox, subject to the deslop-jetbrains-bundling flatten) into a fresh headless
// IntelliJ Platform, then runs the integrationTest source set inside it. Unlike a flat
// BasePlatformTestCase this exercises the shipped classloader layout — lib/ vs lib/modules/
// changes the outcome. Named "integrationTest", so the task is :deslop-lsp4ij:integrationTest.
intellijPlatformTesting {
    testIde {
        register("integrationTest") {
            // Pin to the module's floor/compile IDE so the test runs the exact platform
            // the plugin ships against (243 = 2024.3 = Android Studio Meerkat).
            type = IntelliJPlatformType.IntellijIdeaCommunity
            version = "2024.3"
            // The plugin <depends> on LSP4IJ; install it into the test sandbox so the
            // Deslop plugin loads enabled — a missing dependency would disable it and mask
            // the classloader assertions behind an unrelated failure.
            plugins {
                plugin("com.redhat.devtools.lsp4ij", "0.20.1")
            }
            task {
                useJUnitPlatform()
                testClassesDirs = integrationTest.output.classesDirs
                classpath = integrationTest.runtimeClasspath
            }
        }
    }
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
