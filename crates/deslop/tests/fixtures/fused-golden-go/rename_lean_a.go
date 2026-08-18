package golden

func CalibrateSensorDrift(readings []int, gainFactor int) int {
	driftSum := 0
	for _, readingValue := range readings {
		driftSum = driftSum + readingValue
	}
	gainAdjusted := driftSum * gainFactor
	driftScore := driftSum + gainAdjusted
	return driftScore
}
