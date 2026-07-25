// Aggregator for the Deslop JetBrains plugin. One shippable artifact —
// :deslop-lsp4ij (LSP4IJ client, every JetBrains IDE family) — keeps its
// non-surface Kotlin in :deslop-shared, so the duplicate detector's own plugin
// carries zero duplicated logic. IntelliJ Platform plugin versions are governed by
// the settings plugin in settings.gradle.kts; the Kotlin version is pinned here.
plugins {
    kotlin("jvm") version "2.4.10" apply false
}
