package com.nimblesite.deslop.jetbrains

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertTrue

/**
 * A crafted worst-offenders report shared by the grouping and panel tests. It holds
 * two `identical` clusters the engine stamped with different languages (Dart, C#),
 * one `nearly_identical` and one `structural_only` cluster, occurrences spanning
 * several folders, an unknown key to prove tolerance, and a cluster whose
 * `occurrence_count` exceeds its carried occurrence list (the live wire caps that
 * list). Every cluster carries the engine's `rank`, `language` and
 * `occurrence_count` because the panel reads those verbatim and derives none of
 * them. Ranks run worst-first: cccc (1) > aaaa (2) > bbbb (3) > dddd (4).
 */
internal val OFFENDERS_FIXTURE_JSON: String =
    """
    {
      "clusters": [
        {
          "id": "cccc", "rank": 1, "weight": 1200.0, "size": 2, "occurrence_count": 2,
          "bucket": "nearly_identical", "language": "dart",
          "category": "logic", "unknown_field": "ignored",
          "occurrences": [
            { "path": "lib/api/thing.dart", "start_line": 5, "end_line": 9, "hidden": false },
            { "path": "app/thing.dart", "start_line": 1, "end_line": 5, "hidden": false }
          ]
        },
        {
          "id": "aaaa", "rank": 2, "weight": 1000.0, "size": 3, "occurrence_count": 3,
          "bucket": "identical", "language": "dart", "category": "logic",
          "occurrences": [
            { "path": "lib/api/client.dart", "start_line": 10, "end_line": 40, "hidden": false },
            { "path": "lib/api/client2.dart", "start_line": 10, "end_line": 40, "hidden": false }
          ]
        },
        {
          "id": "bbbb", "rank": 3, "weight": 800.0, "size": 2, "occurrence_count": 2,
          "bucket": "identical", "language": "csharp", "category": "data",
          "occurrences": [
            { "path": "src/Service.cs", "start_line": 1, "end_line": 20, "hidden": false },
            { "path": "src/Other.cs", "start_line": 1, "end_line": 20, "hidden": false }
          ]
        },
        {
          "id": "dddd", "rank": 4, "weight": 500.0, "size": 2, "occurrence_count": 2,
          "bucket": "structural_only", "language": "python", "category": "logic",
          "occurrences": [
            { "path": "lib/util/helper.py", "start_line": 3, "end_line": 10, "hidden": false },
            { "path": "tools/helper.py", "start_line": 3, "end_line": 10, "hidden": false }
          ]
        }
      ]
    }
    """.trimIndent()

/**
 * Black-box tests for the pure, EDT-free grouping seam [DeslopOffenderGrouping]. They
 * exercise clone-type-first grouping, axis nesting, folder toggling, precedence
 * reordering, worst-first ranking, and the human-readable node labels — all without
 * constructing Swing.
 */
internal class DeslopOffenderGroupingTest {
    @Test
    fun cloneTypeFirstGroupsIdenticalClustersAcrossLanguages() {
        val roots = DeslopOffenderGrouping.build(OFFENDERS_FIXTURE_JSON, listOf(Axis.CLONE_TYPE))
        assertEquals(
            listOf("Identical (2)", "Nearly identical (1)", "Structural only (1)"),
            roots.map(OffenderNode::label),
            "clone-type groups render worst-first with a per-group cluster count",
        )
        val identical = assertIs<GroupNode>(roots.first())
        val members = identical.children.map { assertIs<ClusterNode>(it).cluster }
        assertEquals(
            listOf("aaaa", "bbbb"),
            members.map(OffenderCluster::id),
            "both identical clusters sit under the one Identical node, worst-first",
        )
        assertEquals(
            listOf("Dart", "C#"),
            members.map { DeslopSupportedFiles.languageName(it.language) },
            "the two identical clusters are in different languages yet share the Identical node",
        )
    }

    @Test
    fun enablingLanguageNestsLanguageNodesUnderCloneType() {
        val roots = DeslopOffenderGrouping.build(OFFENDERS_FIXTURE_JSON, listOf(Axis.CLONE_TYPE, Axis.LANGUAGE))
        val identical = assertIs<GroupNode>(roots.first())
        val languageGroups = identical.children.map { assertIs<GroupNode>(it) }
        assertTrue(
            languageGroups.all { it.axis == Axis.LANGUAGE },
            "the second axis nests language groups beneath the clone-type group",
        )
        assertEquals(
            listOf("Dart (1)", "C# (1)"),
            languageGroups.map(OffenderNode::label),
            "nested language groups render worst-first",
        )
        val dartCluster = assertIs<ClusterNode>(languageGroups.first().children.single())
        assertEquals("aaaa", dartCluster.cluster.id, "the Dart language group holds the Dart identical cluster")
    }

