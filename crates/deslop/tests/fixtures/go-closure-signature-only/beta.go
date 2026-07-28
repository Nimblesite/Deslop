package beta

// makeAccumulator carries the same closure signature as alpha.go and a
// deliberately different closure body: a running total over a counted
// loop. Nothing here is copy-pasted from alpha.go — only the parameter
// list and result types coincide, which is exactly the false positive
// #154 exists to kill.
func makeAccumulator(width int) func(name string, count int, active bool) (int, error) {
	return func(name string, count int, active bool) (int, error) {
		total := width
		for index := 0; index < count; index++ {
			total = total + index
		}
		return total, nil
	}
}
