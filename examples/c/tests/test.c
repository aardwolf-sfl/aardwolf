#include "framework.h"

#include "../src/maintenance.h"

#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <stdio.h>

void make_standardized(char *dest, Standard standard, const char *name)
{
    time_t now;
    time(&now);
    char timestamp[10];

    if (standard == A) {
        strftime(timestamp, 10, "%y%m%d", localtime(&now));
        sprintf(dest, "A_%s_%s", name, timestamp);
    } else if (standard == B) {
        strftime(timestamp, 10, "%Y%m%d", localtime(&now));
        sprintf(dest, "B%s%s", name, timestamp);
    }
}

bool_t is_deep_equal(char **actual, int actual_size, char **expected, int expected_size)
{
    bool_t result = actual_size == expected_size;
    for (int i = 0; result && i < actual_size; i++) {
        result = result && strcmp(actual[i], expected[i]) == 0;
    }

    return result;
}

#define init_strings(var, count) \
char *var[count]; \
for (int i = 0; i < count; i++) { var[i] = (char*)malloc(100); memset(var[i], 0, 100); }

#define destroy_strings(var, count) \
for (int i = 0; i < count; i++) { free(var[i]); }

void test_tasks_threshold()
{
    struct entity entities[2];

    entity_init(&entities[0], 1, A, 3, 2, 10);
    entity_init(&entities[1], 2, A, 3, 2, 10);

    entity_add_task(&entities[0], "e1t1", 2);
    entity_add_task(&entities[0], "e1t2", 1);
    entity_add_task(&entities[1], "e2t1", 2);
    entity_add_task(&entities[1], "e2t2", 4);

    int actual_size;
    init_strings(actual, 5);
    init_strings(expected, 2);
    make_standardized(expected[0], A, "e1t1");
    make_standardized(expected[1], A, "e1t2");

    process(entities, 2, actual, &actual_size);

    ASSERT(is_deep_equal(actual, actual_size, expected, 2));

    entity_destroy(&entities[0]);
    entity_destroy(&entities[1]);

    destroy_strings(actual, 5);
    destroy_strings(expected, 2);
}

void test_standard_names()
{
    struct entity entities[2];

    entity_init(&entities[0], 1, A, 3, 1, 10);
    entity_init(&entities[1], 2, B, 3, 1, 10);

    entity_add_task(&entities[0], "e1t1", 2);
    entity_add_task(&entities[1], "e2t1", 2);

    int actual_size;
    init_strings(actual, 5);
    init_strings(expected, 2);
    make_standardized(expected[0], A, "e1t1");
    make_standardized(expected[1], B, "e2t1");

    process(entities, 2, actual, &actual_size);

    ASSERT(is_deep_equal(actual, actual_size, expected, 2));

    entity_destroy(&entities[0]);
    entity_destroy(&entities[1]);

    destroy_strings(actual, 5);
    destroy_strings(expected, 2);
}

void test_waiting_threshold()
{
    struct entity entities[2];

    entity_init(&entities[0], 1, A, 3, 5, 3);
    entity_init(&entities[1], 2, A, 3, 5, 4);

    entity_add_task(&entities[0], "e1t1", 10);
    entity_add_task(&entities[1], "e2t1", 10);

    int actual_size;
    init_strings(actual, 5);
    init_strings(expected, 1);
    make_standardized(expected[0], A, "e1t1");

    process(entities, 2, actual, &actual_size);
    ASSERT(actual_size == 0);

    process(entities, 2, actual, &actual_size);
    ASSERT(actual_size == 0);

    process(entities, 2, actual, &actual_size);
    ASSERT(actual_size == 0);

    process(entities, 2, actual, &actual_size);
    ASSERT(is_deep_equal(actual, actual_size, expected, 1));

    entity_destroy(&entities[0]);
    entity_destroy(&entities[1]);

    destroy_strings(actual, 5);
    destroy_strings(expected, 1);
}

void test_critical_tasks()
{
    struct entity entities[2];

    entity_init(&entities[0], 1, A, 3, 2, 10);
    entity_init(&entities[1], 2, A, 3, 2, 10);

    entity_add_task(&entities[0], "e1t1", 1);
    entity_add_task(&entities[1], "e2t1", 2);

    int actual_size;
    init_strings(actual, 5);
    init_strings(expected, 1);
    make_standardized(expected[0], A, "e1t1");

    process(entities, 2, actual, &actual_size);
    ASSERT(actual_size == 0);

    process(entities, 2, actual, &actual_size);
    ASSERT(is_deep_equal(actual, actual_size, expected, 1));

    entity_destroy(&entities[0]);
    entity_destroy(&entities[1]);

    destroy_strings(actual, 5);
    destroy_strings(expected, 1);
}

int main()
{
    INIT();
    TEST(test_tasks_threshold);
    TEST(test_standard_names);
    TEST(test_waiting_threshold);
    TEST(test_critical_tasks);

    return 0;
}