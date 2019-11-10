#include "runtime.h"

#include <stdlib.h>
#include <stdio.h>
#include <stdint.h>
#include <string.h>

#define FILE_FORMAT_VERSION 1

#define ASCII_ZERO 48

// Opened on the first API use. Closed after the process termination.
static FILE * __aardwolf_fd = NULL;

FILE * __aardwolf_get_fd(void)
{
    if (__aardwolf_fd == NULL) {
        // Initialization.
        char *destination = getenv("AARDWOLF_DATA_DEST");
        // "execution-trace" is quite general name, we use exclamation mark
        // at the beginning as an attempt to prevent collisions with source
        // filenames.
        char filename[] = "!execution-trace.aard";
        char * filepath;

        // NOTE: sizeof(filename) includes null terminator as well.
        if (destination == NULL) {
            filepath = (char*)malloc(sizeof(filename));
            strcpy(filepath, filename);
        } else {
            size_t destination_length = strlen(destination) + 1;

            filepath = (char*)malloc(destination_length + sizeof(filename));
            memset(filepath, 0, destination_length + sizeof(filename));

            strcpy(filepath, destination);
            filepath[destination_length - 1] = '/';
            strcpy(filepath + destination_length, filename);
        }

        __aardwolf_fd = fopen(filepath, "w");

        // Print header.
        fputs("AARD/D", __aardwolf_fd);
        fputc(FILE_FORMAT_VERSION + ASCII_ZERO, __aardwolf_fd);

        free(filepath);
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
