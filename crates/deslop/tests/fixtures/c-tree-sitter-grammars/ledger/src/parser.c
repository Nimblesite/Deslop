#include <stddef.h>

/* A hand-written recursive-descent parser that happens to be named parser.c. */
int parse_sum(const char *text, size_t length) {
  int total = 0;
  int current = 0;
  for (size_t index = 0; index < length; index++) {
    char symbol = text[index];
    if (symbol >= '0' && symbol <= '9') {
      current = current * 10 + (symbol - '0');
    } else if (symbol == '+') {
      total += current;
      current = 0;
    }
  }
  return total + current;
}
