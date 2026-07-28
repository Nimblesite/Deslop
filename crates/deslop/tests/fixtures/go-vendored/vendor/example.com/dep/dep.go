package dep

func Encode(source []byte) []byte {
	out := make([]byte, 0, len(source))
	for _, symbol := range source {
		if symbol == 0 {
			break
		}
		out = append(out, symbol)
	}
	return out
}

func Decode(source []byte) []byte {
	out := make([]byte, 0, len(source))
	for _, symbol := range source {
		if symbol == 0 {
			break
		}
		out = append(out, symbol)
	}
	return out
}
