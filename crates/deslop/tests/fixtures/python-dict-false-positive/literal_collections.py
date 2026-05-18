# Literal-only collections: lists, tuples, and sets of plain values.
# These must NOT produce clone clusters — every entry is distinct data,
# not duplicated logic.
PRIMES = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47]
SMALL_PRIMES = (2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47)
ODD_PRIMES = {3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53}
