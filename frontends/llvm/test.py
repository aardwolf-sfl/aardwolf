import sys
import os
import subprocess
import tempfile

# Add aardwolf_tools path to sys.path as a temporary "dependency resolution
# solution".
sys.path.append(os.path.realpath(
    os.path.join(os.path.dirname(__file__), '..', '..', 'tools')))

import aardwolf_tools  # nopep8


root = os.path.realpath(os.path.dirname(__file__))
frontend = os.path.join(root, 'build', 'lib', 'libAardwolfLLVM.so')
tests = os.path.join(root, 'tests')


def change_ext(filename, ext):
    name, _ = os.path.splitext(filename)
    return name + ext


def extract_opt_level(filename):
    with open(filename) as fh:
        for line in fh.readlines():
            line = line.lstrip()

            if line.startswith('// OPT: -O1'):
                return '-O1'

            if line.startswith('// OPT: -O2'):
                return '-O2'

            if line.startswith('// OPT: -O3'):
                return '-O3'

        return '-O0'


def process(filename):
    tmpdir = tempfile.gettempdir()

    obj_file = change_ext(filename, '.o')
    opt_level = extract_opt_level(filename)
    clang = f'clang -Xclang -load -Xclang {frontend} -c -g {opt_level} -o {obj_file} {filename}'
    subprocess.run(clang, shell=True, cwd=tmpdir, check=True)

    outfile = os.path.join(tmpdir, os.path.basename(filename)) + '.aard'

    parsed = aardwolf_tools.parse_file(outfile)

    return parsed


aardwolf_tools.run_driver(
    test_files=aardwolf_tools.find_tests(tests, '.c'),
    process_source=process,
    annotations_prefix='// ')
