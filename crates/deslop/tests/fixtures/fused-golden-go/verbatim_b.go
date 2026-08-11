package golden

func Accumulate(values []int, floor int) int {
	total := 0
	for _, value := range values {
		if value > floor {
			total = total + value*2
		} else {
			total = total - 1
		}
	}
	return total
}
