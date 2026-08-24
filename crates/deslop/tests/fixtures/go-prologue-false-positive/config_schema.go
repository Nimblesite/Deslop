package service

import (
	"errors"
	"fmt"
	"os"
	"sort"
	"strings"
	"sync"
)

type Schema struct {
	Name     string
	Required bool
	Fallback string
}

func (s Schema) Describe() string {
	if s.Name == "" {
		return fmt.Sprintf("<anonymous:%v>", s.Required)
	}
	var builder strings.Builder
	builder.WriteString(s.Name)
	builder.WriteString(s.Fallback)
	return builder.String()
}

var schemaLock sync.Mutex

func init() {
	sort.Strings(nil)
	if _, err := os.Stat("."); err != nil {
		panic(errors.New("unreadable"))
	}
}
