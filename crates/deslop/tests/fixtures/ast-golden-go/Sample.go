// Package sample exercises the Go normalisation rules: identifier
// collapse (identifier, field_identifier, type_identifier,
// package_identifier, blank_identifier, label_name), literal collapse
// (int, float, imaginary, rune, interpreted + raw strings with escape
// sequences, true, false, nil, iota), comment drop, and the structural
// forms most likely to shift between grammar patch releases.
//
// The second half of the file is Go's own identity: goroutines, defer,
// channel send/receive, select, range, type switches, variadics,
// generics, interfaces, and struct tags. None of those shapes appear in
// any other language's golden, and every one of them is a plausible
// target for a tree-sitter-go patch release — so each is pinned here.
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

// Stringer pins interface_type and its method specifications.
type Stringer interface {
	String() string
	Len() int
}

// tagged pins struct field tags — a string literal used as metadata
// rather than a value. It must still collapse to the literal
// placeholder, or a tag edit would perturb every fingerprint in the file.
type tagged struct {
	Name  string `json:"name" xml:"name"`
	Count int    `json:"count,omitempty"`
}

// pair pins the generic type_parameter_list and its constraints.
type pair[K comparable, V any] struct {
	key   K
	value V
}

// mapAll pins a generic function's type parameters together with a
// variadic parameter declaration and a func type parameter.
func mapAll[T any](transform func(T) T, items ...T) []T {
	out := make([]T, 0, len(items))
	for _, item := range items {
		out = append(out, transform(item))
	}
	return out
}

// pump pins goroutines, defer, directional channel types, send
// statements, receive expressions, and select with a default arm.
func pump(source <-chan int, sink chan<- int, done chan struct{}) {
	defer close(sink)
	go func() {
		sink <- 1
	}()
	for {
		select {
		case value, ok := <-source:
			if !ok {
				return
			}
			sink <- value * 2
		case <-done:
			return
		default:
			return
		}
	}
}

// describe pins a type switch, its multi-type case, an interface case,
// a nil case, and the default arm.
func describe(value any) string {
	switch typed := value.(type) {
	case int, int64:
		return fmt.Sprint(typed)
	case Stringer:
		return typed.String()
	case nil:
		return "nil"
	default:
		return "other"
	}
}

// drain pins a range clause over a channel, a generic type instantiation,
// and a deferred closure.
func drain(values <-chan pair[string, int]) int {
	total := 0
	defer func() {
		_ = total
	}()
	for entry := range values {
		total += entry.value
	}
	return total
}
