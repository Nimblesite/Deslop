# osprey: flavor=python
def describe(value):
    zero = 0
    step = 1
    multiplier = 2
    boundary = 100
    adjusted = value * multiplier + step
    if adjusted > boundary:
        return adjusted - value + step
    if value > zero:
        return adjusted + value * step
    else:
        return adjusted - value * step
