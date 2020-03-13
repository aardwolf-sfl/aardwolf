#include "../include/sorted.h"

int sorted_new(sorted_t *self)
{
    return vector_new(&self->data);
}

int sorted_with_capacity(sorted_t *self, int capacity)
{
    return vector_with_capacity(&self->data, capacity);
}

void sorted_drop(sorted_t *self)
{
    vector_drop(&self->data);
}

int sorted_add(sorted_t *self, int value)
{
    for (int i = 0; i < vector_len(&self->data); i++) {
        if (value < vector_index(&self->data, i)) {
            return vector_insert(&self->data, i, value);
        }
    }

    return vector_push(&self->data, value);
}

int sorted_pop_front(sorted_t *self, int *value)
{
    return vector_remove(&self->data, 0, value);
}

int sorted_pop_back(sorted_t *self, int *value)
{
    return vector_pop(&self->data, value);
}

int sorted_index(sorted_t *self, int index)
{
    return vector_index(&self->data, index);
}

int sorted_len(sorted_t *self)
{
    return vector_len(&self->data);
}

void sorted_debug(sorted_t *self, FILE *fd)
{
    fprintf(fd, "sorted_t(");
    vector_debug(&self->data, fd);
    fprintf(fd, ")");
}
