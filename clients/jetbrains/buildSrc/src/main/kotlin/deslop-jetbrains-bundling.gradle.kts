import com.nimblesite.deslop.jetbrains.gradle.CopyLspArtifactsToSandbox
import com.nimblesite.deslop.jetbrains.gradle.hostPlatform
import java.io.File
import org.jetbrains.intellij.platform.gradle.tasks.BuildPluginTask
import org.jetbrains.intellij.platform.gradle.tasks.PrepareSandboxTask

// Single source of the LSP-bundling wiring applied by the Deslop plugin artifact
// (:deslop-lsp4ij). It lives in buildSrc rather than inline so the bundling logic
// stays out of the module build script and can be reused if another surface is
// ever added; the plugin module applies id("deslop-jetbrains-bundling").
//
// rootProject is clients/jetbrains regardless of which module applies this, so the
// ../../ climb still reaches the repository root from the aggregator project.
//
// DESLOP_LSP_BUNDLE_DIR (release): a directory laid out as <platform>/deslop-lsp[.exe]
// for every shipped platform — all are staged so the published zip installs offline
// on any OS/arch. Unset (local dev): only the host platform is staged from
// DESLOP_BINARY_DIR or ../../target/release.
val deploymentManifestFile = rootProject.layout.projectDirectory
    .file("../../shipwright.json")
    .asFile
val bundleDir = System.getenv("DESLOP_LSP_BUNDLE_DIR")

val prepareSandbox = tasks.named<PrepareSandboxTask>("prepareSandbox")
val copyLspArtifactsToSandbox = tasks.register<CopyLspArtifactsToSandbox>("copyLspArtifactsToSandbox") {
    dependsOn(prepareSandbox)
    deploymentManifest.set(deploymentManifestFile)
    pluginDirectory.set(prepareSandbox.flatMap { it.pluginDirectory })
    if (bundleDir != null) {
        bundleSource.set(File(bundleDir))
    } else {
        val hostPlatformName = hostPlatform()
        val lspBinaryName = if (hostPlatformName.startsWith("win32")) "deslop-lsp.exe" else "deslop-lsp"
        val binaryDirectory = System.getenv("DESLOP_BINARY_DIR")?.let(::File)
            ?: rootProject.layout.projectDirectory.dir("../../target/release").asFile
        hostBinary.set(binaryDirectory.resolve(lspBinaryName))
        hostPlatform.set(hostPlatformName)
    }
}

tasks.named<BuildPluginTask>("buildPlugin") {
    dependsOn(copyLspArtifactsToSandbox)
    eachFile {
        if (!isDirectory && path.contains("/bin/")) {
            permissions { unix("rwxr-xr-x") }
        }
    }
}
