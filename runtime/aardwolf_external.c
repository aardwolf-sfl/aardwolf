#include <stdio.h>

#include "runtime.h"

int main(int argc, char *argv[])
{
    if (argc == 1) {
        aardwolf_write_header();
    } else if (argc == 2) {
        aardwolf_write_external(argv[1]);
        return 0;
    } else {
        fprintf(stderr, "invalid usage of aardwolf_external");
        return 1;
    }
}
