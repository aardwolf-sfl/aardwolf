import difflib
import os
import re
import sys
import subprocess
import tempfile

# Add project root path to be searched for packages to properly load aardwolf
sys.path.append(os.path.realpath(
    os.path.join(os.path.dirname(__file__), '..')))

from aardwolf import process_file  # nopep8


def find_tests():
    root = os.path.realpath(os.path.dirname(__file__))

    tests = []
    for filename in os.listdir(root):
        fullpath = os.path.join(root, filename)
        _, ext = os.path.splitext(filename)

        if os.path.isfile(fullpath) and ext == '.py' and filename != '__init__.py':
            tests.append((fullpath, filename))

    return tests


def extract_annotations(filepath):
    prefix = '# AARD: '

    with open(filepath) as fh:
        annotations = []
        for line in fh.readlines():
            line = line.lstrip()
            if line.startswith(prefix):
                annotations.append(line[len(prefix):])

            if line.startswith('# AARD: SKIP'):
                return None

        return ''.join(annotations)[:-1]


def run_view(filepath):
    view_path = os.path.realpath(os.path.join(os.path.dirname(
        __file__), '..', '..', '..', 'tools', 'view.py'))

    tmpdir = tempfile.gettempdir()
    process_file(filepath, outdir=tmpdir)
    outfile = os.path.join(tmpdir, os.path.basename(filepath)) + '.aard'

    parsed = subprocess.run(f'python {view_path} {outfile}', capture_output=True, shell=True,
                            check=True, encoding='utf-8')

    return parsed.stdout


def normalize_actual(content):
    # Remove blank lines
    content = '\n'.join(filter(lambda line: line != '', content.splitlines()))

    # Normalize file IDs
    ids = [(match.group(1), match.group(2))
           for match in re.finditer(r'@(\d+) = (.+)', content)]
    for i, (id, filepath) in enumerate(ids):
        content = re.sub(f'@{id}', f'@{i + 1}', content)
        content = re.sub(f'#{id}', f'#{i + 1}', content)
        content = re.sub(os.path.dirname(filepath) + os.path.sep, '', content)

    return content


def compare(actual, expected):
    diff = ''.join(difflib.unified_diff(actual.splitlines(True),
                                        expected.splitlines(True), 'actual.aard', 'expected.aard'))
    return diff == '', diff


def main():
    passed = 0
    failed = 0
    skipped = 0

    display_diff = '--diff' in sys.argv
    display_actual = '--actual' in sys.argv

    for fullpath, filename in find_tests():
        actual = normalize_actual(run_view(fullpath))
        expected = extract_annotations(fullpath)

        if expected is None:
            skipped += 1
        else:
            equal, diff = compare(actual, expected)
            if equal:
                passed += 1
                print(f'PASS: {filename}')
            else:
                failed += 1
                print(f'FAIL: {filename}')

                if display_diff:
                    print()
                    print(diff)
                    print()

                if display_actual:
                    print()
                    print(actual)
                    print()

    print()
    print(f'passed: {passed}')
    print(f'failed: {failed}')

    if skipped > 0:
        print(f'skipped: {skipped}')

    if failed > 0 and not (display_diff or display_actual):
        print()
        print('There are failed test cases. To see what is wrong, execute this script with --diff or --actual.')


main()
