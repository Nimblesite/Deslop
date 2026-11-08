#ifndef LIST_H
#define LIST_H
struct node { int value; struct node *next; };
struct node *list_insert_sorted(struct node *head, int value);
int list_length(const struct node *head);
#endif
