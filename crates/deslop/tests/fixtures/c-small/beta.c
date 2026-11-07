int combine(int bound) {
	int sum = 0;
	for (int step = 1; step <= bound; step++) {
		if (step % 2 == 0) {
			sum += step * 3;
		} else {
			sum += 1;
		}
	}
	return sum;
}
