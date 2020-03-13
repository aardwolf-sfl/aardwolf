#ifndef SORTED_H
#define SORTED_H

#include <stdio.h>

#include "../include/vector.h"

typedef struct {
    vector_t data;
} sorted_t;

int sorted_new(sorted_t *self);
int sorted_with_capacity(sorted_t *self, int capacity);
void sorted_drop(sorted_t *self);

int sorted_add(sorted_t *self, int value);
int sorted_pop_front(sorted_t *self, int *value);
int sorted_pop_back(sorted_t *self, int *value);

int sorted_index(sorted_t *self, int index);
int sorted_len(sorted_t *self);

void sorted_debug(sorted_t *self, FILE *fd);

#endif /* SORTED_H */
