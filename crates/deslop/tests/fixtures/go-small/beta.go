package beta

func combine(bound int) int {
	sum := 0
	for step := 1; step <= bound; step++ {
		if step%2 == 0 {
			sum += step * 7
		} else {
			sum += 4
		}
	}
	return sum
}
