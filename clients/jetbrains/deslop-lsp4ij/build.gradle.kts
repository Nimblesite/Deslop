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

dependencies {
    // Compile-time only: the extracted IDE product jars (ApplicationManager, ToolWindowEP,
    // ActionManager, PluginManagerCore) and the platform test framework (TestApplicationManager).
    // These files() resolve each config with its own attributes — enough to COMPILE against —
    // while the complete RUNTIME platform comes from the TestIdeTask's own platformPath below
    // (the raw configs omit testFramework.jar, which the platform's session listener needs).
    add(integrationTest.compileOnlyConfigurationName, files(configurations["intellijPlatformClasspath"]))
    add(integrationTest.compileOnlyConfigurationName, files(configurations["intellijPlatformTestClasspath"]))
    add(integrationTest.implementationConfigurationName, kotlin("test-junit5"))
    // The 2024.3 platform's JUnit5 session listener hard-references JUnit4 at session open,
    // so JUnit4 must be on the runtime classpath even though the tests are pure JUnit5.
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
                // The borrowed `test` classpath below includes jars that
                // :deslop-lsp4ij:prepareTestSandbox stages into the shared
                // plugins-test sandbox. Consuming another task's outputs
                // without a declared edge fails Gradle's implicit-dependency
                // validation whenever `test` and `integrationTest` are
                // requested in one invocation — exactly what CI runs — and
                // would leave the sandbox absent on a lone `integrationTest`
                // run. dependsOn fixes both.
                dependsOn(tasks.named("prepareTestSandbox"))
                testClassesDirs = integrationTest.output.classesDirs
                // CRUCIAL: IPGP's test runtime defaults plugins on idea.plugins.path to the core
                // (flat test) classloader, which would resolve the plugin's classes off THIS
                // classpath and make the sandbox lib/ vs lib/modules/ layout irrelevant — the
                // exact blind spot that lets the packaging bug slip through. Forcing false gives
                // the installed plugin its own PluginClassLoader that scans its sandbox lib/, so
                // the flatten (or its absence) actually decides whether the extensions resolve.
                systemProperty("idea.use.core.classloader.for.plugin.path", "false")
                // Reuse the `test` task's COMPLETE, IPGP-curated IntelliJ Platform runtime
                // classpath (core lib + the exact bundled product modules — booting the headless
                // app needs e.g. intellij.platform.settings.local, and loading every lib/modules
                // jar instead double-registers module descriptors). Then strip EVERY Deslop
                // production artifact: the two modules' build outputs AND the plugin jars the test
                // task stages into its sandbox (deslop-lsp4ij-<v>.jar, deslop-jetbrains.*.jar). If
                // any stayed on this flat classpath, the installed plugin's classloader would
                // delegate to it and resolve the tool window factory + action even under the
                // broken lib/modules/ layout — masking the very bug this test exists to catch. The
                // installed plugin (loaded from its own sandbox via idea.plugins.path) is the ONLY
                // sanctioned source; the test's own classes are added back explicitly.
                val pluginBuildDirs = listOf(
                    layout.buildDirectory.get().asFile.absolutePath,
                    project(":deslop-shared").layout.buildDirectory.get().asFile.absolutePath,
                )
                classpath = integrationTest.output +
                    tasks.test.get().classpath.filter { file ->
                        !file.name.startsWith("deslop") &&
                            pluginBuildDirs.none { file.absolutePath.startsWith(it) }
                    }
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
