#!/usr/bin/env python3
"""Scores deslop and jscpd reports against Deslop's corpus clone registers.

One predicate for both tools, the corpus gate's own ([CORPUS-SCORE]): a judged
entry is *matched* when a single reported group shows distinct occurrences
overlapping every judged range. A matched CLEARLY IN is a true positive, an
unmatched one a false negative; a matched CLEARLY OUT is a false positive.

Deslop groups are its clone clusters (structural_only findings are
informational and never answer a verdict, as in the gate). jscpd groups are
its reported pairs. The register's pinned clone type is NOT required to match
here, so both tools are held to the same, type-blind predicate; deslop's kind
is then compared with the judged type separately, as the taxonomy question.

usage: score.py [--registers DIR] [--results DIR] [--out FILE]
"""

import argparse
import glob
import itertools
import json
import os
from collections import Counter

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))
RESULTS = os.path.join(REPO, "target", "compare-jscpd", "results")
CLEARLY_IN, CLEARLY_OUT = "clearly_in", "clearly_out"
INFORMATIONAL = {"structural_only"}
# What each deslop kind means in research terms (docs/specs/taxonomy.md).
KIND_TYPES = {
    "identical": {"Type I"},
    "nearly_identical": {"Type II", "Type III"},
    "loosely_similar": {"Type III"},
}
KIND_ORDER = ["identical", "nearly_identical", "loosely_similar"]


def parse_range(text):
    path, lines = text.rsplit(":", 1)
    start, end = lines.split("-")
    return path, int(start), int(end)


def clean(path):
    return path[2:] if path.startswith("./") else path


def deslop_groups(report):
    for cluster in report.get("clusters", []):
        if cluster["kind"] in INFORMATIONAL:
            continue
        occ = [(clean(o["path"]), o["start_line"], o["end_line"])
               for o in cluster["occurrences"] if not o.get("hidden")]
        yield cluster["kind"], occ


def jscpd_groups(report):
    for dup in report.get("duplicates", []):
        occ = [(clean(dup[side]["name"]), dup[side]["start"], dup[side]["end"])
               for side in ("firstFile", "secondFile")]
        yield "duplication", occ


def overlaps(occ, rng):
    return occ[0] == rng[0] and occ[1] <= rng[2] and rng[1] <= occ[2]


def matches(occurrences, ranges):
    """Distinct occurrences overlapping every range, or None."""
    for chosen in itertools.permutations(range(len(occurrences)), len(ranges)):
        if all(overlaps(occurrences[i], r) for i, r in zip(chosen, ranges)):
            return [occurrences[i] for i in chosen]
    return None


def coverage(chosen, ranges):
    """Share of the judged lines the matching occurrences show."""
    judged = shown = 0
    for occ, (_, start, end) in zip(chosen, ranges):
        judged += end - start + 1
        shown += max(0, min(end, occ[2]) - max(start, occ[1]) + 1)
    return shown / judged if judged else 0.0


def best_match(groups, ranges):
    best = None
    for kind, occ in groups:
        chosen = matches(occ, ranges)
        if chosen is None:
            continue
        cov = coverage(chosen, ranges)
        if best is None or cov > best[1]:
            best = (kind, cov)
    return best


def score(register, groups):
    groups = list(groups)
    rows = []
    for verdict in (CLEARLY_IN, CLEARLY_OUT):
        for entry in register.get(verdict, []):
            ranges = [parse_range(r) for r in entry["occurrences"]]
            hit = best_match(groups, ranges)
            rows.append({"verdict": verdict, "type": entry.get("type"),
                         "matched": hit is not None,
                         "kind": hit[0] if hit else None,
                         "coverage": hit[1] if hit else None})
    return rows, Counter(kind for kind, _ in groups)


def load_registers(directory):
    return {r["name"]: r for r in (json.load(open(f)) for f in glob.glob(f"{directory}/*.json"))
            if "name" in r and "sha" in r}


def read_time(path):
    values = dict(line.strip().split("=", 1) for line in open(path) if "=" in line)
    return float(values.get("wall_seconds", "nan")), int(values.get("peak_rss_bytes", "0"))


