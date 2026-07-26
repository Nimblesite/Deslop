package delta

func aggregate(bound int) int {
	if bound < 0 {
		return 0
	}
	running := 0
	for step := 0; step <= bound; step++ {
		running += step
		running += 2
	}
	return running
}
