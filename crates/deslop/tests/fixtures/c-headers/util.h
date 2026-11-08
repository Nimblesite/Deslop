#ifndef UTIL_H
#define UTIL_H

struct util_pair {
	int key;
	int value;
};

int util_sum(const struct util_pair *pairs, int count);

int util_max(const struct util_pair *pairs, int count);

const char *util_grade(int score);

#endif
