#include "runtime.h"

#include <stdlib.h>
#include <stdio.h>
#include <stdint.h>
#include <time.h>

#define FILE_FORMAT_VERSION 1

#define ASCII_ZERO 48
#define ASCII_NEWLINE 10

// Opened on the first API use. Closed after the process termination.
static FILE * __aardwolf_fd = NULL;

FILE * __aardwolf_get_fd(void)
{
    if (__aardwolf_fd == NULL) {
        // Initialization.
        char filename[15] = "aardwolf.xx.log";

        // This is called at most once.
        srand(time(NULL));
        uint8_t random = rand() % 100;

        // Replace `xx` with an actual id.
        filename[9] = random / 10 + ASCII_ZERO;
        filename[10] = random % 10 + ASCII_ZERO;

        __aardwolf_fd = fopen(filename, "w");

        // Print header.
        fputs("AARD", __aardwolf_fd);
        fputc(FILE_FORMAT_VERSION + ASCII_ZERO, __aardwolf_fd);
        fputc(ASCII_NEWLINE, __aardwolf_fd);
    }

    return __aardwolf_fd;
}

void __aardwolf_write_data(uint8_t token, void* data, size_t type_size)
{
    FILE *fd = __aardwolf_get_fd();
    fputc(token, fd);
    fwrite(data, type_size, 1, fd);
    fflush(fd);
}


void aardwolf_write_statement(statement_ref_t id)
{
    __aardwolf_write_data(TOKEN_STATEMENT, &id, sizeof(statement_ref_t));
}

void aardwolf_write_external(const char *external)
{
    FILE *fd = __aardwolf_get_fd();
    fputc(TOKEN_EXTERNAL, fd);
    fputs(external, fd);
    fputc(0, fd); // null terminator
    fflush(fd);
}

void aardwolf_write_data_i32(int32_t value)
{
    __aardwolf_write_data(TOKEN_DATA_I32, &value, sizeof(int32_t));
}

void aardwolf_write_data_i64(int64_t value)
{
    __aardwolf_write_data(TOKEN_DATA_I64, &value, sizeof(int64_t));
}

void aardwolf_write_data_f32(float value)
{
    __aardwolf_write_data(TOKEN_DATA_F32, &value, sizeof(float));
}

void aardwolf_write_data_f64(double value)
{
    __aardwolf_write_data(TOKEN_DATA_F64, &value, sizeof(double));
}
