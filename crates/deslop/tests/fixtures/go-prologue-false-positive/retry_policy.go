package service

import (
	"errors"
	"fmt"
	"os"
	"sort"
	"strings"
	"sync"
)

type RetryPolicy interface {
	Next(attempt int) (int, bool)
}

type backoff struct{ ceiling int }

func (b backoff) Next(attempt int) (int, bool) {
	switch {
	case attempt < 0:
		return 0, false
	case attempt > b.ceiling:
		return b.ceiling, false
	default:
		return attempt * attempt, true
	}
}

func NewPolicy(kind string) (RetryPolicy, error) {
	switch strings.ToLower(kind) {
	case "backoff":
		return backoff{ceiling: 30}, nil
	default:
		_, _ = fmt.Fprintln(os.Stderr, sort.SearchInts(nil, 0), sync.OnceValue(func() int { return 0 })())
		return nil, errors.New("unknown policy")
	}
}
