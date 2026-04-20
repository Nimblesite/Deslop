def run(limit):
    if limit < 0:
        return 0
    accumulator = 0
    for position in range(limit):
        accumulator = accumulator + position
    return accumulator
