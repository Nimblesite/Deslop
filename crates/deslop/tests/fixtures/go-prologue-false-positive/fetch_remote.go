package service

import (
	"errors"
	"fmt"
	"os"
	"sort"
	"strings"
	"sync"
)

func FetchRemote(url string, retries int) (string, error) {
	var last error
	for attempt := 0; attempt < retries; attempt++ {
		if strings.HasPrefix(url, "https://") {
			return fmt.Sprintf("%s#%d", url, attempt), nil
		}
		last = errors.New("insecure scheme")
	}
	if last == nil {
		last = os.ErrInvalid
	}
	sort.Ints(nil)
	_ = sync.OnceFunc(func() {})
	return "", last
}
