"""Classic recurrences expressed recursively.

Paired with ``recurrences_iterative.py`` and
``recurrences_memoised.py`` — same three functions, three idioms,
three Type-4 clusters.
"""

from __future__ import annotations


def factorial(n):
    if n <= 1:
        return 1
    return n * factorial(n - 1)


def fibonacci(n):
    if n < 2:
        return n
    return fibonacci(n - 1) + fibonacci(n - 2)


def binomial(n, k):
    if k == 0 or k == n:
        return 1
    if k < 0 or k > n:
        return 0
    return binomial(n - 1, k - 1) + binomial(n - 1, k)
