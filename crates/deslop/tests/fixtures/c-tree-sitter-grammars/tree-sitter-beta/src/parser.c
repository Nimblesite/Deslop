#include "tree_sitter/parser.h"

#define LANGUAGE_VERSION 14
#define STATE_COUNT 6

static bool ts_lex(TSLexer *lexer, TSStateId state) {
  START_LEXER();
  switch (state) {
    case 0:
      if (lookahead == '(') ADVANCE(1);
      if (lookahead == ')') ADVANCE(2);
      if (lookahead == '+') ADVANCE(3);
      END_STATE();
    case 1:
      ACCEPT_TOKEN(anon_sym_LPAREN);
      END_STATE();
    case 2:
      ACCEPT_TOKEN(anon_sym_RPAREN);
      END_STATE();
    case 3:
      ACCEPT_TOKEN(anon_sym_PLUS);
      END_STATE();
    default:
      return false;
  }
}
