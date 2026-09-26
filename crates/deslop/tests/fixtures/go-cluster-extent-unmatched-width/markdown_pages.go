package pages

import (
	"io"
	"os"
	"path/filepath"
	"strings"

	"example.com/tooling/tree"
)

// WriteMarkdownPages writes one markdown page for this node and every descendant
// into dir, with no header and links left as they are.
func WriteMarkdownPages(node *tree.Node, dir string) error {
	identity := func(s string) string { return s }
	noHeader := func(s string) string { return "" }
	return WriteMarkdownPagesWith(node, dir, noHeader, identity)
}

// WriteMarkdownPagesWith writes one markdown page for this node and every visible
// descendant, with a custom header and link handler.
func WriteMarkdownPagesWith(node *tree.Node, dir string, header, linkHandler func(string) string) error {
	for _, child := range node.Children() {
		if !child.IsVisible() || child.IsHelpTopic() {
			continue
		}
		if err := WriteMarkdownPagesWith(child, dir, header, linkHandler); err != nil {
			return err
		}
	}

	basename := strings.ReplaceAll(node.Path(), " ", "_") + markdownExtension
	filename := filepath.Join(dir, basename)
	f, err := os.Create(filename)
	if err != nil {
		return err
	}
	defer f.Close()

	if _, err := io.WriteString(f, header(filename)); err != nil {
		return err
	}
	if err := WriteMarkdownPage(node, f, linkHandler); err != nil {
		return err
	}
	return nil
}
