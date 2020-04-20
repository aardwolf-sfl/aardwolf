#ifndef AARDWOLF_RUNTIME_H
#define AARDWOLF_RUNTIME_H

#include <stdint.h>

#define TOKEN_STATEMENT 0xff
#define TOKEN_EXTERNAL 0xfe
#define TOKEN_DATA_UNSUPPORTED 0x10
#define TOKEN_DATA_I8 0x11
#define TOKEN_DATA_I16 0x12
#define TOKEN_DATA_I32 0x13
#define TOKEN_DATA_I64 0x14
#define TOKEN_DATA_U8 0x15
#define TOKEN_DATA_U16 0x16
#define TOKEN_DATA_U32 0x17
#define TOKEN_DATA_U64 0x18
#define TOKEN_DATA_F32 0x19
#define TOKEN_DATA_F64 0x20
#define TOKEN_DATA_BOOL 0x21
#define TOKEN_DATA_NAMED 0x28
#define TOKEN_DATA_NULL 0x29

typedef uint64_t file_ref_t;
typedef uint64_t statement_ref_t;

// Log executed statement.
void aardwolf_write_statement(file_ref_t file_id, statement_ref_t stmt_id);

// Log external identifier. This is intended for differentiating individual test
// cases such that aardwolf can assign blocks of traces to these test cases
// and use this information along with test case status given later for the
// analysis.
void aardwolf_write_external(const char *external);

// Separated function for generating the header. It is called automatically in
// normal version of runtime. This should be only called when bare runtime is
// used and the file header must be generated explicitly.
void aardwolf_write_header();

// Only primitive types. It is the responsibility of the frontend to correctly
// write serialize complex types (e.g., arrays or structures).
//
// Before every data dump, there must be indication of what types they are.
// It cannot be done beforehand just once, because in dynamically-typed
// languages, the type of a variable can change.
void aardwolf_write_data_unsupported();
void aardwolf_write_data_i8(int8_t value);
void aardwolf_write_data_i16(int16_t value);
void aardwolf_write_data_i32(int32_t value);
void aardwolf_write_data_i64(int64_t value);
void aardwolf_write_data_u8(uint8_t value);
void aardwolf_write_data_u16(uint16_t value);
void aardwolf_write_data_u32(uint32_t value);
void aardwolf_write_data_u64(uint64_t value);
void aardwolf_write_data_f32(float value);
void aardwolf_write_data_f64(double value);
void aardwolf_write_data_bool(uint8_t value);
void aardwolf_write_data_named(const char *value);
void aardwolf_write_data_null();
// TODO: Others

#endif // AARDWOLF_RUNTIME_H
