import difflib
import os
import re
import sys
import subprocess
import tempfile
import aardwolf_tools

# Add project root path to be searched for packages to properly load aardwolf
sys.path.append(os.path.realpath(
    os.path.join(os.path.dirname(__file__), '..')))

import aardwolf  # nopep8


def process_analysis(filename):
    tmpdir = tempfile.gettempdir()

    aardwolf.process_file(filename, outdir=tmpdir)
    outfile = os.path.join(tmpdir, os.path.basename(filename)) + '.aard'

    parsed = aardwolf_tools.parse_file(outfile)

    return parsed


def process_trace(filename):
    tmpdir = tempfile.gettempdir()

    def __print(*args):
        with open(os.devnull, 'w') as devnull:
            return print(*args, file=devnull)

    processed = aardwolf.process_file(filename, outdir=tmpdir)
    # Some globals need to be passed to `exec` always, even if it was an empty
    # dict! We now replace built-in print with that which discards all its
    # arguments.
    exec(processed, {'print': __print})
    outfile = os.path.join(tmpdir, 'aard.trace')

    parsed = aardwolf_tools.parse_file(outfile)

    # Reset the file handle so the next test will create a new one
    aardwolf.runtime.WRITER = None

    return parsed


root = os.path.realpath(os.path.dirname(__file__))
analysis = os.path.join(root, 'analysis')
trace = os.path.join(root, 'trace')

# Run analysis tests
print('>>> ANALYSIS')
print()

aardwolf_tools.run_driver(
    test_files=aardwolf_tools.find_tests(
        analysis, '.py', ignore=['__init__.py']),
    process_source=process_analysis,
    annotations_prefix='# ')

# Run trace tests
print()
print()
print('>>> TRACE')
print()
aardwolf_tools.run_driver(
    test_files=aardwolf_tools.find_tests(
        trace, '.py', ignore=['__init__.py']),
    process_source=process_trace,
    annotations_prefix='# ')
