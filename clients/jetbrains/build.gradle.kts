// Aggregator for the Deslop JetBrains plugins. Two shippable artifacts —
// :deslop-ultimate (native IntelliJ LSP API, Ultimate/Rider) and :deslop-lsp4ij
// (LSP4IJ client, Android Studio / IntelliJ Community) — share every non-surface
// Kotlin file through :deslop-shared, so the duplicate detector's own plugins
// carry zero duplicated logic. IntelliJ Platform plugin versions are governed by
// the settings plugin in settings.gradle.kts; the Kotlin version is pinned here.
plugins {
    kotlin("jvm") version "2.3.20" apply false
}
