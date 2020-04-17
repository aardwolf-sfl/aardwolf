import ast
import symtable
import os

from .analysis import Analysis
from .static_data import StaticData
from .dynamic_data import Instrumenter

def _process(tree, symbols, outdir, filename):
    analysis = Analysis(symbols, filename)
    analysis.visit(tree)
    StaticData(analysis).write(outdir)
    tree = Instrumenter(analysis).visit(tree)
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

