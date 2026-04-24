"""Memoised twin of ``recurrences_recursive.py``.

Uses ``functools.lru_cache`` so the recursive shape is preserved but
each subproblem runs once. Semantically equivalent to the bare
recursion and the iteration variants — same behavior, different code
[Type-4] across all three.
"""

from __future__ import annotations

from functools import lru_cache


@lru_cache(maxsize=None)
def factorial(n):
    if n <= 1:
        return 1
    return n * factorial(n - 1)


@lru_cache(maxsize=None)
def fibonacci(n):
    if n < 2:
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)


@lru_cache(maxsize=None)
def binomial(n, k):
    if k == 0 or k == n:
        return 1
    if k < 0 or k > n:
        return 0
    return binomial(n - 1, k - 1) + binomial(n - 1, k)
