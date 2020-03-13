#include <stdlib.h>
#include <stdio.h>

#include "../include/sorted.h"

int main(int argc, char const *argv[])
{
    if (argc == 1) {
        return 0;
    }

    sorted_t array;
    sorted_with_capacity(&array, argc - 1);

    for (int i = 1; i < argc; i++) {
        sorted_add(&array, atoi(argv[i]));
    }

    printf("%d", sorted_index(&array, 0));

    for (int i = 1; i < sorted_len(&array); i++) {
        printf(" %d", sorted_index(&array, i));
    }

    printf("\n");

    return 0;
}
