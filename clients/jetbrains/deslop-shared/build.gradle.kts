// Shared library for the Deslop LSP4IJ plugin. The `.module` sub-plugin puts the
// IntelliJ Platform on the compile classpath but emits no plugin.xml / sandbox /
// buildPlugin tasks. Compiles against IntelliJ IDEA Community 2024.3 — the build
// floor (243) the shipped plugin declares in its since-build — so the bytecode can
// only reference APIs that exist in every Android Studio / IntelliJ Community we
// claim to support. Picking the floor as the compile base, rather than a newer
// base with a lower since-build, is what guarantees no NoSuchMethodError at runtime
// on an older IDE. Use only com.intellij.modules.platform APIs here.
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
        intellijIdeaCommunity("2024.3")
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
