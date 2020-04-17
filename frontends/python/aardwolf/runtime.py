import os

from .writer import Writer
from .constants import *

WRITER = None


def _init_if_needed():
    global WRITER
    if WRITER is None:
        WRITER = Writer(os.path.join(os.environ.get(
            'AARDWOLF_DATA_DEST', os.getcwd()), 'aard.trace'))
        WRITER.write_str('AARD/D1')


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


def write_external(external):
    _init_if_needed()
    WRITER.write_token(TOKEN_EXTERNAL)
    WRITER.write_cstr(external)
