package describe

func describe(code int) string {
	if code == 200 {
		return "ok"
	}
	if code == 404 {
		return "missing"
	}
	if code == 500 {
		return "error"
	}
	return "unknown"
}
