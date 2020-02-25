#include "runtime.h"

// An implementation of runtime interface with empty bodies. This may be useful
// for test cases which call `aardwolf_write_external` when just testing the
// code without using Aardwolf analysis.

void aardwolf_write_statement(statement_ref_t id) { }

void aardwolf_write_external(const char *external) { }

void aardwolf_write_data_unsupported();

void aardwolf_write_data_i8(int8_t value) { }

void aardwolf_write_data_i16(int16_t value) { }

void aardwolf_write_data_i32(int32_t value) { }

void aardwolf_write_data_i64(int64_t value) { }

void aardwolf_write_data_u8(uint8_t value) { }

void aardwolf_write_data_u16(uint16_t value) { }

void aardwolf_write_data_u32(uint32_t value) { }

void aardwolf_write_data_u64(uint64_t value) { }

void aardwolf_write_data_f32(float value) { }

void aardwolf_write_data_f64(double value) { }
