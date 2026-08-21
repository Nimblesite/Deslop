package com.nimblesite.deslop.jetbrains

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.doubleOrNull
import kotlinx.serialization.json.intOrNull
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject

/**
 * The three axes a Deslop report's clusters can be grouped by, listed most-important
 * first (clone type). Each axis derives its group value from a cluster's FIRST
 * occurrence, so a cluster always lands in exactly one group per axis.
 */
internal enum class Axis(val displayName: String) {
    /** Groups by the clone `bucket` — the most important axis; identical clones cluster together across languages. */
    CLONE_TYPE("Clone type"),

    /** Groups by the language the engine stamped on the cluster ([PIPELINE-LANG-TRAIT]). */
    LANGUAGE("Language"),

    /** Groups by the parent directory of the first occurrence's path. */
    FOLDER("Folder"),
    ;

    /** The group value [cluster] falls under for this axis, keyed off its first occurrence. */
    internal fun keyOf(cluster: OffenderCluster): String {
        val path = cluster.firstOccurrence.path
        return when (this) {
            CLONE_TYPE -> cloneTypeLabel(cluster.bucket)
            LANGUAGE -> DeslopSupportedFiles.languageName(cluster.language)
            FOLDER -> folderOf(path)
        }
    }
}

/** One code location inside a cluster, parsed from a report `occurrences` entry. */
internal data class OffenderOccurrence(
    val path: String,
    val startLine: Int,
    val endLine: Int,
    val hidden: Boolean,
)

/** A duplicate cluster parsed from a report; always carries at least one occurrence. */
internal data class OffenderCluster(
    val id: String,
    /** The engine's one-based worst-first rank over the whole report ([VSIX-TOP-OFFENDERS-RANK-GLOBAL]). */
    val rank: Int,
    val weight: Double,
    val size: Int,
    /** The engine's display count of this cluster's occurrences (`report::occurrence_count`). */
    val occurrenceCount: Int,
    val bucket: String,
    val category: String?,
    /** The engine's language id for the cluster ([PIPELINE-LANG-TRAIT]); empty on older reports. */
    val language: String,
    val occurrences: List<OffenderOccurrence>,
) {
    /** First occurrence — the one the language/folder axes and navigation key off. */
    val firstOccurrence: OffenderOccurrence get() = occurrences.first()
}

/** A node in the grouped worst-offenders tree; an EDT-free model the UI mirrors into Swing. */
internal sealed class OffenderNode {
    /** The human-readable text shown for this node. */
    abstract val label: String

    /** The child nodes (sub-groups, clusters, or occurrences); empty for a leaf. */
    abstract val children: List<OffenderNode>
}

/** A grouping node for one [axis] value, e.g. `Identical (12)`; children are the next level. */
internal class GroupNode internal constructor(
    val axis: Axis,
    val value: String,
    val clusterCount: Int,
    /**
     * Summed weight of the clusters beneath this group. Never displayed: it exists
     * only to order sibling groups, so the group carrying more duplication sorts
     * first. An ordering key over engine values, not a reported figure.
     */
    val weightTotal: Double,
    override val children: List<OffenderNode>,
) : OffenderNode() {
    override val label: String get() = "$value ($clusterCount)"
}

/** A cluster leaf, e.g. `3 clones · client.dart · w=1234`; expands to its occurrences. */
internal class ClusterNode internal constructor(
    val cluster: OffenderCluster,
    override val children: List<OffenderNode>,
) : OffenderNode() {
    override val label: String
        get() =
            "${cluster.occurrenceCount} clones · ${baseNameOf(cluster.firstOccurrence.path)} · " +
                "w=${cluster.weight.toLong()}"
}

/** An occurrence leaf, e.g. `lib/api/client.dart:10`. */
internal class OccurrenceNode internal constructor(val occurrence: OffenderOccurrence) : OffenderNode() {
    override val children: List<OffenderNode> get() = emptyList()
    override val label: String get() = "${occurrence.path}:${occurrence.startLine}"
}

/**
 * Pure, EDT-free grouping seam: parses a Deslop report JSON string and groups its
 * clusters through the enabled axes in the configured order of precedence. Separated
 * from the Swing panel so the grouped structure is unit-testable without an IDE.
 */
internal object DeslopOffenderGrouping {
    /**
     * Parses [reportJson]'s clusters. Throws on malformed JSON (a truncated response,
     * client/LSP version skew), so a caching caller can keep its last-good clusters
     * rather than clear or crash. Separated from [group] so re-grouping never re-parses.
     */
    fun parse(reportJson: String): List<OffenderCluster> = parseClusters(reportJson)

    /**
     * Groups already-parsed [clusters] through [axes] in order. Pure and total — never
     * throws — so a toolbar-driven re-group can safely run on the EDT. Empty [axes]
     * yields a flat worst-first list of cluster leaves; otherwise group nodes (worst
     * summed-weight first) whose children are the next axis's groups or, at the last
     * axis, the cluster leaves (worst-first).
     */
    fun group(clusters: List<OffenderCluster>, axes: List<Axis>): List<OffenderNode> =
        buildLevel(clusters, axes)

