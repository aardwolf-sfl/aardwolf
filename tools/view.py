import sys
import os
import struct

input_file = sys.argv[-1]

HEADER_STATIC = b'AARD/S1'
HEADER_DYNAMIC = b'AARD/D1'

TOKEN_STATEMENT = b'\xff'
TOKEN_FUNCTION = TOKEN_EXTERNAL = b'\xfe'
TOKEN_FILENAMES = b'\xfd'

TOKEN_VALUE_SCALAR = b'\xe0'
TOKEN_VALUE_STRUCTURAL = b'\xe1'
TOKEN_VALUE_ARRAY_LIKE = b'\xe2'

META = 0x60
META_ARG = 0x61
META_RET = 0x62
META_CALL = 0x64

TOKEN_DATA_UNSUPPORTED = b'\x10'
TOKEN_DATA_I8 = b'\x11'
TOKEN_DATA_I16 = b'\x12'
TOKEN_DATA_I32 = b'\x13'
TOKEN_DATA_I64 = b'\x14'
TOKEN_DATA_U8 = b'\x15'
TOKEN_DATA_U16 = b'\x16'
TOKEN_DATA_U32 = b'\x17'
TOKEN_DATA_U64 = b'\x18'
TOKEN_DATA_F32 = b'\x19'
TOKEN_DATA_F64 = b'\x20'

def read_stmt(f):
    file_id = struct.unpack('Q', f.read(8))[0]
    stmt_id = struct.unpack('Q', f.read(8))[0]
    return f'#{file_id}:{stmt_id}'

def read_i8(f):
    return struct.unpack('b', f.read(1))[0]

def read_u8(f):
    return struct.unpack('B', f.read(1))[0]

def read_i16(f):
    return struct.unpack('h', f.read(2))[0]

def read_u16(f):
    return struct.unpack('H', f.read(2))[0]

def read_i32(f):
    return struct.unpack('i', f.read(4))[0]

def read_u32(f):
    return struct.unpack('I', f.read(4))[0]

def read_i64(f):
    return struct.unpack('q', f.read(8))[0]

def read_u64(f):
    return struct.unpack('Q', f.read(8))[0]

def read_f32(f):
    return struct.unpack('f', f.read(4))[0]

def read_f64(f):
    return struct.unpack('d', f.read(8))[0]

def read_cstr(f):
    result = ''
    byte = f.read(1)
    while byte != b'\0' and byte != 0 and byte is not None:
        result += chr(byte[0])
        byte = f.read(1)

    return result

def read_str(f):
    return f'"{read_cstr(f)}"'

def read_access(f):
    value_type = f.read(1)
    assert value_type in [TOKEN_VALUE_SCALAR, TOKEN_VALUE_STRUCTURAL, TOKEN_VALUE_ARRAY_LIKE], 'invalid value type'

    if value_type == TOKEN_VALUE_SCALAR:
        return f'%{read_u64(f)}'
    elif value_type == TOKEN_VALUE_STRUCTURAL:
        return f'{read_access(f)}.{read_access(f)}'
    elif value_type == TOKEN_VALUE_ARRAY_LIKE:
        base = read_access(f)
        count = read_u32(f)
        index = ', '.join(sorted([read_access(f) for _ in range(count)]))
        return f'{base}[{index}]'

def read_metadata(f):
    raw_metadata = read_u8(f)

    if raw_metadata & META:
        meta = []

        if (raw_metadata & ~META) == (META_ARG & ~META):
            meta.append('arg')

        if (raw_metadata & ~META) == (META_RET & ~META):
            meta.append('ret')

        if (raw_metadata & ~META) == (META_CALL & ~META):
            meta.append('call')

        joined = ', '.join(meta)
        return f'  {{ {joined} }}'
    else:
        return ''


def get_static_handlers():
    def _parse_stmt(f):
        stmt_id = read_stmt(f)

        n_succ = read_u8(f)
        succ_ids = ', '.join(sorted([read_stmt(f) for _ in range(n_succ)]))

        n_defs = read_u8(f)
        defs = ', '.join(sorted([read_access(f) for _ in range(n_defs)]))

        n_uses = read_u8(f)
        uses = ', '.join(sorted([read_access(f) for _ in range(n_uses)]))

        loc = f'@{read_u64(f)} {read_u32(f)}:{read_u32(f)}-{read_u32(f)}:{read_u32(f)}'

        metadata = read_metadata(f)

        return f'{stmt_id} -> {succ_ids}  ::  defs: {defs} / uses: {uses} [{loc}]{metadata}'

    def _parse_func(f):
        name = read_cstr(f)
        return f'\nfunction: {name}\n'

    def _parse_filenames(f):
        n_filenames = read_u32(f)
        filenames = '\n'.join([f'@{read_u64(f)} = {read_cstr(f)}' for _ in range(n_filenames)])
        return f'\n{filenames}'

    return {
        TOKEN_STATEMENT: _parse_stmt,
        TOKEN_FUNCTION: _parse_func,
        TOKEN_FILENAMES: _parse_filenames,
    }

def get_dynamic_handlers():
    def _prepend(prefix, handler):
        return lambda f: f'{prefix}: {handler(f)}'

    return {
        TOKEN_STATEMENT: _prepend('statement', read_stmt),
        TOKEN_EXTERNAL: _prepend('external', read_str),
        TOKEN_DATA_UNSUPPORTED: lambda f: 'unsupported data type',
        TOKEN_DATA_I8: _prepend('i8', read_i8),
        TOKEN_DATA_I16: _prepend('i16', read_i16),
        TOKEN_DATA_I32: _prepend('i32', read_i32),
        TOKEN_DATA_I64: _prepend('i64', read_i64),
        TOKEN_DATA_U8: _prepend('u8', read_u8),
        TOKEN_DATA_U16: _prepend('u16', read_u16),
        TOKEN_DATA_U32: _prepend('u32', read_u32),
        TOKEN_DATA_U64: _prepend('u64', read_u64),
        TOKEN_DATA_F32: _prepend('f32', read_f32),
        TOKEN_DATA_F64: _prepend('f64', read_f64),
    }

def parse(f):
    header = f.read(7)
    assert header == HEADER_STATIC or header == HEADER_DYNAMIC, "invalid header"

    handlers = get_static_handlers() if header == HEADER_STATIC else get_dynamic_handlers()

    token = f.read(1)
    while token is not None and token != 0 and len(token) == 1:
        if token in handlers:
            handler = handlers[token]
            print(handler(f))
        else:
            token_value = int.from_bytes(token, byteorder=sys.byteorder)
            print('invalid token identifier: 0x{:02x}'.format(token_value))
            exit(1)

        token = f.read(1)

if __name__ == '__main__':
    if not os.path.isfile(input_file):
        print('no such file or directory: {}'.format(input_file))
        exit(1)

    with open(input_file, 'rb') as f:
        parse(f)
