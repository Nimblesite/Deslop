"""Decorator-driven twin of ``text_pipeline_class.py``.

Registers each step via a decorator; ``run`` walks the registry.
Semantics match the class and functional variants.
"""

from __future__ import annotations


REGISTRY = []


def step(func):
    REGISTRY.append(func)
    return func


@step
def normalise_whitespace(text):
    return " ".join(text.split())


@step
def lowercase(text):
    return text.lower()


@step
def strip_punctuation(text):
    keep = []
    for ch in text:
        if ch.isalnum() or ch == " ":
            keep.append(ch)
    return "".join(keep)


@step
def deduplicate_words(text):
    seen = []
    for word in text.split():
        if word not in seen:
            seen.append(word)
    return " ".join(seen)


def run(text):
    value = text
    for func in REGISTRY:
        value = func(value)
    return value
