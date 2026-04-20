def compute(input_value):
    if input_value < 0:
        return 0
    total = 0
    for index in range(input_value):
        total = total + index
    return total
