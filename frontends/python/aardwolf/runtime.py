import os
import sys

from .writer import Writer
from .constants import *

WRITER = None
TEST_RESULTS = None


def _init_if_needed():
    global WRITER
    if WRITER is None:
        WRITER = Writer(os.path.join(os.environ.get(
            DATA_DEST, os.getcwd()), 'aard.trace'))
        WRITER.write_str('AARD/D1')


def _init_test_results_if_needed():
    global TEST_RESULTS
    if TEST_RESULTS is None:
        TEST_RESULTS = open(os.path.join(os.environ.get(
            DATA_DEST, os.getcwd()), 'aard.result'), 'w')


def write_stmt(id):
    _init_if_needed()
    WRITER.write_token(TOKEN_STATEMENT)
    WRITER.write_u64(id[0])
    WRITER.write_u64(id[1])


def write_expr(result, id):
    _init_if_needed()
    write_stmt(id)
    return result


def write_lazy(expr, id):
    _init_if_needed()
    write_stmt(id)
    return expr()


def write_value(value, accessors=None):
    _init_if_needed()

    if accessors is None:
        accessors = [lambda v: v]

    for accessor in accessors:
        v = accessor(value)

        if isinstance(v, bool):
            # This check needs to be *before* int check because bool is a subtype of
            # int.
            WRITER.write_token(TOKEN_DATA_BOOL)
            WRITER.write_u8(int(v))

        elif isinstance(v, int):
            # We consider all int values to be of type i64 because we have no
            # information of their actual type. Using `bit_length` may cause
            # unnecessary type changes in the runtime. Integers have unlimited
            # precision, if the v does not fit into 8 bytes, it is unsupported
            # by Aardwolf.
            try:
                _ = v.to_bytes(8, byteorder=sys.byteorder)
                WRITER.write_token(TOKEN_DATA_I64)
                WRITER.write_i64(v)
            except OverflowError:
                WRITER.write_token(TOKEN_DATA_UNSUPPORTED)

        elif isinstance(v, float):
            if sys.float_info.max == 1.7976931348623157e+308:
                WRITER.write_f64(v)
            elif sys.float_info.max == 3.4028235e+38:
                WRITER.write_f32(v)
            else:
                # Something ridiculous is happening
                WRITER.write_token(TOKEN_DATA_UNSUPPORTED)

        # Not yet supported by the core:
        # elif v is None:
        #     WRITER.write_token(TOKEN_DATA_NULL)

        # else:
        #     WRITER.write_token(TOKEN_DATA_NAMED)
        #     WRITER.write_cstr(type(v).__name__)

        else:
            WRITER.write_token(TOKEN_DATA_UNSUPPORTED)

    return value

def aardwolf_iter(iter, id, accessors=None):
    return AardwolfIter(iter, id, accessors)

class AardwolfIter:
    def __init__(self, inner, id, accessors):
        self.inner_ = iter(inner)
        self.id_ = id
        self.accessors_ = accessors

    def __iter__(self):
        return self

    def __next__(self):
        value = next(self.inner_)
        write_stmt(self.id_)
        write_value(value, self.accessors_)
        return value


def write_external(external):
    _init_if_needed()
    WRITER.write_token(TOKEN_EXTERNAL)
    WRITER.write_cstr(external)


def write_test_status(name, passed):
    _init_test_results_if_needed()
    status = 'PASS' if passed else 'FAIL'
    TEST_RESULTS.write(f'{status}: {name}\n')
