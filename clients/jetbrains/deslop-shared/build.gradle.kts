// Shared library for both Deslop plugin artifacts. The `.module` sub-plugin puts
// the IntelliJ Platform on the compile classpath but emits no plugin.xml / sandbox
// / buildPlugin tasks. Compiles against the unified IntelliJ IDEA base (the IC/IU
// split ended in 2025.3). Nothing here may use an Ultimate-only API (e.g.
// com.intellij.platform.lsp.api.*) — only com.intellij.modules.platform APIs — so
// the same classes load in Android Studio / Community AND Ultimate/Rider. The
// Ultimate-only LSP client lives solely in :deslop-ultimate; CI Plugin Verifier
// against Android Studio guards the rule.
plugins {
    kotlin("jvm")
    id("org.jetbrains.intellij.platform.module")
}

group = "com.nimblesite"
// Lock-step with the Rust binaries / VSIX: the release passes -PdeslopVersion=<tag>.
version = providers.gradleProperty("deslopVersion").getOrElse("0.1.0")

dependencies {
    api("org.jetbrains.kotlinx:kotlinx-serialization-json:1.9.0")
    testImplementation(kotlin("test-junit5"))

    intellijPlatform {
        intellijIdea("2026.1")
    }
}

kotlin {
    jvmToolchain(17)
}

tasks.test {
    useJUnitPlatform()
    // Repository root, so tests can find shipwright.json and the sibling LSP4IJ
    // plugin.xml regardless of which module dir Gradle runs them from.
    // rootProject is clients/jetbrains → parentFile clients → parentFile repo root.
    systemProperty("deslop.repoRoot", rootProject.projectDir.parentFile.parentFile.absolutePath)
    // The real-binary contract test (DeslopRealBinaryContractTest) is opt-in via
    // this property. CI sets DESLOP_LSP_REAL_BINARY to the built release binary;
    // locally `make _jetbrains-real-binary-test` runs it.
    System.getenv("DESLOP_LSP_REAL_BINARY")?.let { systemProperty("deslop.lsp.realBinary", it) }
}
