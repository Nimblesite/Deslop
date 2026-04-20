"""Comprehension / ``sum`` twin of ``pipeline_loop.py``.

Same five functions, same semantics, idiomatic Python.
"""

from __future__ import annotations

from collections import defaultdict


def filter_positive(transactions):
    return [tx for tx in transactions if tx["amount"] > 0]


def total_amount(transactions):
    return sum(tx["amount"] for tx in transactions)


def average_amount(transactions):
    if not transactions:
        return 0.0
    return sum(tx["amount"] for tx in transactions) / len(transactions)


def totals_by_category(transactions):
    buckets: dict[str, float] = defaultdict(float)
    for tx in transactions:
        buckets[tx["category"]] += tx["amount"]
    return dict(buckets)


def top_n_by_amount(transactions, count):
    return sorted(transactions, key=lambda tx: tx["amount"], reverse=True)[:count]
