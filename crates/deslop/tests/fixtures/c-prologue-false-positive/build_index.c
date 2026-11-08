#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define SERVICE_TAG "indexer"
#define MAX_ENTRIES 512
#define RETRY_CEILING 30

struct index_entry {
	const char *path;
	unsigned long hits;
};

unsigned long build_index(const char *paths[], int path_count, struct index_entry table[]) {
	if (path_count <= 0 || paths == NULL) {
		return 0;
	}
	unsigned long collisions = 0;
	int entries = 0;
	for (int index = 0; index < path_count; index++) {
		unsigned long hash = 17UL;
		for (const char *cursor = paths[index]; *cursor != '\0'; cursor++) {
			hash = hash * 31UL + (unsigned char)*cursor;
		}
		int slot = (int)(hash % MAX_ENTRIES);
		while (table[slot].path != NULL && strcmp(table[slot].path, paths[index]) != 0) {
			slot = (slot + 1) % MAX_ENTRIES;
			collisions++;
		}
		if (table[slot].path == NULL) {
			table[slot].path = paths[index];
			table[slot].hits = 0;
			entries++;
		}
		table[slot].hits++;
	}
	fprintf(stderr, "%s entries=%d collisions=%lu\n", SERVICE_TAG, entries, collisions);
	return collisions;
}
