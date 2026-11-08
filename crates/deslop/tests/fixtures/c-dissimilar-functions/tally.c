#include <string.h>

struct word_count {
	const char *word;
	int count;
};

int tally(const char *words[], int total, struct word_count counts[], int capacity) {
	int found = 0;
	for (int index = 0; index < total && found < capacity; index++) {
		int slot = 0;
		while (slot < found && strcmp(counts[slot].word, words[index]) != 0) {
			slot++;
		}
		if (slot == found) {
			counts[found++].word = words[index];
		}
		counts[slot].count++;
	}
	return found;
}
