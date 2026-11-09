#include <stdlib.h>
#include <string.h>
#include "list.h"

struct node *list_insert_sorted(struct node *head, int value) {
    struct node *created = malloc(sizeof(*created));
    if (created == NULL) {
        return head;
    }
    created->value = value;
    created->next = NULL;
    if (head == NULL || head->value >= value) {
        created->next = head;
        return created;
    }
    struct node *cursor = head;
    while (cursor->next != NULL && cursor->next->value < value) {
        cursor = cursor->next;
    }
    created->next = cursor->next;
    cursor->next = created;
    return head;
}

int list_length(const struct node *head) {
    int count = 0;
    for (; head != NULL; head = head->next) count++;
    return count;
}
