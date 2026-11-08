#include "util.h"

int util_sum(const struct util_pair *pairs, int count) {
	int total = 0;
	for (int index = 0; index < count; index++) {
		total += pairs[index].value;
	}
	return total;
}

int util_max(const struct util_pair *pairs, int count) {
	int best = count > 0 ? pairs[0].value : 0;
	for (int index = 1; index < count; index++) {
		if (pairs[index].value > best) {
			best = pairs[index].value;
		}
	}
	return best;
}

const char *util_grade(int score) {
	if (score >= 90) {
		return "excellent";
	}
	if (score >= 75) {
		return "solid";
	}
	if (score >= 50) {
		return "borderline";
	}
	return "fail";
}
