def accumulate(values, floor):
    total = 0
    for value in values:
        if value > floor:
            total = total + value * 2
        else:
            total = total - 1
    return total