def pct(part, whole):
    return f"{100 * part / whole:.1f}%" if whole else "—"


def summarise(rows):
    ins = [r for r in rows if r["verdict"] == CLEARLY_IN]
    outs = [r for r in rows if r["verdict"] == CLEARLY_OUT]
    tp = sum(r["matched"] for r in ins)
    fp = sum(r["matched"] for r in outs)
    covs = [r["coverage"] for r in ins if r["matched"]]
    return {"in": len(ins), "tp": tp, "fn": len(ins) - tp, "out": len(outs), "fp": fp,
            "correct": tp + len(outs) - fp, "judged": len(ins) + len(outs),
            "coverage": sum(covs) / len(covs) if covs else None}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--registers", default=os.path.join(REPO, "corpus", "register"))
    parser.add_argument("--results", default=RESULTS)
    parser.add_argument("--out", default=os.path.join(REPO, "docs", "reports", "deslop-vs-jscpd.md"))
    args = parser.parse_args()
    registers = load_registers(args.registers)
    repos = [line.split("\t")[0] for line in open(os.path.join(HERE, "repos.tsv"))
             if line.strip() and not line.startswith("#")]

    tools = {"deslop": (deslop_groups, "{}.deslop.json"),
             "jscpd": (jscpd_groups, "{}.jscpd/jscpd-report.json")}
    per_repo, all_rows, volume = [], {t: [] for t in tools}, {t: Counter() for t in tools}
    for repo in repos:
        line = {"repo": repo}
        for tool, (reader, pattern) in tools.items():
            report_path = os.path.join(args.results, pattern.format(repo))
            if not os.path.exists(report_path):
                line[tool] = None
                continue
            rows, kinds = score(registers[repo], reader(json.load(open(report_path))))
            all_rows[tool] += rows
            volume[tool] += kinds
            wall, rss = read_time(os.path.join(args.results, f"{repo}.{tool}.time"))
            line[tool] = {**summarise(rows), "wall": wall, "rss": rss, "groups": sum(kinds.values())}
        per_repo.append(line)
    write(args.out, per_repo, all_rows, volume, args.results)
    print(f"wrote {args.out}")


def write(out, per_repo, all_rows, volume, results):
    totals = {tool: summarise(rows) for tool, rows in all_rows.items()}
    lines = ["# deslop vs jscpd on Deslop's judged corpus", "",
             "Generated by `scripts/compare-jscpd/score.py` from scans by `run.sh`. "
             f"Builds: {build_line(results)}. "
             "Ground truth: Deslop's clone registers (`corpus/register/`), pairs judged "
             "CLEARLY IN (a real duplicate) or CLEARLY OUT (not one). A pair counts as found "
             "when one reported group shows distinct occurrences overlapping both judged "
             "ranges (any line overlap), applied identically to both tools.", "",
             "## Accuracy against the judgments", "",
             "| tool | judged | correct | score | IN found | false negatives | OUT reported (FP) | judged lines shown |",
             "|---|---|---|---|---|---|---|---|"]
    for tool, t in totals.items():
        lines.append(f"| {tool} | {t['judged']} | {t['correct']} | {pct(t['correct'], t['judged'])} | "
                     f"{t['tp']}/{t['in']} ({pct(t['tp'], t['in'])}) | {t['fn']} | {t['fp']}/{t['out']} | "
                     f"{pct(t['coverage'] or 0, 1)} |")
    lines += ["", "## Per repository", "",
              "| repo | deslop IN found | jscpd IN found | deslop FP | jscpd FP | deslop wall s | jscpd wall s | deslop peak MiB | jscpd peak MiB | deslop groups | jscpd pairs |",
              "|---|---|---|---|---|---|---|---|---|---|---|"]
    for line in per_repo:
        d, j = line.get("deslop"), line.get("jscpd")
        if not d or not j:
            lines.append(f"| {line['repo']} | missing report | | | | | | | | | |")
            continue
        lines.append(
            f"| {line['repo']} | {d['tp']}/{d['in']} | {j['tp']}/{j['in']} | {d['fp']}/{d['out']} | "
            f"{j['fp']}/{j['out']} | {d['wall']:.2f} | {j['wall']:.2f} | {d['rss'] / 2**20:.0f} | "
            f"{j['rss'] / 2**20:.0f} | {d['groups']} | {j['groups']} |")
    done = [l for l in per_repo if l.get("deslop") and l.get("jscpd")]
    for tool in ("deslop", "jscpd"):
        wall = sum(l[tool]["wall"] for l in done)
        peak = max((l[tool]["rss"] for l in done), default=0)
        lines.append(f"\n{tool}: total wall {wall:.1f} s, highest peak memory {peak / 2**20:.0f} MiB.")
    lines += taxonomy(all_rows, volume)
    lines += jscpd_only(results)
    lines += ["", "## Caveat: whose candidates were judged", "",
              "The registers were built by judging candidate pairs drawn from Deslop's own reports "
              "(two Deslop versions side by side), so every judged pair is one some Deslop build "
              "proposed. Recall here measures how many of those real duplicates each tool finds; "
              "pairs only jscpd reports were never judged, so neither tool's precision outside the "
              "register is measured, and the 8 CLEARLY OUT entries are too few to rank precision. "
              "A fair precision comparison needs a judging pass over a sample of jscpd's own pairs."]
    with open(out, "w") as handle:
        handle.write("\n".join(lines) + "\n")


