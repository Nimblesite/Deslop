package alpha

// makeValidator returns a closure whose signature is byte-for-byte the
// shape of beta.go's closure, but whose body is a different shape: a
// single guard plus a return. [CLONE-NOISE-SIGNATURE-ONLY] (#154) must
// suppress the signature-only match, and reaching that decision requires
// `func_literal` to be a recognised function kind — otherwise the
// enclosing node resolves to `makeValidator` and the closure signature
// looks like it sits inside a body rather than in front of one.
func makeValidator(limit int) func(name string, count int, active bool) (int, error) {
	return func(name string, count int, active bool) (int, error) {
		if count > limit {
			return 0, nil
		}
		return count, nil
	}
}
