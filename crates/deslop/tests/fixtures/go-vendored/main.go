package main

func totalWeight(items []int) int {
	total := 0
	for _, item := range items {
		if item < 0 {
			continue
		}
		total += item * 3
	}
	return total
}
