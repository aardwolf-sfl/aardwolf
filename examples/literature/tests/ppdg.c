#include <stdio.h>
#include <stdlib.h>

#include "../../../runtime/runtime.h"
#include "../src/ppdg.h"


#define TEST(name, values, expected) \
{ \
    aardwolf_write_external(name); \
    int actual = findmax(values, sizeof(values) / sizeof(int)); \
    if (actual == expected) { \
        printf("\"%s\": PASSED\n", name); \
    } else { \
        printf("\"%s\": FAILED\n", name); \
    } \
}

void test_all_positive()
{
    int values[] = {1, 2, 3};
    TEST("all positive", values, 3);
}

void test_mixed()
{
    int values[] = {1, -2, 3};
    TEST("mixed", values, 3);
}

void test_all_negative()
{
    int values[] = {-1, -2, -3};
    TEST("all negative", values, -1);
}

int main()
{
    test_all_positive();
    test_mixed();
    test_all_negative();
    return 0;
}
