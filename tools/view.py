import sys
import os
import struct

input_file = sys.argv[-1]

HEADER_STATIC = b'AARD/S1'
HEADER_DYNAMIC = b'AARD/D1'

TOKEN_STATEMENT = b'\xff'
TOKEN_FUNCTION = TOKEN_EXTERNAL = b'\xfe'
TOKEN_FILENAMES = b'\xfd'

TOKEN_DATA_I32 = b'\x11'
TOKEN_DATA_I64 = b'\x12'
TOKEN_DATA_F32 = b'\x15'
TOKEN_DATA_F64 = b'\x16'

def read_stmt(f):
    id = struct.unpack('Q', f.read(8))[0]
    return f'#{id}'

def read_u8(f):
    return struct.unpack('B', f.read(1))[0]

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


def get_static_handlers():
    def _parse_stmt(f):
        stmt_id = read_stmt(f)
        
        n_succ = read_u8(f)
        succ_ids = ', '.join([read_stmt(f) for _ in range(n_succ)])

        n_defs = read_u8(f)
        defs = ', '.join([f'%{read_u64(f)}' for _ in range(n_defs)])

        n_uses = read_u8(f)
        uses = ', '.join([f'%{read_u64(f)}' for _ in range(n_uses)])

        n_loc = read_u8(f)
        loc = [str(read_u32(f)) for _ in range(n_loc)]
        loc[0] = f'@{loc[0]}'

        if n_loc == 2:
            loc.append('?')

        loc = ', '.join(loc)

        # TODO: Statement metadata
        read_u8(f)

        return f'{stmt_id} -> {succ_ids}  ::  defs: {defs} / uses: {uses} [{loc}]'

    def _parse_func(f):
        name = read_cstr(f)
        return f'\nfunction: {name}\n'

    def _parse_filenames(f):
        n_filenames = read_u32(f)
        filenames = '\n'.join([f'@{read_u32(f)} = {read_cstr(f)}' for _ in range(n_filenames)])
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
        TOKEN_DATA_I32: _prepend('i32', read_i32),
        TOKEN_DATA_I64: _prepend('i64', read_i64),
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
            print(f'invalid token identifier: {token_value}')
            exit(1)

        token = f.read(1)

if __name__ == '__main__':
    if not os.path.isfile(input_file):
        print('no such file or directory: {}'.format(input_file))
        exit(1)

    with open(input_file, 'rb') as f:
        parse(f)
