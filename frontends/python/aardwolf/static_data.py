import ast
import os

from .writer import Writer
from .constants import *
from .utils import unique


class Stmt:
    def __init__(self, node, defs, uses):
        self.node_ = node
        self.succ_ = []
        self.defs_ = unique(defs)
        self.uses_ = unique(uses)

    def add_succ(self, node):
        self.succ_.append(node)

    def get_loc(self):
        if isinstance(self.node_, ast.If):
            return self._get_loc(self.node_.test)
        elif isinstance(self.node_, ast.For):
            return self._get_loc(self.node_.target)
        elif isinstance(self.node_, ast.While):
            return self._get_loc(self.node_.test)
        elif isinstance(self.node_, ast.withitem):
            return self._get_loc(self.node_.context_expr)
        else:
            return self._get_loc(self.node_)

    def _get_loc(self, node):
        return node.lineno, node.col_offset + 1, node.end_lineno, node.end_col_offset + 1

    def get_meta(self):
        if isinstance(self.node_, ast.arg):
            return META_ARG
        elif isinstance(self.node_, ast.Return):
            return META_RET
        elif isinstance(self.node_, ast.Call):
            return META_CALL
        else:
            return 0

    def write(self, analysis, writer):
        writer.write_token(TOKEN_STATEMENT)

        # Statement id
        self._write_stmt_id(analysis, writer, self.node_)

        # Successors
        writer.write_u8(len(self.succ_))
        for succ in self.succ_:
            self._write_stmt_id(analysis, writer, succ)

        # Defs
        writer.write_u8(len(self.defs_))
        for access in self.defs_:
            self._write_access(analysis, writer, access)

        # Uses
        writer.write_u8(len(self.uses_))
        for access in self.uses_:
            self._write_access(analysis, writer, access)

        # Location
        loc = self.get_loc()

        writer.write_u64(analysis.file_id_)
        writer.write_u32(loc[0])
        writer.write_u32(loc[1])
        writer.write_u32(loc[2])
        writer.write_u32(loc[3])

        # Metadata
        writer.write_u8(self.get_meta())

    def _write_stmt_id(self, analysis, writer, node):
        writer.write_u64(analysis.file_id_)
        writer.write_u64(analysis.nodes_.get(node))

    def _write_access(self, analysis, writer, access):
        if access.is_scalar():
            writer.write_token(TOKEN_VALUE_SCALAR)
            writer.write_u64(analysis.values_.get(access))
        elif access.is_structural():
            writer.write_token(TOKEN_VALUE_STRUCTURAL)
            self._write_access(analysis, writer, access.base_)
            self._write_access(analysis, writer, access.accessors_)
        elif access.is_array_like():
            writer.write_token(TOKEN_VALUE_ARRAY_LIKE)
            self._write_access(analysis, writer, access.base_)
            writer.write_u32(len(access.accessors_))
            for index in access.accessors_:
                self._write_access(analysis, writer, index)


class StaticData:
    def __init__(self, analysis):
        self.analysis_ = analysis

    def write(self, outdir=None):
        if outdir is None:
            outdir = os.getcwd()

        output = os.path.basename(self.analysis_.filename_) + '.aard'
        output = os.path.join(outdir, output)
        output = os.path.realpath(output)

        writer = Writer(output)
        writer.write_str('AARD/S1')

        for func, body in self.analysis_.ctx_store_.items():
            # Empty function
            if len(body[0]) == 0:
                continue

            writer.write_token(TOKEN_FUNCTION)
            writer.write_cstr(func)

            stmts = self._get_stmts(body)

            for stmt in stmts:
                stmt.write(self.analysis_, writer)

        writer.write_token(TOKEN_FILENAMES)
        writer.write_u32(1)
        writer.write_u64(self.analysis_.file_id_)
        writer.write_cstr(self.analysis_.filename_)
        writer.close()

    def _get_stmts(self, func_body):
        # Normalize the basic blocks first. It properly reconnect the edges of
        # empty basic blocks.
        for block in func_body:
            block.normalize()

        stmts = []
        for block in func_body:
            prev = None
            for node in block:
                defs = self.analysis_.defs_.get(node, [])
                uses = self.analysis_.uses_.get(node, [])

                stmt = Stmt(node, defs, uses)
                stmts.append(stmt)

                if prev is not None:
                    prev.add_succ(stmt.node_)

                prev = stmt

            if prev is not None:
                for succ in block.succ():
                    if len(succ) > 0:
                        prev.add_succ(succ.entry())
                    else:
                        assert len(list(succ.succ())) == 0

        return stmts
