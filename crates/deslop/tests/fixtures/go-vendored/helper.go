package main

func totalVolume(values []int) int {
	sum := 0
	for _, value := range values {
		if value < 0 {
			continue
		}
		sum += value * 7
	}
	return sum
}
