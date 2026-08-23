import math

BASELINE = 4

values = [1, 2, 3]
def extracted_from_cluster_bb739e(values, BASELINE, sum, math, len, print):
    scaled = [value * BASELINE for value in values]
    report = {"total": sum(scaled), "sqrt": math.sqrt(len(scaled)), "count": len(values)}
    print(report)


extracted_from_cluster_bb739e(values, BASELINE, sum, math, len, print)

values = list(range(4, 7))
extracted_from_cluster_bb739e(values, BASELINE, sum, math, len, print)
