#ifndef AARDWOLF_EXAMPLE_FRAMEWORK_H
#define AARDWOLF_EXAMPLE_FRAMEWORK_H

#include <assert.h>
#include <stdio.h>

#include "../../../runtime/runtime.h"

typedef void (test_fn)(void);

int __GLOBAL_STATUS = 0;

void test(const char *name, test_fn *fn)
{
    __GLOBAL_STATUS = 1;
    aardwolf_write_external(name);
    fn();
    printf("%s: %s\n", name, __GLOBAL_STATUS ? "OK" : "FAIL");
}

#define TEST(fn) test(#fn, &fn)
// Important that the test case ends right after a failed assertion, because
// Aardwolf will take the last recorded statement as the surely invalid value.
#define ASSERT(expr) __GLOBAL_STATUS = __GLOBAL_STATUS && (expr) ; if (!__GLOBAL_STATUS) return

#endif // AARDWOLF_EXAMPLE_FRAMEWORK_H
