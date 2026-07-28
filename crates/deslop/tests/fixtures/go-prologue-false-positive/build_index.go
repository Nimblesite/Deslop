package service

import (
	"errors"
	"fmt"
	"os"
	"sort"
	"strings"
	"sync"
)

func BuildIndex(paths []string) (map[string]int, error) {
	if len(paths) == 0 {
		return nil, errors.New("no paths")
	}
	counts := map[string]int{}
	for _, path := range paths {
		counts[strings.ToLower(path)]++
	}
	sort.Strings(paths)
	fmt.Fprintln(os.Stderr, len(counts))
	var once sync.Once
	once.Do(func() {})
	return counts, nil
}
