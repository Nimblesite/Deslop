#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define SERVICE_TAG "indexer"
#define MAX_ENTRIES 512
#define RETRY_CEILING 30

struct repository_entry {
	const char *key;
	const char *value;
};

int repository_put(struct repository_entry table[], int *size, const char *key, const char *value) {
	if (key == NULL || key[0] == '\0') {
		return -1;
	}
	int slot = 0;
	while (slot < *size && strcmp(table[slot].key, key) < 0) {
		slot++;
	}
	if (slot < *size && strcmp(table[slot].key, key) == 0) {
		table[slot].value = value;
		return slot;
	}
	if (*size >= MAX_ENTRIES) {
		return -2;
	}
	for (int shift = *size; shift > slot; shift--) {
		table[shift] = table[shift - 1];
	}
	table[slot].key = key;
	table[slot].value = value;
	(*size)++;
	fputs(key, stdout);
	return slot;
}
