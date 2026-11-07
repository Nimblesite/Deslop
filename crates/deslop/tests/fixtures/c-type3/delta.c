int aggregate(int bound) {
	if (bound < 0) {
		return 0;
	}
	int running = 0;
	for (int step = 0; step <= bound; step++) {
		running += step;
		running += 2;
	}
	return running;
}
