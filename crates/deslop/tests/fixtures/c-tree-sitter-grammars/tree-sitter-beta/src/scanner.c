#include "tree_sitter/parser.h"

/* Hand-written: tracks indentation for the external scanner. */
static unsigned measure_indent(TSLexer *lexer) {
  unsigned width = 0;
  for (;;) {
    if (lexer->lookahead == ' ') {
      width += 1;
    } else if (lexer->lookahead == '\t') {
      width += 8;
    } else {
      break;
    }
    lexer->advance(lexer, true);
  }
  return width;
}
