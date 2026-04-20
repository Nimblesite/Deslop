"""Generator / ``functools.reduce`` twin of ``pipeline_loop.py``.

The functional-programming flavour: lazy generators plus reduce. Same
semantics as the loop and comprehension variants, radically different
AST and tokens.
"""

from __future__ import annotations

from functools import reduce
from heapq import nlargest


def filter_positive(transactions):
    return list(tx for tx in transactions if tx["amount"] > 0)


def total_amount(transactions):
    return reduce(lambda acc, tx: acc + tx["amount"], transactions, 0.0)


def average_amount(transactions):
    count = reduce(lambda acc, _: acc + 1, transactions, 0)
    if count == 0:
        return 0.0
    total = reduce(lambda acc, tx: acc + tx["amount"], transactions, 0.0)
    return total / count


def totals_by_category(transactions):
    def merge(acc, tx):
        acc[tx["category"]] = acc.get(tx["category"], 0.0) + tx["amount"]
        return acc

    return reduce(merge, transactions, {})


def top_n_by_amount(transactions, count):
    return nlargest(count, transactions, key=lambda tx: tx["amount"])
