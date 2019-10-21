#ifndef AARDWOLF_RUNTIME_H
#define AARDWOLF_RUNTIME_H

#include <stdint.h>

#define TOKEN_STATEMENT 0xff
#define TOKEN_EXTERNAL 0xfe
#define TOKEN_DATA_I32 0x11
#define TOKEN_DATA_I64 0x12
#define TOKEN_DATA_F32 0x15
#define TOKEN_DATA_F64 0x16

typedef uint64_t statement_ref_t;

// Log executed statement.
void aardwolf_write_statement(statement_ref_t id);

// Log external identifier. This is intended for differentiating individual test
// cases such that aardwolf can assign blocks of traces to these test cases
// and use this information along with test case status given later for the
// analysis.
void aardwolf_write_external(const char *external);

// Only primitive types. It is the responsibility of the frontend to correctly
// write serialize complex types (e.g., arrays or structures).
//
// Before every data dump, there must be indication of what types they are.
// It cannot be done beforehand just once, because in dynamically-typed
// languages, the type of a variable can change.
void aardwolf_write_data_i32(int32_t value);
void aardwolf_write_data_i64(int64_t value);
void aardwolf_write_data_f32(float value);
void aardwolf_write_data_f64(double value);
// TODO: Others

#endif // AARDWOLF_RUNTIME_H
