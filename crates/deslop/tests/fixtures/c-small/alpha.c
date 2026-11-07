int accumulate(int limit) {
	int total = 0;
	for (int index = 1; index <= limit; index++) {
		if (index % 2 == 0) {
			total += index * 3;
		} else {
			total += 1;
		}
	}
	return total;
}
