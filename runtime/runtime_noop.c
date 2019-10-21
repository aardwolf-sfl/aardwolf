#include "runtime.h"

// An implementation of runtime interface with empty bodies. This may be useful
// for test cases which call `aardwolf_write_external` when just testing the
// code without using Aardwolf analysis.

void aardwolf_write_statement(statement_ref_t id) { }

void aardwolf_write_external(const char *external) { }

void aardwolf_write_data_i32(int32_t value) { }

void aardwolf_write_data_i64(int64_t value) { }

void aardwolf_write_data_f32(float value) { }

void aardwolf_write_data_f64(double value) { }
