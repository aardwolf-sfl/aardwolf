#include <string.h>

#include "framework.h"

#include "../src/stats.h"
#include "../src/ops.h"

void test_get_max_positive()
{
    float A[] = { 3.14, 23, 42 };
    ASSERT(get_max(A, 3) == 42);
}

void test_get_max_mixed()
{
    float A[] = { 3.14, 23, -42 };
    ASSERT(get_max(A, 3) == 23);
}

void test_get_max_negative()
{
    float A[] = { -3.14, -23, -42 };
    ASSERT(get_max(A, 3) == 3.14);
}

void test_normalize_positive()
{
    float A[] = { 3.14, 23, 42 };
    float R[] = { 3.14 / 42, 23.0 / 42, 1 };

    ASSERT(memcmp(normalize(A, 3), R, sizeof(float) * 3) == 0);
}

void test_normalize_mixed()
{
    float A[] = { 3.14, 23, -42 };
    float R[] = { 3.14 / 23, 1, -42.0 / 23 };

    ASSERT(memcmp(normalize(A, 3), R, sizeof(float) * 3) == 0);
}

void test_normalize_negative()
{
    float A[] = { -3.14, -23, -42 };
    float R[] = { 1, -23 / -3.14, -42 / -3.14 };

    ASSERT(memcmp(normalize(A, 3), R, sizeof(float) * 3) == 0);
}

void test_matrix_dot_square()
{
    float A[] = { 2, 3, 4, 5 };
    float B[] = { 1, 0, 0, 1 };
    float C[4];
    float R[] = { 2, 3, 4, 5 };

    ASSERT(memcmp(matrix_dot(C, A, B, 2, 2, 2), R, sizeof(float) * 4) == 0);
}

void test_matrix_dot_rectangle()
{
    float A[] = { 2, 3, 4, 5, 6, 7 };
    float B[] = { 1, 0, 0 };
    float C[2];
    float R[] = { 2, 5 };

    ASSERT(memcmp(matrix_dot(C, A, B, 2, 3, 1), R, sizeof(float) * 2) == 0);
}

int main()
{
    TEST(test_get_max_positive);
    TEST(test_get_max_mixed);
    TEST(test_get_max_negative);
    TEST(test_normalize_positive);
    TEST(test_normalize_mixed);
    TEST(test_normalize_negative);
    TEST(test_matrix_dot_square);
    TEST(test_matrix_dot_rectangle);

    return 0;
}
