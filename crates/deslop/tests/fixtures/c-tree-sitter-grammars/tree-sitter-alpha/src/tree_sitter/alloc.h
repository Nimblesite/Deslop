#ifndef TREE_SITTER_ALLOC_H_
#define TREE_SITTER_ALLOC_H_

static inline void *ts_alloc_grow(void *contents, unsigned size, unsigned capacity, unsigned element) {
  unsigned wanted = capacity;
  while (wanted < size) {
    wanted = wanted == 0 ? 8 : wanted * 2;
  }
  if (wanted != capacity) {
    contents = realloc(contents, wanted * element);
  }
  return contents;
}

#endif