def build_line(results):
    path = os.path.join(results, "builds.txt")
    return open(path).read().strip().replace("\n", "; ") if os.path.exists(path) else "not recorded"


def jscpd_only(results):
    import gaps
    pairs = [p for repo in gaps.read_repos() for p in (gaps.repo_pairs(repo, results) or [])]
    unseen = sum(1 for p in pairs if p["status"] == "deslop_missed")
    return ["", "## Pairs only jscpd reports", "",
            f"jscpd reported {len(pairs)} pairs. Deslop's report shows no cluster overlapping both "
            f"ranges for {unseen} of them ({pct(unseen, len(pairs))}). None of these pairs has been "
            "judged, so they are neither counted as Deslop misses nor as jscpd false positives."]


def taxonomy(all_rows, volume):
    lines = ["", "## Taxonomy", "",
             "jscpd reports one category, a duplicated token run, with no grade of similarity. "
             "Deslop grades each finding: Identical (Type I), Nearly identical (Type II and close "
             "Type III), Similar (Type III with larger edits), plus shape-only findings it lists "
             "as information and never counts.", "",
             "Reported groups by category:", ""]
    for tool, kinds in volume.items():
        lines.append(f"- {tool}: " + ", ".join(f"{k} {v}" for k, v in kinds.most_common()))
    lines += ["", "Of the found CLEARLY IN pairs the judge pinned a clone type on, how often "
              "deslop's category means that type:", "",
              "| judged type | found | category agrees | identical | nearly_identical | loosely_similar |",
              "|---|---|---|---|---|---|"]
    rows = [r for r in all_rows["deslop"] if r["verdict"] == CLEARLY_IN and r["type"] and r["matched"]]
    for judged in ("Type I", "Type II", "Type III"):
        typed = [r for r in rows if r["type"] == judged]
        agree = sum(judged in KIND_TYPES.get(r["kind"], set()) for r in typed)
        by_kind = Counter(r["kind"] for r in typed)
        lines.append(f"| {judged} | {len(typed)} | {agree} ({pct(agree, len(typed))}) | " +
                     " | ".join(str(by_kind[k]) for k in KIND_ORDER) + " |")
    lines += ["", "Found CLEARLY IN pairs by judged type, for both tools:", "",
              "| judged type | judged | deslop found | jscpd found |", "|---|---|---|---|"]
    for judged in ("Type I", "Type II", "Type III", None):
        rows_by = {t: [r for r in all_rows[t] if r["verdict"] == CLEARLY_IN and r["type"] == judged]
                   for t in all_rows}
        n = len(rows_by["deslop"])
        cells = [f"{sum(r['matched'] for r in rows_by[t])} ({pct(sum(r['matched'] for r in rows_by[t]), n)})"
                 for t in ("deslop", "jscpd")]
        lines.append(f"| {judged or 'untyped'} | {n} | {cells[0]} | {cells[1]} |")
    return lines


if __name__ == "__main__":
    main()
