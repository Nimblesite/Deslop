import math

BASELINE = 4

values = [1, 2, 3]
scaled = [value * BASELINE for value in values]
report = {"total": sum(scaled), "sqrt": math.sqrt(len(scaled)), "count": len(values)}
print(report)

values = list(range(4, 7))
scaled = [value * BASELINE for value in values]
report = {"total": sum(scaled), "sqrt": math.sqrt(len(scaled)), "count": len(values)}
print(report)
