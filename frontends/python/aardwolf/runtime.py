import os
import sys
import copy

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


# Accessor tree is in "S-expression" form. Examples:
# a = foo -> write_value(value, [])
# a, b = foo -> write_value(value, [[], []])
# a, (b, c) = foo -> write_value(value, [[], [[], []]])
def write_value(value, accessor_tree=None):
    _init_if_needed()

    if accessor_tree is None:
        accessor_tree = []

    for v in unpack_values(value, accessor_tree):
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
                WRITER.write_token(TOKEN_DATA_F64)
                WRITER.write_f64(v)
            elif sys.float_info.max == 3.4028235e+38:
                WRITER.write_token(TOKEN_DATA_F32)
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


class NonSubscriptable:
    def __getitem__(self, key):
        return None


def unpack_values(value, tree):
    if len(tree) == 0:
        if isinstance(tree, tuple):
            # *rest unpacking. For now, assume unsupported data type for now.
            return [None]
        else:
            return [value]
    else:
        # Ensure that we can use value[index].
        if not hasattr(value, '__getitem__'):
            if hasattr(value, '__iter__'):
                # Iterators are problematic. We cannot iterate through them
                # since that would make the original program invalid. We could
                # theoretically make a deep copy, but that has some issues on
                # its own (recursive data structure, IO resources, etc.). One
                # possibility is to wrap the iterator into our class but we need
                # to make sure that it does not break things like `isinstance`,
                # `getattr`, etc.
                value = NonSubscriptable()
            else:
                value = NonSubscriptable()

        output = []
        for index, node in enumerate(tree):
            output.extend(unpack_values(value[index], node))

        return output


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
