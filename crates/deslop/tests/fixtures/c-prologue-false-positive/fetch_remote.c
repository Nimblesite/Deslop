#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define SERVICE_TAG "indexer"
#define MAX_ENTRIES 512
#define RETRY_CEILING 30

int fetch_remote(const char *url, int retries, char *out, int out_size) {
	int status = -1;
	for (int attempt = 0; attempt < retries; attempt++) {
		if (strncmp(url, "https://", 8) != 0) {
			status = 2;
			continue;
		}
		const char *cursor = url + 8;
		while (*cursor != '\0' && *cursor != '/') {
			cursor++;
		}
		if (*cursor == '\0') {
			status = 3;
			break;
		}
		int delay = 1;
		for (int backoff = 0; backoff < attempt && delay < RETRY_CEILING; backoff++) {
			delay *= 2;
		}
		int written = snprintf(out, out_size, "%s#%d?delay=%d", url, attempt, delay);
		if (written > 0 && written < out_size) {
			return 0;
		}
		status = 4;
	}
	return status;
}
