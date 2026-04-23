"""Class-based text-transformation pipeline.

Each step is a method on a stateful object. Paired with the functional
and decorator-driven variants — same behavior, different code [Type-4]
cluster with respect to each.
"""

from __future__ import annotations


class TextPipeline:
    def __init__(self, text):
        self.text = text

    def normalise_whitespace(self):
        parts = self.text.split()
        self.text = " ".join(parts)
        return self

    def lowercase(self):
        self.text = self.text.lower()
        return self

    def strip_punctuation(self):
        keep = []
        for ch in self.text:
            if ch.isalnum() or ch == " ":
                keep.append(ch)
        self.text = "".join(keep)
        return self

    def deduplicate_words(self):
        seen = []
        for word in self.text.split():
            if word not in seen:
                seen.append(word)
        self.text = " ".join(seen)
        return self

    def finish(self):
        return self.text


def run(text):
    return (
        TextPipeline(text)
        .normalise_whitespace()
        .lowercase()
        .strip_punctuation()
        .deduplicate_words()
        .finish()
    )