    @Test
    fun togglingFolderChangesStructure() {
        val withoutFolder = DeslopOffenderGrouping.build(OFFENDERS_FIXTURE_JSON, listOf(Axis.CLONE_TYPE))
        val withFolder = DeslopOffenderGrouping.build(OFFENDERS_FIXTURE_JSON, listOf(Axis.CLONE_TYPE, Axis.FOLDER))
        val identicalFlat = assertIs<GroupNode>(withoutFolder.first())
        val identicalFoldered = assertIs<GroupNode>(withFolder.first())
        assertIs<ClusterNode>(
            identicalFlat.children.first(),
            "with folder off the Identical group's children are cluster leaves",
        )
        val folderGroups = identicalFoldered.children.map { assertIs<GroupNode>(it) }
        assertEquals(
            listOf("lib/api (1)", "src (1)"),
            folderGroups.map(OffenderNode::label),
            "enabling folder inserts a folder grouping level, changing the structure",
        )
    }

    @Test
    fun reorderingLanguageOutermostChangesHierarchy() {
        val cloneTypeFirst = DeslopOffenderGrouping.build(OFFENDERS_FIXTURE_JSON, listOf(Axis.CLONE_TYPE, Axis.LANGUAGE))
        val languageFirst = DeslopOffenderGrouping.build(OFFENDERS_FIXTURE_JSON, listOf(Axis.LANGUAGE, Axis.CLONE_TYPE))
        assertEquals("Identical (2)", cloneTypeFirst.first().label, "clone-type outermost keeps Identical at the top level")
        val topLanguages = languageFirst.map { assertIs<GroupNode>(it) }
        assertTrue(topLanguages.all { it.axis == Axis.LANGUAGE }, "reordering makes language the outermost axis")
        assertEquals(
            listOf("Dart (2)", "C# (1)", "Python (1)"),
            topLanguages.map(OffenderNode::label),
            "top-level language groups render worst-first",
        )
        assertEquals(
            listOf("Nearly identical (1)", "Identical (1)"),
            topLanguages.first().children.map(OffenderNode::label),
            "clone type nests under language once language is outermost",
        )
    }

    @Test
    fun worstFirstOrderingOfGroupsAndClusters() {
        val flat = DeslopOffenderGrouping.build(OFFENDERS_FIXTURE_JSON, emptyList())
        assertEquals(
            listOf("cccc", "aaaa", "bbbb", "dddd"),
            flat.map { assertIs<ClusterNode>(it).cluster.id },
            "all axes off yields a flat worst-first list of clusters",
        )
        val cloneTypeGroups = DeslopOffenderGrouping.build(OFFENDERS_FIXTURE_JSON, listOf(Axis.CLONE_TYPE))
        assertEquals(
            listOf("Identical", "Nearly identical", "Structural only"),
            cloneTypeGroups.map { assertIs<GroupNode>(it).value },
            "groups order by their summed weight: Identical 1800 > Nearly identical 1200 > Structural only 500",
        )
    }

    @Test
    fun clusterAndOccurrenceLabelsAreHumanReadable() {
        val flat = DeslopOffenderGrouping.build(OFFENDERS_FIXTURE_JSON, emptyList())
        val clientCluster = flat.map { assertIs<ClusterNode>(it) }.first { it.cluster.id == "aaaa" }
        assertEquals(
            "3 clones · client.dart · w=1000",
            clientCluster.label,
            "a cluster leaf reads as clone count, first-occurrence basename, and rounded weight",
        )
        assertEquals(
            listOf("lib/api/client.dart:10", "lib/api/client2.dart:10"),
            clientCluster.children.map(OffenderNode::label),
            "a cluster expands to its occurrences labelled path:startLine",
        )
    }

    @Test
    fun languageLabelsMapExtensionsToDisplayNamesWithOtherFallback() {
        assertEquals("Rust", DeslopSupportedFiles.languageLabel("rs"))
        assertEquals("JavaScript", DeslopSupportedFiles.languageLabel("mjs"), "every JS extension shares one language label")
        assertEquals("TypeScript", DeslopSupportedFiles.languageLabel("TSX"), "language lookup is case-insensitive")
        assertEquals("Other", DeslopSupportedFiles.languageLabel("kt"), "an unanalysed extension groups under Other")
        assertEquals("Other", DeslopSupportedFiles.languageLabel(null), "a file without an extension groups under Other")
    }

