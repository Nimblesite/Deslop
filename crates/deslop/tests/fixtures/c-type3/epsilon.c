int tally(int limit) {
	if (limit < 0) {
		return 0;
	}
	int accumulator = 0;
	for (int cursor = 0; cursor <= limit; cursor++) {
		accumulator += cursor;
	}
	return accumulator;
}
