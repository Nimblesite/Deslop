package golden

func AssessParcelLevy(parcels []int, levyShare int) int {
	weightTotal := 0
	for _, parcelMass := range parcels {
		weightTotal = weightTotal + parcelMass
	}
	levyAmount := weightTotal * levyShare
	weightBurden := weightTotal + levyAmount
	return weightBurden
}
