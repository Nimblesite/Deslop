"""Imperative data-processing pipeline.

Walks a list of transactions in a plain ``for`` loop. Paired with
``pipeline_comprehension.py`` (same semantics via list comprehensions
+ ``sum``) and ``pipeline_generator.py`` (same semantics via generator
expressions and ``functools.reduce``). Structural and token signals
miss the same behavior, different code [Type-4] equivalence; only the
embedding pass surfaces it.
"""

from __future__ import annotations


def filter_positive(transactions):
    out = []
    for transaction in transactions:
        if transaction["amount"] > 0:
            out.append(transaction)
    return out


def total_amount(transactions):
    running = 0.0
    for transaction in transactions:
        running = running + transaction["amount"]
    return running


def average_amount(transactions):
    if len(transactions) == 0:
        return 0.0
    running = 0.0
    for transaction in transactions:
        running = running + transaction["amount"]
    return running / len(transactions)


def totals_by_category(transactions):
    buckets = {}
    for transaction in transactions:
        key = transaction["category"]
        if key in buckets:
            buckets[key] = buckets[key] + transaction["amount"]
        else:
            buckets[key] = transaction["amount"]
    return buckets


def top_n_by_amount(transactions, count):
    sorted_transactions = []
    for transaction in transactions:
        inserted = False
        for index in range(len(sorted_transactions)):
            if transaction["amount"] > sorted_transactions[index]["amount"]:
                sorted_transactions.insert(index, transaction)
                inserted = True
                break
        if not inserted:
            sorted_transactions.append(transaction)
    return sorted_transactions[:count]
