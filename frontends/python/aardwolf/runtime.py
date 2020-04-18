import os

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


def write_external(external):
    _init_if_needed()
    WRITER.write_token(TOKEN_EXTERNAL)
    WRITER.write_cstr(external)


def write_test_status(name, passed):
    _init_test_results_if_needed()
    status = 'PASS' if passed else 'FAIL'
    TEST_RESULTS.write(f'{status}: {name}\n')
