package pages

import (
	"io"
	"os"
	"path/filepath"
	"strings"

	"example.com/tooling/tree"
)

// pageOption is one option as a yaml page lists it.
type pageOption struct {
	Name         string
	Shorthand    string `yaml:",omitempty"`
	DefaultValue string `yaml:"default_value,omitempty"`
	Usage        string `yaml:",omitempty"`
}

// WriteYamlPages writes one yaml page for this node and every descendant
// into dir, with no header and links left as they are.
func WriteYamlPages(node *tree.Node, dir string) error {
	identity := func(s string) string { return s }
	noHeader := func(s string) string { return "" }
	return WriteYamlPagesWith(node, dir, noHeader, identity)
}

// WriteYamlPagesWith writes one yaml page for this node and every visible
// descendant, with a custom header and link handler.
func WriteYamlPagesWith(node *tree.Node, dir string, header, linkHandler func(string) string) error {
	for _, child := range node.Children() {
		if !child.IsVisible() || child.IsHelpTopic() {
			continue
		}
		if err := WriteYamlPagesWith(child, dir, header, linkHandler); err != nil {
			return err
		}
	}

	basename := strings.ReplaceAll(node.Path(), " ", "_") + ".yaml"
	filename := filepath.Join(dir, basename)
	f, err := os.Create(filename)
	if err != nil {
		return err
	}
	defer f.Close()

	if _, err := io.WriteString(f, header(filename)); err != nil {
		return err
	}
	if err := WriteYamlPage(node, f, linkHandler); err != nil {
		return err
	}
	return nil
}
