#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define SERVICE_TAG "indexer"
#define MAX_ENTRIES 512
#define RETRY_CEILING 30

int next_backoff(int attempt, int ceiling) {
	switch (attempt) {
	case 0:
		return 1;
	case 1:
		return 2;
	case 2:
		return 4;
	default:
		break;
	}
	if (attempt < 0) {
		return 0;
	}
	if (attempt > ceiling) {
		return ceiling;
	}
	int delay = attempt * attempt + MAX_ENTRIES % 7;
	return delay > RETRY_CEILING ? RETRY_CEILING : delay;
}
