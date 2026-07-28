package epsilon

func tally(limit int) int {
	if limit < 0 {
		return 0
	}
	accumulator := 0
	for cursor := 0; cursor <= limit; cursor++ {
		accumulator += cursor
	}
	return accumulator
}
