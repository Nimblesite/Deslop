"""Functional twin of ``text_pipeline_class.py``.

Each step is a pure function; composition happens in ``run``. Same
semantics, radically different AST.
"""

from __future__ import annotations

from functools import reduce


def normalise_whitespace(text):
    return " ".join(text.split())


def lowercase(text):
    return text.lower()


def strip_punctuation(text):
    return "".join(ch for ch in text if ch.isalnum() or ch == " ")


def deduplicate_words(text):
    seen = []
    for word in text.split():
        if word not in seen:
            seen.append(word)
    return " ".join(seen)


STEPS = (normalise_whitespace, lowercase, strip_punctuation, deduplicate_words)


def run(text):
    return reduce(lambda value, step: step(value), STEPS, text)
