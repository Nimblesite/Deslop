"""Shared rule-identifier index over documents and source ASTs ([SPEC-ID-GATE]).

`spec-crossrefs.py` audits a named list of identifiers; `spec-id-gate.py`
refuses any identifier the documents do not define. Both read the same two
indexes, so a rule that resolves for one resolves for the other.

Source identifiers come from parsed comment and string nodes only — never from
a text scan of code. Document identifiers come from headings and bold
requirement bullets, which are prose, not code.
"""

from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path

import tree_sitter
import tree_sitter_rust
import tree_sitter_typescript

SOURCE_ROOTS = ("crates", "clients/vscode/src")
DOC_ROOT = "docs"
DOC_SUFFIXES = ("*.md", "*.td")
COMMENT_TYPES = frozenset(("line_comment", "block_comment", "comment"))
STRING_TYPES = frozenset(("string_literal", "string"))
TEST_DIR_NAMES = frozenset(("tests", "test", "fixtures", "__tests__"))
HEADING_PREFIX = "#"
FENCE = "```"
# List markers, bold markers and ordinals may precede a requirement's
# identifier; nothing else may.
LEAD_IN_PUNCTUATION = "#*-. \t0123456789"


@dataclass(frozen=True, order=True)
class Citation:
    """A comment or literal selected by its parsed node, with a stable location."""

    path: str
    line: int
    is_test: bool = False

    def link(self) -> str:
        """Render a repository-relative markdown link."""
        return f"[{self.path}:{self.line}](../{self.path}#L{self.line})"


def identifiers(text: str) -> set[str]:
    """Read bracket-delimited rule slugs without treating code as text patterns.

    A slug is two or more upper-case alphanumeric parts joined by hyphens, at
    least one of which carries a letter — so a character class such as `[0-9]`
    inside a comment is not mistaken for a rule.
    """
    result = set()
    for fragment in text.split("[")[1:]:
        candidate, closing, _rest = fragment.partition("]")
        parts = candidate.split("-")
        if not closing or len(parts) < 2:
            continue
        if all(part and part.isalnum() and part.upper() == part for part in parts) and any(
            any(character.isalpha() for character in part) for part in parts
        ):
            result.add(candidate)
    return result


def doc_definitions() -> dict[str, list[Citation]]:
    """Index requirement headings and bold requirement bullets, not mentions.

    A heading defines every identifier it carries, wherever in the heading it
    sits: `## [RANK-MASS-SUM] Mass` and `## Fix 4 — [PAIR-SIZE-COHERENCE]`
    both define their rule. A requirement written as a bold lead-in — as a
    bullet or as its own paragraph — defines its rule the same way. Type-diagram
    models (`.td`) define the wire rules their comments carry.
    """
    result = defaultdict(list)
    for suffix in DOC_SUFFIXES:
        for path in sorted(Path(DOC_ROOT).rglob(suffix)):
            _index_document(path, result)
    return result


def _index_document(path: Path, result: dict[str, list[Citation]]) -> None:
    """Add every identifier `path` defines to `result`."""
    fenced = False
    for line, text in enumerate(path.read_text().splitlines(), start=1):
        stripped = text.lstrip()
        if stripped.startswith(FENCE):
            fenced = not fenced
            continue
        for identifier in _defined_here(path, text, stripped, fenced):
            result[identifier].append(Citation(str(path), line))


def _defined_here(path: Path, text: str, stripped: str, fenced: bool) -> set[str]:
    """The identifiers this line defines, as opposed to merely mentions.

    A heading defines every identifier it carries, wherever it sits in the
    heading. Any other line defines an identifier only when that identifier
    leads it, after list markers, ordinals and bold markers.
    """
    if path.suffix == ".td":
        return identifiers(text) if stripped.startswith(HEADING_PREFIX) else set()
    if fenced:
        return set()
    if text.startswith(HEADING_PREFIX):
        return identifiers(text)
    lead_in, opening, rest = text.partition("[")
    if opening and not lead_in.strip(LEAD_IN_PUNCTUATION):
        return identifiers(f"[{rest.partition(']')[0]}]")
    return set()


def parser_set() -> dict[str, tree_sitter.Parser]:
    """Keep parsers and their languages alive for every traversal."""
    languages = {
        ".rs": tree_sitter.Language(tree_sitter_rust.language()),
        ".ts": tree_sitter.Language(tree_sitter_typescript.language_typescript()),
        ".tsx": tree_sitter.Language(tree_sitter_typescript.language_tsx()),
    }
    return {suffix: tree_sitter.Parser(language) for suffix, language in languages.items()}


def test_context(path: Path, node: tree_sitter.Node) -> bool:
    """Include integration suites and inline Rust test modules."""
    if set(path.parts) & TEST_DIR_NAMES or test_name(path.stem):
        return True
    parent = node.parent
    while parent is not None:
        if parent.type == "mod_item":
            name = parent.child_by_field_name("name")
            if name and test_name(name.text.decode()):
                return True
        parent = parent.parent
    return False


def test_name(name: str) -> bool:
    """Recognize test names without misclassifying names such as latest_report."""
    return name in {"test", "tests"} or name.startswith("test_") or name.endswith(("_tests", "_test", ".test"))


def parsed_citations(path: Path, parser: tree_sitter.Parser):
    """Yield comment/literal references only; never search raw source."""
    data = path.read_bytes()
    tree = parser.parse(data)
    pending = [tree.root_node]
    while pending:
        node = pending.pop()
        if node.type in COMMENT_TYPES | STRING_TYPES:
            citation = Citation(str(path), node.start_point.row + 1, test_context(path, node))
            for identifier in identifiers(node.text.decode()):
                yield identifier, node.type in COMMENT_TYPES, citation
        else:
            pending.extend(reversed(node.children))


def source_citations():
    """Collect current citations independently of git's tracked-file list."""
    comments, literals = defaultdict(set), defaultdict(set)
    parsers = parser_set()
    for root in SOURCE_ROOTS:
        for path in sorted(Path(root).rglob("*")):
            if path.suffix not in parsers:
                continue
            for identifier, is_comment, citation in parsed_citations(path, parsers[path.suffix]):
                (comments if is_comment else literals)[identifier].add(citation)
    return comments, literals
