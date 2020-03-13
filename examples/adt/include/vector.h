#ifndef VECTOR_H
#define VECTOR_H

#include <stdio.h>

typedef struct {
    int* data;
    int size;
    int capacity;
} vector_t;

int vector_new(vector_t *self);
int vector_with_capacity(vector_t *self, int capacity);
void vector_drop(vector_t *self);

int vector_push(vector_t *self, int value);
int vector_pop(vector_t *self, int *value);
int vector_insert(vector_t *self, int index, int value);
int vector_remove(vector_t *self, int index, int *value);

int vector_index(vector_t *self, int index);
int vector_len(vector_t *self);

void vector_debug(vector_t *self, FILE *fd);

#endif /* VECTOR_H */
