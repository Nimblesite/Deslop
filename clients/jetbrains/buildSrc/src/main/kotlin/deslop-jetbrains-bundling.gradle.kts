import com.nimblesite.deslop.jetbrains.gradle.CopyLspArtifactsToSandbox
import com.nimblesite.deslop.jetbrains.gradle.hostPlatform
import java.io.File
import java.nio.file.Files
import java.nio.file.StandardCopyOption
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

// [DEPLOY-JETBRAINS-PACKAGE] Move every content-module jar (IPGP 2.14 stages the
// :deslop-shared project dependency under lib/modules/) up onto the main plugin classpath
// (lib/) and drop the now-empty modules dir. The tool window factory and Tools action are
// declared in the MAIN plugin.xml, so their classes must resolve from the main plugin
// classloader — a top-level lib/*.jar. A lib/modules/*.jar sits behind a child classloader
// the parent cannot see, and this flat plugin declares no <content>, so those classes never
// load and both extensions silently vanish (no Deslop tool window, no "Deslop: Open HTML
// Report" action). LSP still works because its factory is in the top-level jar.
fun flattenContentModuleJars(libDir: File) {
    val modulesDir = libDir.resolve("modules")
    modulesDir.listFiles { file -> file.isFile && file.extension == "jar" }?.forEach { jar ->
        Files.move(jar.toPath(), libDir.resolve(jar.name).toPath(), StandardCopyOption.REPLACE_EXISTING)
    }
    modulesDir.delete()
}

// Apply the flatten to EVERY PrepareSandboxTask — the shipped zip (buildPlugin), runIde,
// and the intellijPlatformTesting testIde sandbox — so this single fix governs every
// sandbox layout. That is exactly what lets the deslop-lsp4ij integrationTest prove the
// contract at the IDE level: comment this block out and that test fails (the tool window
// and action never register), which is the regression the fix prevents.
tasks.withType<PrepareSandboxTask>().configureEach {
    val pluginDir = pluginDirectory
    doLast {
        flattenContentModuleJars(pluginDir.get().asFile.resolve("lib"))
    }
}

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
