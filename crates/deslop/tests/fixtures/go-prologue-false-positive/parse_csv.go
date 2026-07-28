package service

import (
	"errors"
	"fmt"
	"os"
	"sort"
	"strings"
	"sync"
)

func ParseCSV(body string) ([][]string, error) {
	rows := [][]string{}
	for _, line := range strings.Split(body, "\n") {
		if line == "" {
			continue
		}
		rows = append(rows, strings.Split(line, ","))
	}
	if len(rows) == 0 {
		return nil, fmt.Errorf("empty: %w", errors.ErrUnsupported)
	}
	sort.SliceStable(rows, func(left, right int) bool { return len(rows[left]) < len(rows[right]) })
	_ = os.Getenv("CSV")
	_ = sync.WaitGroup{}
	return rows, nil
}
