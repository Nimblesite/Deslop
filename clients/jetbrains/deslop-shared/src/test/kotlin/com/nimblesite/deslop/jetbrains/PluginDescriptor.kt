package com.nimblesite.deslop.jetbrains

import java.nio.file.Path
import javax.xml.parsers.DocumentBuilderFactory
import org.w3c.dom.Document

/**
 * Loads and queries the shipped LSP4IJ `plugin.xml` with a real XML parser — never
 * regex over structured data, per the repo rule. Shared by the descriptor and
 * file-pattern tests so the parsing is written once.
 */
internal object PluginDescriptor {
    fun lsp4ij(): Document {
        val repoRoot = System.getProperty("deslop.repoRoot")
            ?: error("deslop.repoRoot system property must be set by the Gradle test task")
        val pluginXml = Path.of(
            repoRoot,
            "clients/jetbrains/deslop-lsp4ij/src/main/resources/META-INF/plugin.xml",
        )
        return DocumentBuilderFactory.newInstance()
            .also { it.isNamespaceAware = false }
            .newDocumentBuilder()
            .parse(pluginXml.toFile())
    }

    /** Every value of [attribute] across elements named [tag], in document order. */
    fun attributeValues(document: Document, tag: String, attribute: String): List<String> {
        val nodes = document.getElementsByTagName(tag)
        return (0 until nodes.length).mapNotNull {
            nodes.item(it).attributes?.getNamedItem(attribute)?.nodeValue
        }
    }
}
