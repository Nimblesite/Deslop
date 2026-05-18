package com.nimblesite.deslop.jetbrains

import com.intellij.openapi.components.PersistentStateComponent
import com.intellij.openapi.components.Service
import com.intellij.openapi.components.State
import com.intellij.openapi.components.Storage
import java.net.URI

internal data class DeslopSettingsState(
    var minNodes: Int = DeslopSettingsDefaults.minNodes,
    var embeddingProvider: String = DeslopSettingsDefaults.embeddingProvider,
    var embeddingModel: String = DeslopSettingsDefaults.embeddingModel,
    var embeddingEndpoint: String = DeslopSettingsDefaults.embeddingEndpoint,
    var embeddingMode: String = DeslopSettingsDefaults.embeddingMode,
    var incremental: Boolean = DeslopSettingsDefaults.incremental,
)

internal data class DeslopLaunchSettings(
    val minNodes: Int,
    val embeddingProvider: String,
    val embeddingModel: String,
    val embeddingEndpoint: String,
    val embeddingMode: String,
    val incremental: Boolean,
)

internal object DeslopSettingsDefaults {
    const val minNodes = 30
    const val embeddingProvider = "ollama"
    const val embeddingModel = "nomic-embed-text"
    const val embeddingEndpoint = "http://127.0.0.1:11434"
    const val embeddingMode = "off"
    const val incremental = true
}

internal class DeslopSettingsException(message: String) : RuntimeException(message)

internal object DeslopSettingsValidator {
    // [REMOVE-STUB] Production allows only the Ollama provider.
    private val providers = setOf("ollama")
    private val modes = setOf("off", "auto", "required")

    fun snapshot(state: DeslopSettingsState): DeslopLaunchSettings {
        val errors = validate(state)
        if (errors.isNotEmpty()) throw DeslopSettingsException(errors.joinToString("; "))
        return DeslopLaunchSettings(
            minNodes = state.minNodes,
            embeddingProvider = state.embeddingProvider,
            embeddingModel = state.embeddingModel,
            embeddingEndpoint = state.embeddingEndpoint,
            embeddingMode = state.embeddingMode,
            incremental = state.incremental,
        )
    }

    fun validate(state: DeslopSettingsState): List<String> {
        val errors = mutableListOf<String>()
        if (state.minNodes < 1) errors += "deslop.minNodes must be at least 1."
        if (state.embeddingProvider !in providers) {
            errors += "deslop.embedding.provider must be one of ${providers.sorted()}."
        }
        if (state.embeddingModel.isBlank()) {
            errors += "deslop.embedding.model must not be blank."
        }
        if (state.embeddingMode !in modes) {
            errors += "deslop.embedding.mode must be one of ${modes.sorted()}."
        }
        if (!validEndpoint(state.embeddingEndpoint)) {
            errors += "deslop.embedding.endpoint must be an http(s) URL with a host."
        }
        return errors
    }

    private fun validEndpoint(value: String): Boolean {
        val uri = runCatching { URI(value) }.getOrNull() ?: return false
        return uri.scheme in setOf("http", "https") && !uri.host.isNullOrBlank()
    }
}

@Service(Service.Level.PROJECT)
@State(name = "DeslopSettings", storages = [Storage("deslop.xml")])
internal class DeslopSettings : PersistentStateComponent<DeslopSettingsState> {
    private var settings = DeslopSettingsState()

    override fun getState(): DeslopSettingsState = settings

    override fun loadState(state: DeslopSettingsState) {
        settings = state
    }

    fun launchSettings(): DeslopLaunchSettings {
        return DeslopSettingsValidator.snapshot(settings)
    }
}
