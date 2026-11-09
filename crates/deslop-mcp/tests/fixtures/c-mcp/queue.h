#ifndef QUEUE_H
#define QUEUE_H
struct job { long priority; struct job *link; };
struct job *queue_insert_by_priority(struct job *first, long priority);
int queue_length(const struct job *first);
#endif
