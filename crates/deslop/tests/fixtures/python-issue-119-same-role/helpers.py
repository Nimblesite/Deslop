def total_recursive(limit):
    if limit <= 0:
        return 0
    return limit + total_recursive(limit - 1)


def total_iterative(limit):
    running = 0
    index = 1
    while index <= limit:
        running = running + index
        index = index + 1
    return running
