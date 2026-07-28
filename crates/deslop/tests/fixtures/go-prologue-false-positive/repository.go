package service

import (
	"errors"
	"fmt"
	"os"
	"sort"
	"strings"
	"sync"
)

type Repository struct {
	mu      sync.RWMutex
	records map[string]string
}

func (r *Repository) Put(key, value string) error {
	r.mu.Lock()
	defer r.mu.Unlock()
	if key == "" {
		return errors.New("blank key")
	}
	r.records[key] = value
	return nil
}

func (r *Repository) Keys() []string {
	keys := make([]string, 0, len(r.records))
	for key := range r.records {
		keys = append(keys, strings.TrimSpace(key))
	}
	sort.Strings(keys)
	fmt.Fprint(os.Stdout, len(keys))
	return keys
}
