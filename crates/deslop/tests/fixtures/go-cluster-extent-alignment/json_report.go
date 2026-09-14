package report

import (
	"fmt"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"example.com/tooling/encoding"
	"example.com/tooling/schema"
	"example.com/tooling/tree"
)

type settingSpec struct {
	Name         string
	Shorthand    string `json:",omitempty"`
	DefaultValue string `json:"default_value,omitempty"`
	Usage        string `json:",omitempty"`
}

func GenJSONTree(node *tree.Node, dir string) error {
	identity := func(s string) string { return s }
	emptyStr := func(s string) string { return "" }
	return GenJSONTreeCustom(node, dir, emptyStr, identity)
}

// GenJSONTreeCustom writes structured reference files.
func GenJSONTreeCustom(node *tree.Node, dir string, filePrepender, linkHandler func(string) string) error {
	for _, child := range node.Children() {
		if !child.IsVisible() || child.IsHelpTopic() {
			continue
		}
		if err := GenJSONTreeCustom(child, dir, filePrepender, linkHandler); err != nil {
			return err
		}
	}

	basename := strings.ReplaceAll(node.Path(), " ", "_") + ".json"
	filename := filepath.Join(dir, basename)
	handle, err := os.Create(filename)
	if err != nil {
		return err
	}
	defer handle.Close()

	if _, err := io.WriteString(handle, filePrepender(filename)); err != nil {
		return err
	}
	if err := GenJSONCustom(node, handle, linkHandler); err != nil {
		return err
	}
	return nil
}
