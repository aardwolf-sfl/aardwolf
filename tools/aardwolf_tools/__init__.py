import sys

from .view import parse as parse_file
from .test import run_driver

__version__ = "0.1.0"


def eprint(message):
    print(message, file=sys.stderr)


def parse(args):
    if len(args) == 0:
        eprint('File to parse not specified.')
        exit(1)

    if len(args) > 1:
        eprint('Only one file to parse at a time is supported.')
        exit(1)

    print(parse_file(args[0]))


if __name__ == '__main__':
    commands = dict([(comm.__name__, comm) for comm in [parse]])

    if len(sys.argv) <= 1:
        eprint('Invalid usage, a command need to be specified as the first parameter.')
        exit(1)
    elif sys.argv[1] not in commands:
        eprint('Invalid command, available: ' +
               ', '.join(commands.keys()) + '.')
        exit(1)
