#include "runtime.h"

#include <stdlib.h>
#include <stdio.h>
#include <stdint.h>
#include <string.h>

#define FILE_FORMAT_VERSION 1

#define ASCII_ZERO 48

// Opened on the first API use. Closed after the process termination.
static FILE * __aardwolf_fd = NULL;

void __write_header(FILE *fd)
{
    fputs("AARD/D", fd);
    fputc(FILE_FORMAT_VERSION + ASCII_ZERO, fd);
}

FILE * __aardwolf_get_fd(void)
{
    if (__aardwolf_fd == NULL) {
        char *dest_dir = getenv("AARDWOLF_DATA_DEST");

        char filename[] = "aard.trace";
        char * filepath;

        // NOTE: sizeof(filename) includes null terminator as well.
        if (dest_dir == NULL) {
            filepath = (char*)malloc(sizeof(filename));
            strcpy(filepath, filename);
        } else {
            size_t destination_length = strlen(dest_dir) + 1;

            filepath = (char*)malloc(destination_length + sizeof(filename));
            memset(filepath, 0, destination_length + sizeof(filename));

            strcpy(filepath, dest_dir);
            filepath[destination_length - 1] = '/';
            strcpy(filepath + destination_length, filename);
        }

#ifndef NO_HEADER
        __aardwolf_fd = fopen(filepath, "w");
#else
        __aardwolf_fd = fopen(filepath, "a");
#endif

        if (__aardwolf_fd == NULL) {
            fprintf(stderr, "Aardwolf error: cannot open %s.\n", filepath);
            free(filepath);
            exit(1);
        }

#ifndef NO_HEADER
        __write_header(__aardwolf_fd);
#endif

        free(filepath);
    }

    return __aardwolf_fd;
}

void __aardwolf_write_data(uint8_t token, void* data, size_t type_size)
{
#ifndef NO_DATA
    FILE *fd = __aardwolf_get_fd();
    fputc(token, fd);
    fwrite(data, type_size, 1, fd);
    fflush(fd);
#endif
}


void aardwolf_write_statement(statement_ref_t id)
{
    __aardwolf_write_data(TOKEN_STATEMENT, &id, sizeof(statement_ref_t));
}

void aardwolf_write_external(const char *external)
{
#ifndef NO_DATA
    FILE *fd = __aardwolf_get_fd();
    fseek(fd, 0, SEEK_END);
    fputc(TOKEN_EXTERNAL, fd);
    fputs(external, fd);
    fputc(0, fd); // null terminator
    fflush(fd);
#endif
}

void aardwolf_write_header()
{
    __write_header(__aardwolf_get_fd());
}

void aardwolf_write_data_unsupported()
{
    __aardwolf_write_data(TOKEN_DATA_UNSUPPORTED, NULL, 0);
}

void aardwolf_write_data_i8(int8_t value)
{
    __aardwolf_write_data(TOKEN_DATA_I8, &value, sizeof(int8_t));
}

void aardwolf_write_data_i16(int16_t value)
{
    __aardwolf_write_data(TOKEN_DATA_I16, &value, sizeof(int16_t));
}

void aardwolf_write_data_i32(int32_t value)
{
    __aardwolf_write_data(TOKEN_DATA_I32, &value, sizeof(int32_t));
}

void aardwolf_write_data_i64(int64_t value)
{
    __aardwolf_write_data(TOKEN_DATA_I64, &value, sizeof(int64_t));
}

void aardwolf_write_data_u8(uint8_t value)
{
    __aardwolf_write_data(TOKEN_DATA_U8, &value, sizeof(uint8_t));
}

void aardwolf_write_data_u16(uint16_t value)
{
    __aardwolf_write_data(TOKEN_DATA_U16, &value, sizeof(uint16_t));
}

void aardwolf_write_data_u32(uint32_t value)
{
    __aardwolf_write_data(TOKEN_DATA_U32, &value, sizeof(uint32_t));
}

void aardwolf_write_data_u64(uint64_t value)
{
    __aardwolf_write_data(TOKEN_DATA_U64, &value, sizeof(uint64_t));
}

void aardwolf_write_data_f32(float value)
{
    __aardwolf_write_data(TOKEN_DATA_F32, &value, sizeof(float));
}

void aardwolf_write_data_f64(double value)
{
    __aardwolf_write_data(TOKEN_DATA_F64, &value, sizeof(double));
}
