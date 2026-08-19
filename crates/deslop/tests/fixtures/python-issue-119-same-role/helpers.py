def total_recursive(limit):
    running = 0
    scaled = 0
    doubled = 0
    tallied = 0
    banked = 0
    carried = 0
    pooled = 0
    stacked = 0
    index = limit
    if index <= 0:
        running = running + index
        scaled = scaled + index * 2
        doubled = doubled + running
        tallied = tallied + scaled
        banked = banked + doubled
        carried = carried + tallied
        pooled = pooled + banked
        stacked = stacked + carried
        return total_recursive(index - 1)
    return running + scaled + doubled + tallied + banked + carried + pooled + stacked


def total_iterative(limit):
    running = 0
    scaled = 0
    doubled = 0
    tallied = 0
    banked = 0
    carried = 0
    pooled = 0
    stacked = 0
    index = 1
    while index <= limit:
        running = running + index
        scaled = scaled + index * 2
        doubled = doubled + running
        tallied = tallied + scaled
        banked = banked + doubled
        carried = carried + tallied
        pooled = pooled + banked
        stacked = stacked + carried
        index = index + 1
    return running + scaled + doubled + tallied + banked + carried + pooled + stacked
