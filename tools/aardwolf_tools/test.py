import os
import sys
import re
import difflib


def find_tests(root, extension, ignore=None):
    if ignore is None:
        ignore = []

    tests = []
    for filename in os.listdir(root):
        fullpath = os.path.join(root, filename)
        _, ext = os.path.splitext(filename)

        if os.path.isfile(fullpath) and ext == extension and filename not in ignore:
            tests.append(fullpath)

    return tests


def extract_annotations(filename, annotations_prefix):
    prefix = annotations_prefix + 'AARD: '

    with open(filename) as fh:
        annotations = []
        for line in fh.readlines():
            line = line.lstrip()
            if line.startswith(prefix):
                annotations.append(line[len(prefix):])

            if line.startswith(annotations_prefix + 'AARD: SKIP'):
                return None

        return ''.join(annotations)[:-1]


def normalize_data(content):
    # Remove blank lines
    content = '\n'.join(filter(lambda line: line != '', content.splitlines()))

    # Normalize file IDs in static data
    ids = [(match.group(1), match.group(2))
           for match in re.finditer(r'@(\d+) = (.+)', content)]
    for i, (id, filepath) in enumerate(ids):
        content = re.sub(f'@{id}', f'@{i + 1}', content)
        content = re.sub(f'#{id}', f'#{i + 1}', content)
        content = re.sub(os.path.dirname(filepath) + os.path.sep, '', content)

    # Normalize file IDs in dynamic data
    ids = [match.group(1) for match in re.finditer(r'statement: #(\d+):\d+', content)]
    for i, id in enumerate(ids):
        content = re.sub(f'#{id}', f'#{i + 1}', content)

    return content


def compare(actual, expected):
    diff = ''.join(difflib.unified_diff(actual.splitlines(True),
                                        expected.splitlines(True), 'actual.aard', 'expected.aard'))
    return diff == '', diff


def run_driver(test_files, process_source, annotations_prefix):
    passed = 0
    failed = 0
    skipped = 0

    display_diff = '--diff' in sys.argv
    display_actual = '--actual' in sys.argv

    for filename in test_files:
        basename = os.path.basename(filename)

        expected = extract_annotations(filename, annotations_prefix)

        if expected is None:
            skipped += 1
            print(f'SKIP: {basename}')
        else:
            actual = normalize_data(process_source(filename))
            equal, diff = compare(actual, expected)
            if equal:
                passed += 1
                print(f'PASS: {basename}')
            else:
                failed += 1
                print(f'FAIL: {basename}')

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

    return failed == 0
