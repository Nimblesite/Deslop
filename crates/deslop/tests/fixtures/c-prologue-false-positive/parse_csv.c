#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define SERVICE_TAG "indexer"
#define MAX_ENTRIES 512
#define RETRY_CEILING 30

int parse_csv(const char *body, int *columns_per_row, int max_rows) {
	int row_count = 0;
	const char *cursor = body;
	while (*cursor != '\0' && row_count < max_rows) {
		const char *line_end = strchr(cursor, '\n');
		if (line_end == NULL) {
			line_end = cursor + strlen(cursor);
		}
		if (line_end > cursor) {
			int columns = 1;
			for (const char *scan = cursor; scan < line_end; scan++) {
				if (*scan == ',') {
					columns++;
				}
			}
			columns_per_row[row_count] = columns;
			row_count++;
		}
		if (*line_end == '\0') {
			break;
		}
		cursor = line_end + 1;
	}
	if (row_count == 0) {
		return -1;
	}
	return row_count;
}
