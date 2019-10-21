#include "ops.h"

#include "stats.h"

float * normalize(float *array, int size)
{
    float max = get_max(array, size); // Bug in `get_max`
    int i;

    for (i = 0; i < size; i++) {
        array[i] /= max;
    }

    return array;
}

float * matrix_dot(float *result, float *A, float *B, int n, int k, int m)
{
    int i, j, l, r;

    for (i = 0; i < n; i++) {
        for (j = 0; j < m; j++) {
            r = i * n + j; // Should be `i * m + j`
            result[r] = 0;
            for (l = 0; l < k; l++) {
                result[r] += A[i * k + l] * B[m * l + j];
            }
        }
    }

    return result;
}