    @Test
    fun languageNamesMapTheEnginesOwnIdsWithOtherFallback() {
        // The grouping axis reads the id the engine stamped, so these are the
        // parser registry's ids ([PIPELINE-LANG-TRAIT]) — not file extensions.
        assertEquals("Rust", DeslopSupportedFiles.languageName("rust"))
        assertEquals("C#", DeslopSupportedFiles.languageName("csharp"))
        assertEquals("JavaScript", DeslopSupportedFiles.languageName("javascript"))
        assertEquals("TypeScript", DeslopSupportedFiles.languageName("typescript"))
        assertEquals("TypeScript", DeslopSupportedFiles.languageName("tsx"), "TSX renders under TypeScript")
        assertEquals("F#", DeslopSupportedFiles.languageName("fsharp"))
        assertEquals("Other", DeslopSupportedFiles.languageName("unknown"), "the engine's own fallback id groups under Other")
        assertEquals("Other", DeslopSupportedFiles.languageName(""), "a report predating the field groups under Other")
        assertEquals("Other", DeslopSupportedFiles.languageName(null))
    }

    @Test
    fun clusterOrderIsTheEnginesRankNotALocalWeightSort() {
        // Deliberately out of weight order in the JSON: the engine ranked the
        // lighter cluster first, and the panel must not re-sort it
        // ([VSIX-TOP-OFFENDERS-RANK-GLOBAL]).
        val json =
            """{"clusters":[
              {"id":"light","rank":1,"weight":10.0,"size":2,"occurrence_count":2,"bucket":"identical",
               "language":"rust","occurrences":[{"path":"a/one.rs","start_line":1,"end_line":2}]},
              {"id":"heavy","rank":2,"weight":900.0,"size":2,"occurrence_count":2,"bucket":"identical",
               "language":"rust","occurrences":[{"path":"a/two.rs","start_line":1,"end_line":2}]}
            ]}""".trimIndent()
        assertEquals(
            listOf("light", "heavy"),
            DeslopOffenderGrouping.build(json, emptyList()).map { assertIs<ClusterNode>(it).cluster.id },
            "the engine's rank decides worst-first order, even against the weights",
        )
    }

    @Test
    fun clusterLabelShowsTheEnginesOccurrenceCount() {
        // The live wire caps `occurrences`, so the count comes off the engine's
        // `occurrence_count` and never off the carried list.
        val json =
            """{"clusters":[{"id":"capped","rank":1,"weight":5.0,"size":2,"occurrence_count":57,
              "bucket":"identical","language":"rust",
              "occurrences":[{"path":"a/one.rs","start_line":1,"end_line":2}]}]}""".trimIndent()
        val node = assertIs<ClusterNode>(DeslopOffenderGrouping.build(json, emptyList()).single())
        assertEquals(1, node.cluster.occurrences.size, "fixture: only one occurrence travelled")
        assertEquals(
            "57 clones · one.rs · w=5",
            node.label,
            "the leaf reports the engine's count, not the length of the truncated list",
        )
    }

    @Test
    fun rootLevelFileGroupsUnderRootFolderAndOtherLanguage() {
        val json =
            """{"clusters":[{"id":"zzzz","weight":10.0,"size":2,"bucket":"loosely_similar",""" +
                """"occurrences":[{"path":"Makefile","start_line":1,"end_line":2}]}]}"""
        assertEquals(
            "(root) (1)",
            DeslopOffenderGrouping.build(json, listOf(Axis.FOLDER)).single().label,
            "a first occurrence with no parent directory groups under (root)",
        )
        assertEquals(
            "Other (1)",
            DeslopOffenderGrouping.build(json, listOf(Axis.LANGUAGE)).single().label,
            "an extension-less path groups under the Other language",
        )
        assertEquals(
            "Loosely similar (1)",
            DeslopOffenderGrouping.build(json, listOf(Axis.CLONE_TYPE)).single().label,
            "loosely_similar renders as its human clone-type label",
        )
    }

    @Test
    fun everyCloneTypeBucketRendersItsHumanLabel() {
        val buckets = mapOf(
            "identical" to "Identical",
            "nearly_identical" to "Nearly identical",
            "structural_only" to "Structural only",
            "loosely_similar" to "Loosely similar",
            "same_behavior" to "Same behavior",
            "mystery_bucket" to "mystery_bucket",
        )
        for ((bucket, expected) in buckets) {
            val json =
                """{"clusters":[{"id":"x","weight":1.0,"size":2,"bucket":"$bucket",""" +
                    """"occurrences":[{"path":"a/b.rs","start_line":1,"end_line":2}]}]}"""
            assertEquals(
                "$expected (1)",
                DeslopOffenderGrouping.build(json, listOf(Axis.CLONE_TYPE)).single().label,
                "bucket '$bucket' renders as '$expected'",
            )
        }
    }
}
