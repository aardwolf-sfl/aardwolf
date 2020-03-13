#include "../include/vector.h"

#include <assert.h>
#include <stdlib.h>
#include <string.h>

#define CAPACITY_INIT 16
#define RESIZE_FACTOR 2

int vector_new(vector_t *self)
{
    return vector_with_capacity(self, CAPACITY_INIT);
}

int vector_with_capacity(vector_t *self, int capacity)
{
    self->capacity = capacity;
    self->size = 0;
    self->data = (int*)malloc(sizeof(*self->data) * self->capacity);

    if (self->data) {
        return 1;
    } else {
        return 0;
    }
}

void vector_drop(vector_t *self)
{
    free(self->data);
}

int ensure_space(vector_t *self)
{
    if (self->size == self->capacity) {
        self->capacity *= RESIZE_FACTOR;
        self->data = (int*)realloc(self->data, sizeof(*self->data) * self->capacity);

        if (self->data) {
            return 1;
        } else {
            return 0;
        }
    } else {
        return 1;
    }
}

int vector_push(vector_t *self, int value)
{
    if (!ensure_space(self)) {
        return 0;
    }

    self->data[self->size++] = value;
    return 1;
}

int vector_pop(vector_t *self, int *value)
{
    if (self->size == 0) {
        *value = 0;
        return 0;
    } else {
        *value = self->data[--self->size];
        return 1;
    }
}

int n_bytes(vector_t *self, int index)
{
    return (self->size - index) * sizeof(*self->data);
}

int vector_insert(vector_t *self, int index, int value)
{
    if (index < 0 || index >= self->size) {
        return 0;
    }

    if (!ensure_space(self)) {
        return 0;
    }

    memmove(&self->data[index + 1], &self->data[index], n_bytes(self, index));
    self->data[index] = value;

    self->size++;

    return 1;
}

int vector_remove(vector_t *self, int index, int *value)
{
    if (index < 0 || index >= self->size) {
        *value = 0;
        return 0;
    }

    *value = self->data[index];
    memmove(&self->data[index], &self->data[index + 1], n_bytes(self, index + 1));

    self->size--;

    return 1;
}

int vector_index(vector_t *self, int index)
{
    assert(index >= 0 && index < self->size && "index out of bounds");
    return self->data[index];
}

int vector_len(vector_t *self)
{
    return self->size;
}

void vector_debug(vector_t *self, FILE *fd)
{
    fprintf(fd, "[");

    if (self->size > 0) {
        fprintf(fd, "%d", self->data[0]);
    }

    for (int i = 1; i < self->size; i++) {
        fprintf(fd, ", %d", self->data[i]);
    }

    fprintf(fd, "]");
}
