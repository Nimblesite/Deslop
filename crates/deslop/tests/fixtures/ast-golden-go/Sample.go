// Package sample exercises the Go normalisation rules: identifier
// collapse (identifier, field_identifier, type_identifier,
// package_identifier, blank_identifier, label_name), literal collapse
// (int, float, imaginary, rune, interpreted + raw strings with escape
// sequences, true, false, nil, iota), comment drop, and the structural
// forms most likely to shift between grammar patch releases.
package sample

import (
	"fmt"
	"strings"
)

// Weekday pins type_identifier collapse and the iota value leaf.
type Weekday int

const (
	sunday Weekday = iota
	monday
)

// point pins struct field_identifier collapse and float literals.
type point struct {
	x float64
	y float64
}

/* Block comments are dropped too. */
func (p point) scale(factor float64) point {
	return point{x: p.x * factor, y: p.y * factor}
}

func classify(value int, flag bool) string {
	var builder strings.Builder
	raw := `raw literal`
	escaped := "tab\tend"
	marker := 'x'
	phase := 3.5i
	_ = phase
	enabled := true
search:
	for index := 0; index < value; index++ {
		switch {
		case index%2 == 0 && flag:
			continue search
		default:
			break search
		}
	}
	if value > 10 || raw == escaped {
		builder.WriteString(fmt.Sprintf("big %d %c", value, marker))
		return builder.String()
	}
	var nothing *point
	if nothing == nil && enabled != false {
		return "small"
	}
	return "other"
}
