#ifndef CALC_PARSER_H_
#define CALC_PARSER_H_

#include <stddef.h>

/* Hand-written: skips spaces before a token. */
static inline size_t skip_blanks(const char *text, size_t length, size_t index) {
  while (index < length) {
    char symbol = text[index];
    if (symbol != ' ' && symbol != '\t' && symbol != '\n') {
      break;
    }
    index += 1;
  }
  return index;
}

#endif
