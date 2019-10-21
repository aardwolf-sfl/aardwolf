#include <stdio.h>
#include "fib.c"

#include "../../../runtime/runtime.h"

int main()
{
    aardwolf_write_external("fib");
    int f = fib(5);
    printf("%d\n", f);
    return 0;
}
