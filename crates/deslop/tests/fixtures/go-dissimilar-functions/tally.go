package tally

func tally(words []string) map[string]int {
	counts := map[string]int{}
	for _, word := range words {
		current := counts[word]
		counts[word] = current + 1
	}
	return counts
}
