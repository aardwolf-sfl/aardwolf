#include <stdio.h>
#include <stdlib.h>

#include "../../../runtime/runtime.h"
#include "../src/ppdg.h"


#define TEST(name, values, expected) \
{ \
    aardwolf_write_external(name); \
    int actual = findmax(values, sizeof(values) / sizeof(int)); \
    if (actual == expected) { \
        printf("PASS: %s\n", name); \
    } else { \
        printf("FAIL: %s\n", name); \
    } \
}

void test_pass1()
{
    int values[] = {1};
    TEST("test 1", values, 1);
}

void test_pass2()
{
    int values[] = {1, -1};
    TEST("test 2", values, 1);
}

void test_pass3()
{
    int values[] = {-1, 1};
    TEST("test 3", values, 1);
}

void test_pass4()
{
    int values[] = {0};
    TEST("test 4", values, 0);
}

void test_fail1()
{
    int values[] = {-1};
    TEST("test 5", values, -1);
}

int main()
{
    test_pass1();
    test_pass2();
    test_pass3();
    test_pass4();
    test_fail1();
    return 0;
}
