package report

import (
	"bytes"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"

	"example.com/tooling/tree"
)

const textExtension = ".txt"

func writeSettings(buf *bytes.Buffer, node *tree.Node, name string) error {
	settings := node.OwnSettings()
	settings.SetOutput(buf)
	if settings.HasVisibleSettings() {
		buf.WriteString("### Settings\n\n```\n")
		settings.PrintDefaults()
		buf.WriteString("```\n\n")
	}

	inherited := node.InheritedSettings()
	inherited.SetOutput(buf)
	if inherited.HasVisibleSettings() {
		buf.WriteString("### Settings inherited from parent nodes\n\n```\n")
		inherited.PrintDefaults()
		buf.WriteString("```\n\n")
	}
	return nil
}

// GenTextTree writes a plain-text page for this node and all descendants.
func GenTextTree(node *tree.Node, dir string) error {
	identity := func(s string) string { return s }
	emptyStr := func(s string) string { return "" }
	return GenTextTreeCustom(node, dir, emptyStr, identity)
}

// GenTextTreeCustom is the same as GenTextTree, but
// with custom filePrepender and linkHandler.
func GenTextTreeCustom(node *tree.Node, dir string, filePrepender, linkHandler func(string) string) error {
	for _, child := range node.Children() {
		if !child.IsVisible() || child.IsHelpTopic() {
			continue
		}
		if err := GenTextTreeCustom(child, dir, filePrepender, linkHandler); err != nil {
			return err
		}
	}

	basename := strings.ReplaceAll(node.Path(), " ", "_") + textExtension
	filename := filepath.Join(dir, basename)
	handle, err := os.Create(filename)
	if err != nil {
		return err
	}
	defer handle.Close()

	if _, err := io.WriteString(handle, filePrepender(filename)); err != nil {
		return err
	}
	if err := GenTextCustom(node, handle, linkHandler); err != nil {
		return err
	}
	return nil
}
