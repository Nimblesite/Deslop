def alpha(value):
    if value < 0:
        return 0
    total = 0
    for index in range(value):
        total = total + index
    return total


def beta(bound):
    if bound < 0:
        return 0
    running = 0
    for step in range(bound):
        running = running + step
    return running
