#include "stats.h"

float get_max(float *array, int size)
{
    float max = 0; // Should be FLT_MIN from float.h
    int i;

    for (i = 0; i < size; i++) {
        if (array[i] > max) {
            max = array[i];
        }
    }

    return max;
}
