def accumulate(bound):
    if bound < 0:
        return 0
    running = 0
    for step in range(bound):
        running = running + step
        running = running + 2
    return running
