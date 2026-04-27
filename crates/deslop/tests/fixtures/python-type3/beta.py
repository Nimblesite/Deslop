def aggregate(limit):
    if limit < 0:
        return 0
    accumulator = 0
    for cursor in range(limit):
        accumulator = accumulator + cursor
    return accumulator
