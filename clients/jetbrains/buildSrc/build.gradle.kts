plugins {
    `kotlin-dsl`
}

repositories {
    gradlePluginPortal()
    mavenCentral()
}

dependencies {
    // The IntelliJ Platform Gradle Plugin marker pulls its task types
    // (PrepareSandboxTask, BuildPluginTask) onto the convention plugin's compile
    // classpath so the shared LSP-bundling logic can reference them. Version is
    // locked to the settings plugin in settings.gradle.kts.
    implementation("org.jetbrains.intellij.platform:org.jetbrains.intellij.platform.gradle.plugin:2.14.0")
}
