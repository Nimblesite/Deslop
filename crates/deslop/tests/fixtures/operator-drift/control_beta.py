def render_ledger(rows, sink):
    for row in rows:
        sink.write(row.label)
        sink.write(row.amount)
    sink.flush()
    return sink
