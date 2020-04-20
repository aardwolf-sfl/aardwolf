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


def find_tests():
    root = os.path.realpath(os.path.dirname(__file__))

    tests = []
    for filename in os.listdir(root):
        fullpath = os.path.join(root, filename)
        _, ext = os.path.splitext(filename)

        if os.path.isfile(fullpath) and ext == '.py' and filename != '__init__.py':
            tests.append(fullpath)

    return tests


def process(filename):
    tmpdir = tempfile.gettempdir()

    aardwolf.process_file(filename, outdir=tmpdir)
    outfile = os.path.join(tmpdir, os.path.basename(filename)) + '.aard'

    parsed = aardwolf_tools.parse_file(outfile)

    return parsed


root = os.path.realpath(os.path.dirname(__file__))

aardwolf_tools.run_driver(
    test_files=aardwolf_tools.find_tests(root, '.py', ignore=['__init__.py']),
    process_source=process,
    annotations_prefix='# ')
