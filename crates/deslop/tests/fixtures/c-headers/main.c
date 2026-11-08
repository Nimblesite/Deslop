#include <stdio.h>
#include "util.h"

int main(void) {
	const struct util_pair samples[] = {
		{1, 30},
		{2, 85},
		{3, 42},
	};
	const int count = (int)(sizeof samples / sizeof samples[0]);
	const int total = util_sum(samples, count);
	const int best = util_max(samples, count);
	printf("total=%d best=%d grade=%s\n", total, best, util_grade(best));
	return 0;
}
