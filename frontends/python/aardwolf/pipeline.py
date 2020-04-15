import ast
import symtable
import os

from .static_data import StaticDataAnalyzer
from .dynamic_data import DynamicDataInstrumenter

def _process(tree, symbols, outdir, filename):
    StaticDataAnalyzer(symbols, outdir, filename).visit(tree)
    tree = DynamicDataInstrumenter().visit(tree)
    return tree

def process_str(source, outdir=None, filename='<string>', mode='exec'):
    tree = ast.parse(source, filename)
    symbols = symtable.symtable(source, filename, mode)

    if outdir is None:
        outdir = os.getcwd()

    tree = _process(tree, symbols, outdir, filename)
    return compile(tree, filename, mode)

def process_file(filename, outdir=None, mode='exec'):
    with open(filename) as fh:
        return process_str(fh.read(), outdir, filename, mode)

