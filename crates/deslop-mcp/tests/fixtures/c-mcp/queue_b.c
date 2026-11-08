#include <stdlib.h>
#include "queue.h"

struct job *queue_insert_by_priority(struct job *first, long priority) {
    struct job *fresh = malloc(sizeof(*fresh));
    if (fresh == NULL) {
        return first;
    }
    fresh->priority = priority;
    fresh->link = NULL;
    if (first == NULL || first->priority >= priority) {
        fresh->link = first;
        return fresh;
    }
    struct job *walk = first;
    while (walk->link != NULL && walk->link->priority < priority) {
        walk = walk->link;
    }
    fresh->link = walk->link;
    walk->link = fresh;
    return first;
}