    /** Parses then groups — the one-shot path for non-caching callers and tests. */
    fun build(reportJson: String, axes: List<Axis>): List<OffenderNode> =
        group(parse(reportJson), axes)

    private fun buildLevel(clusters: List<OffenderCluster>, axes: List<Axis>): List<OffenderNode> {
        val axis = axes.firstOrNull() ?: return clusters.worstFirst().map(::clusterNode)
        val remaining = axes.drop(1)
        return clusters.groupBy(axis::keyOf)
            .map { (value, members) -> groupNode(axis, value, members, remaining) }
            .sortedByDescending(GroupNode::weightTotal)
    }

    private fun groupNode(axis: Axis, value: String, members: List<OffenderCluster>, remaining: List<Axis>) =
        GroupNode(axis, value, members.size, members.sumOf(OffenderCluster::weight), buildLevel(members, remaining))

    private fun clusterNode(cluster: OffenderCluster): ClusterNode =
        ClusterNode(cluster, cluster.occurrences.map(::OccurrenceNode))
}

/**
 * Clusters in the engine's own worst-first order ([VSIX-TOP-OFFENDERS-RANK-GLOBAL]).
 * Sorting by the stamped rank reproduces the report's ranking exactly, tie-break
 * included; re-deriving it from `weight` here would be a second ranking engine.
 */
private fun List<OffenderCluster>.worstFirst(): List<OffenderCluster> =
    sortedBy(OffenderCluster::rank)

/** Parses every well-formed cluster from a report JSON string, ignoring unknown keys. */
private fun parseClusters(reportJson: String): List<OffenderCluster> =
    Json.parseToJsonElement(reportJson).jsonObject["clusters"]
        ?.jsonArray.orEmpty()
        .mapNotNull { parseCluster(it.jsonObject) }

/** Parses a single cluster; returns null when it carries no usable occurrence. */
private fun parseCluster(node: JsonObject): OffenderCluster? {
    val occurrences = node["occurrences"]?.jsonArray.orEmpty().mapNotNull { parseOccurrence(it.jsonObject) }
    if (occurrences.isEmpty()) return null
    val size = node.intOr("size", occurrences.size)
    return OffenderCluster(
        id = node.stringOr("id", ""),
        rank = node.intOr("rank", 0),
        weight = node.doubleOr("weight", 0.0),
        size = size,
        occurrenceCount = node.intOr("occurrence_count", size),
        bucket = node.stringOr("bucket", ""),
        category = node.stringOrNull("category"),
        language = node.stringOr("language", ""),
        occurrences = occurrences,
    )
}

/** Parses a single occurrence; returns null when it has no path to group or navigate to. */
private fun parseOccurrence(node: JsonObject): OffenderOccurrence? {
    val path = node.stringOrNull("path") ?: return null
    return OffenderOccurrence(
        path = path,
        startLine = node.intOr("start_line", 0),
        endLine = node.intOr("end_line", 0),
        hidden = node.boolOr("hidden", false),
    )
}

/** The human clone-type label for a raw `bucket`, falling back to the raw value. */
private fun cloneTypeLabel(bucket: String): String = when (bucket) {
    "identical" -> "Identical"
    "nearly_identical" -> "Nearly identical"
    "structural_only" -> "Structural only"
    "loosely_similar" -> "Loosely similar"
    "same_behavior" -> "Same behavior"
    else -> bucket
}

/** The parent directory of [path] (slash-normalised), or `(root)` when there is none. */
private fun folderOf(path: String): String {
    val normalized = path.replace('\\', '/')
    val separator = normalized.lastIndexOf('/')
    if (separator < 0) return ROOT_FOLDER
    return normalized.substring(0, separator).ifEmpty { ROOT_FOLDER }
}

/** The final path segment of [path] (its file name), slash-normalised. */
private fun baseNameOf(path: String): String = path.replace('\\', '/').substringAfterLast('/')

/** Folder label shown when a first occurrence has no parent directory. */
private const val ROOT_FOLDER = "(root)"

/** The string value at [key], or null when absent or not a string primitive. */
private fun JsonObject.stringOrNull(key: String): String? = (this[key] as? JsonPrimitive)?.contentOrNull

/** The string value at [key], or [fallback] when absent. */
private fun JsonObject.stringOr(key: String, fallback: String): String = stringOrNull(key) ?: fallback

/** The number at [key] read as a Double, or [fallback] when absent or non-numeric. */
private fun JsonObject.doubleOr(key: String, fallback: Double): Double =
    (this[key] as? JsonPrimitive)?.doubleOrNull ?: fallback

/** The number at [key] read as an Int, or [fallback] when absent or non-integral. */
private fun JsonObject.intOr(key: String, fallback: Int): Int = (this[key] as? JsonPrimitive)?.intOrNull ?: fallback

/** The boolean at [key], or [fallback] when absent or not a boolean primitive. */
private fun JsonObject.boolOr(key: String, fallback: Boolean): Boolean =
    (this[key] as? JsonPrimitive)?.booleanOrNull ?: fallback
