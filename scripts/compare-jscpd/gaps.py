#!/usr/bin/env python3
"""Finds jscpd duplicate pairs that deslop's report corroborates, grades
lower, hides, or misses entirely.

A jscpd pair is matched when one deslop cluster has two distinct occurrences
overlapping the pair's two line ranges (score.matches). Unlike score's
deslop_groups, structural_only clusters and hidden occurrences count as
matches here, because the question is what the report contains at all, not
what the corpus gate scores. Status per pair, in priority order:

  deslop_identical        an identical cluster matches via visible occurrences
  deslop_different_kind   a nearly_identical/loosely_similar/same_behavior cluster matches visibly
  deslop_structural_only  only a structural_only cluster matches visibly
  deslop_hidden           clusters match, but only through hidden occurrences
  deslop_missed           no cluster in the report matches the pair at all

Each side also records `analysed`: whether deslop measured that file at all
(it has a per_file row), so a miss on a file deslop never read is told apart
from a miss on one it did.

usage: gaps.py [--results DIR] [--out FILE]
"""

import argparse
import json
import os
from collections import Counter, defaultdict

import score

HERE = os.path.dirname(os.path.abspath(__file__))
GRADED = {"nearly_identical", "loosely_similar", "same_behavior"}
PREDICATE = ("A jscpd pair counts as seen by deslop when one cluster in its report has "
             "two distinct occurrences overlapping both of the pair's line ranges.")


def read_repos():
    path = os.path.join(HERE, "repos.tsv")
    return [line.split("\t")[0].strip() for line in open(path)
            if line.strip() and not line.startswith("#")]


def occurrence(o):
    return (score.clean(o["path"]), o["start_line"], o["end_line"], bool(o.get("hidden")))


def range_of(dup, side):
    file = dup[side]
    return (score.clean(file["name"]), file["start"], file["end"])


def loc(side, analysed):
    path = score.clean(side["name"])
    return {"path": path, "start": side["start"], "end": side["end"],
            "analysed": path in analysed}


def cluster_match(cluster, ranges):
    files = {r[0] for r in ranges}
    occ = [o for o in map(occurrence, cluster["occurrences"]) if o[0] in files]
    visible = [o for o in occ if not o[3]]
    if score.matches(visible, ranges):
        hidden = False
    elif occ and score.matches(occ, ranges):
        hidden = True
    else:
        return None
    return {"cluster_id": cluster.get("id"), "kind": cluster["kind"],
            "rank": cluster.get("rank"), "hidden": hidden}


def match_clusters(dup, clusters):
    ranges = [range_of(dup, side) for side in ("firstFile", "secondFile")]
    hits = (cluster_match(c, ranges) for c in clusters)
    return sorted((h for h in hits if h), key=lambda m: (m["rank"] is None, m["rank"]))


def status_of(matches):
    visible = [m for m in matches if not m["hidden"]]
    if any(m["kind"] == "identical" for m in visible):
        return "deslop_identical"
    if any(m["kind"] in GRADED for m in visible):
        return "deslop_different_kind"
    if any(m["kind"] == "structural_only" for m in visible):
        return "deslop_structural_only"
    if visible:
        raise ValueError(f"unclassified cluster kind in {visible}")
    return "deslop_hidden" if matches else "deslop_missed"


def pair_entry(repo, dup, matches, analysed):
    return {"repo": repo, "format": dup["format"], "lines": dup["lines"],
            "first": loc(dup["firstFile"], analysed),
            "second": loc(dup["secondFile"], analysed),
            "status": status_of(matches), "deslop_matches": matches,
            "fragment_head": dup.get("fragment", "")[:200]}


def repo_pairs(repo, results):
    deslop = os.path.join(results, f"{repo}.deslop.json")
    jscpd = os.path.join(results, f"{repo}.jscpd", "jscpd-report.json")
    if not (os.path.exists(deslop) and os.path.exists(jscpd)):
        return None
    report = json.load(open(deslop))
    clusters = report.get("clusters", [])
    analysed = {score.clean(row["path"]) for row in report["metrics"]["per_file"]}
    dups = json.load(open(jscpd)).get("duplicates", [])
    return [pair_entry(repo, dup, match_clusters(dup, clusters), analysed) for dup in dups]


def format_counts(pairs):
    counts = defaultdict(Counter)
    for pair in pairs:
        counts[pair["status"]][pair["format"]] += 1
    return {status: dict(formats) for status, formats in counts.items()}


def build(repos, pairs):
    return {"generated_by": "gaps.py", "predicate": PREDICATE, "repos": repos,
            "summary": {"jscpd_pairs": len(pairs),
                        "by_status": dict(Counter(p["status"] for p in pairs)),
                        "missed_with_an_unanalysed_file": sum(
                            1 for p in pairs if p["status"] == "deslop_missed"
                            and not (p["first"]["analysed"] and p["second"]["analysed"])),
                        "by_status_and_format": format_counts(pairs)},
            "pairs": pairs}


def pair_sort_key(pair):
    return (pair["repo"], pair["status"], pair["first"]["path"],
            pair["second"]["path"], pair["first"]["start"], pair["second"]["start"])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--results", default=score.RESULTS)
    parser.add_argument("--out", default=os.path.join(score.REPO, "target", "compare-jscpd", "gaps.json"))
    args = parser.parse_args()
    repos, pairs = [], []
    for repo in read_repos():
        found = repo_pairs(repo, args.results)
        if found is not None:
            repos.append(repo)
            pairs += found
    pairs.sort(key=pair_sort_key)
    with open(args.out, "w") as handle:
        json.dump(build(repos, pairs), handle, indent=1)
        handle.write("\n")
    summary = build(repos, pairs)["summary"]
    print(f"wrote {args.out}: {summary['jscpd_pairs']} pairs from {len(repos)} repos")
    for status, count in sorted(summary["by_status"].items()):
        print(f"  {status}: {count}")


if __name__ == "__main__":
    main()
