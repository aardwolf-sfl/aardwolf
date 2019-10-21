import sys
import os
import struct

input_file = sys.argv[-1]

HEADER = b'AARD1\n'
TOKEN_STATEMENT = b'\xff'
TOKEN_EXTERNAL = b'\xfe'
TOKEN_DATA_I32 = b'\x11'
TOKEN_DATA_I64 = b'\x12'
TOKEN_DATA_F32 = b'\x15'
TOKEN_DATA_F64 = b'\x16'

def read_stmt(f):
    id = struct.unpack('Q', f.read(8))[0]
    return f'#{id}'

def read_i32(f):
    return struct.unpack('i', f.read(4))[0]

def read_i64(f):
    return struct.unpack('q', f.read(8))[0]

def read_f32(f):
    return struct.unpack('f', f.read(4))[0]

def read_f64(f):
    return struct.unpack('d', f.read(8))[0]

def read_str(f):
    result = ''
    byte = f.read(1)
    while byte != b'\0' and byte != 0 and byte is not None:
        result += chr(byte[0])
        byte = f.read(1)
    
    return f'"{result}"'

def parse(f):
    header = f.read(6)
    assert header == HEADER, "invalid header"

    handlers = {
        TOKEN_STATEMENT: ('statement', read_stmt),
        TOKEN_EXTERNAL: ('external', read_str),
        TOKEN_DATA_I32: ('i32', read_i32),
        TOKEN_DATA_I64: ('i64', read_i64),
        TOKEN_DATA_F32: ('f32', read_f32),
        TOKEN_DATA_F64: ('f64', read_f64),
    }

    token = f.read(1)
    while token is not None and token != 0 and len(token) == 1:
        if token in handlers:
            handler = handlers[token]
            print(f'{handler[0]}: {handler[1](f)}')
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
