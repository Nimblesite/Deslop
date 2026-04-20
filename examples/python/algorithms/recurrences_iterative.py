"""Iterative twin of ``recurrences_recursive.py``.

Loops + accumulators — same three recurrences, same semantics.
"""

from __future__ import annotations


def factorial(n):
    accumulator = 1
    for index in range(2, n + 1):
        accumulator = accumulator * index
    return accumulator


def fibonacci(n):
    if n < 2:
        return n
    previous, current = 0, 1
    for _ in range(2, n + 1):
        previous, current = current, previous + current
    return current


def binomial(n, k):
    if k < 0 or k > n:
        return 0
    if k == 0 or k == n:
        return 1
    row = [1] * (n + 1)
    for i in range(1, n + 1):
        for j in range(i - 1, 0, -1):
            row[j] = row[j] + row[j - 1]
    return row[k]
