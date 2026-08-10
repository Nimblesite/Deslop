package golden

func Route(weight int, distance int, carrier string) string {
	score := weight*3 + distance
	if score > 900 {
		return carrier + "-freight"
	}
	if score > 400 {
		return carrier + "-ground"
	}
	return carrier + "-parcel"
}
