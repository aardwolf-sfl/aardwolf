import functools
import types
import inspect
import re
import sys

from .runtime import write_external, write_test_status
from .utils import use_aardwolf


def wrap_test(f):
    if not use_aardwolf():
        return f

    @functools.wraps(f)
    def aardwolf_test_wrapper(*args, **kwargs):
        name = f.__name__
        write_external(name)

        try:
            output = f(*args, **kwargs)
            write_test_status(name, True)
            return output
        except Exception as ex:
            write_test_status(name, False)
            raise ex

    return aardwolf_test_wrapper


def wrap_module(module_name=None, starts_with=None, regex=None, ignore=None):
    pattern = '.+'
    if regex is not None:
        pattern = regex
    elif starts_with is not None:
        pattern = f'^{starts_with}.*'

    compiled = re.compile(pattern)

    if ignore is None:
        ignore = []

    if module_name is None:
        # Try to get the module by inspecting the stack frame of the caller.
        frame = inspect.stack()[1]
        module = inspect.getmodule(frame[0])
    else:
        # `wrap_module` can be called with `module_name=__name__` if needed
        # (presumably required only when another level of indirection is used on
        # top of `wrap_module`).
        module = sys.modules[module_name]

    # Iterate over members of the module to find functions to wrap. Note that
    # the `wrap_module` function must be called at the *end* of the module so it
    # is completely loaded and the members are retrieved properly.
    for name, item in inspect.getmembers(module):
        is_function = isinstance(item, types.FunctionType)
        satisfies_filter = name not in ignore and (
            compiled.search(name) is not None)

        if is_function and satisfies_filter:
            in_module = item.__module__ == module.__name__
            if in_module:
                setattr(module, name, wrap_test(item))
