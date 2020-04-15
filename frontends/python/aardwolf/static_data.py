import ast
import symtable
import os

from .writer import Writer
from .constants import *
from .shared import Access
from .utils import unique


class StaticDataAnalyzer(ast.NodeVisitor):
    def __init__(self, symbols, outdir, filename):
        self.symbols_ = symbols
        self.filename_ = filename

        output = os.path.basename(filename) + '.aard'
        output = os.path.join(outdir, output)
        output = os.path.realpath(output)
        self.output_ = output
        self.writer_ = None

        try:
            self.file_id_ = os.stat(filename).st_ino
        except:
            self.file_id_ = 0

        self.stmts_ = {}
        self.values_ = {}
        self.links_ = {}
        self.parent_ = None

        self.class_stack_ = []
        self.func_stack_ = []
        self.postponed_ = []

    # === Data management ===

    def _get_stmt_id(self, node):
        if node not in self.stmts_:
            index = len(self.stmts_) + 1
            self.stmts_[node] = index

        return (self.file_id_, self.stmts_[node])

    def _get_value_id(self, name):
        if name not in self.values_:
            index = len(self.values_) + 1
            self.values_[name] = index

        return self.values_[name]

    def _connect(self, nodes, prev=None, next=None):
        for node in nodes:
            if prev is not None:
                link = self._link_safe(node)
                if isinstance(prev, list):
                    for p in prev:
                        self._link_safe(p).add_next(link)
                else:
                    self._link_safe(prev).add_next(link)

            prev = node

        if next is not None:
            link = self._link_safe(prev)
            if isinstance(next, list):
                for n in next:
                    link.add_next(self._link_safe(n))
            else:
                link.add_next(self._link_safe(next))

    def _link_safe(self, node):
        if node not in self.links_:
            self.links_[node] = Link(node)

        return self.links_[node]

    # === Writing ===

    def _write_stmt_id(self, stmt_id):
        self.writer_.write_u64(stmt_id[0])
        self.writer_.write_u64(stmt_id[1])

    def _write_stmt(self, node, defs=None, uses=None, loc=None, meta=None):
        if len(self.func_stack_) == 0:
            self.postponed_.append({
                'node': node,
                'defs': defs,
                'uses': uses,
                'loc': loc,
                'meta': meta,
            })
            return

        self.writer_.write_token(TOKEN_STATEMENT)

        # Statement id
        stmt_id = self._get_stmt_id(node)
        self._write_stmt_id(stmt_id)

        # Successors
        next = unique(self._link_safe(node).next())
        self.writer_.write_u8(len(next))
        for succ in next:
            succ_id = self._get_stmt_id(succ)
            self._write_stmt_id(succ_id)

        # Defs
        if defs is None:
            self.writer_.write_u8(0)
        else:
            defs = unique(defs)
            self.writer_.write_u8(len(defs))
            for access in defs:
                self._write_access(access)

        # Uses
        if uses is None:
            self.writer_.write_u8(0)
        else:
            uses = unique(uses)
            self.writer_.write_u8(len(uses))
            for access in uses:
                self._write_access(access)

        # Location
        if loc is None:
            loc = node

        self.writer_.write_u64(self.file_id_)
        self.writer_.write_u32(loc.lineno)
        self.writer_.write_u32(loc.col_offset + 1)
        self.writer_.write_u32(loc.end_lineno)
        self.writer_.write_u32(loc.end_col_offset + 1)

        # Metadata
        if meta is None:
            self.writer_.write_u8(0)
        else:
            self.writer_.write_u8(meta)

    def _write_access(self, access):
        if access.is_scalar():
            self.writer_.write_token(TOKEN_VALUE_SCALAR)
            self.writer_.write_u64(self._get_value_id(access.value_))
        elif access.is_structural():
            self.writer_.write_token(TOKEN_VALUE_STRUCTURAL)
            self._write_access(access.base_)
            self._write_access(access.accessors_)
        elif access.is_array_like():
            self.writer_.write_token(TOKEN_VALUE_ARRAY_LIKE)
            self._write_access(access.base_)
            self.writer_.write_u32(len(access.accessors_))
            for index in access.accessors_:
                self._write_access(index)

    # === Visitor ===

    def visit_Module(self, node):
        self.writer_ = Writer(self.output_)
        self.writer_.write_str('AARD/S1')

        self._connect(node.body)
        self.generic_visit(node)

        if len(self.postponed_) > 0:
            self.writer_.write_token(TOKEN_FUNCTION)
            self.writer_.write_cstr("__main__")
            self.func_stack_.append('__main__')

            for postponed in self.postponed_:
                self._write_stmt(postponed['node'], defs=postponed['defs'],
                                 uses=postponed['uses'], loc=postponed['loc'], meta=postponed['meta'])

            self.func_stack_.pop()

        # Export metadata
        self.writer_.write_token(TOKEN_FILENAMES)
        self.writer_.write_u32(1)
        self.writer_.write_u64(self.file_id_)
        self.writer_.write_cstr(self.filename_)

        self.writer_.close()

    # Function declaration
    def visit_FunctionDef(self, node):
        prefix = ''

        if len(self.class_stack_) > 0:
            prefix += '::'.join(self.class_stack_) + '::'

        if len(self.func_stack_) > 0:
            prefix += '::'.join(self.func_stack_) + '::'

        self.writer_.write_token(TOKEN_FUNCTION)
        self.writer_.write_cstr(prefix + node.name)

        self.func_stack_.append(node.name)
        symbols = self.symbols_
        self.symbols_ = symbols.lookup(node.name).get_namespace()

        # Export args as statements
        for arg in node.args.args:
            defs = [Access.scalar(self.symbols_.lookup(arg.arg))]
            self._write_stmt(arg, defs=defs, meta=META_ARG)

        # Use next=node as artificial exit node
        self._connect(node.body, next=node)

        self.generic_visit(node)

        self.func_stack_.pop()
        self.symbols_ = symbols

    # Class declaration (aka namespace)
    def visit_ClassDef(self, node):
        self.class_stack_.append(node.name)
        symbols = self.symbols_
        self.symbols_ = symbols.lookup(node.name).get_namespace()

        self.generic_visit(node)

        self.class_stack_.pop()
        self.symbols_ = symbols

    # === Statements ===

    def visit_Expr(self, node):
        # Expression statement (with its return value not used or stored)
        if isinstance(node.value, ast.Call):
            # Calling a function as a standalone statement.
            self._visit_call(node.value, node)
        else:
            self.generic_visit(node)

    def visit_Call(self, node):
        if self.parent_ is not None:
            # Set proper control flow (calling the function before the
            # containing expression).
            self._link_safe(self.parent_).prepend(self._link_safe(node))

        # Calling a function inside an expression.
        self._visit_call(node, node)

    def _visit_call(self, node, container):
        self.parent_ = container
        self.generic_visit(node)
        self.parent_ = None

        defs = [get_value_access(node, self.symbols_)]
        uses = []

        for arg in node.args:
            uses.extend(get_uses(arg, self.symbols_))

        for arg in node.keywords:
            uses.extend(get_uses(arg.value, self.symbols_))

        self._write_stmt(container, defs=defs, uses=uses, meta=META_CALL)

    def visit_Assign(self, node):
        self.parent_ = node
        self.generic_visit(node)
        self.parent_ = None

        # TODO: Tuple destructuring.
        defs = [get_value_access(target, self.symbols_)
                for target in node.targets]
        uses = get_uses(node.value, self.symbols_)
        self._write_stmt(node, defs=defs, uses=uses)

    # def visit_AnnAssign(self, node):

    def visit_AugAssign(self, node):
        self.parent_ = node
        self.generic_visit(node)
        self.parent_ = None

        defs = [get_value_access(node.target, self.symbols_)]
        uses = get_uses(node.target, self.symbols_)
        uses.extend(get_uses(node.value, self.symbols_))
        self._write_stmt(node, defs=defs, uses=uses)

    def visit_Raise(self, node):
        self.parent_ = node
        self.generic_visit(node)
        self.parent_ = None

        uses = get_uses(node, self.symbols_)
        self._write_stmt(node, uses=uses)

    def visit_Assert(self, node):
        self.parent_ = node
        self.generic_visit(node)
        self.parent_ = None

        uses = get_uses(node, self.symbols_)
        self._write_stmt(node, uses=uses)

    def visit_Delete(self, node):
        self.parent_ = node
        self.generic_visit(node)
        self.parent_ = None

        uses = get_uses(node, self.symbols_)
        self._write_stmt(node, uses=uses)

    # def visit_Pass(self, node)

    def visit_If(self, node):
        self.parent_ = node.test
        self.visit(node.test)
        self.parent_ = None

        link = self._link_safe(node)
        next = link.next()
        link.unlink()
        self._connect(node.body, prev=node, next=next)
        self._connect(node.orelse, prev=node, next=next)

        uses = get_uses(node.test, self.symbols_)
        self._write_stmt(node, uses=uses, loc=node.test)

        for stmt in node.body:
            self.visit(stmt)

        for stmt in node.orelse:
            self.visit(stmt)

    def visit_For(self, node):
        call_next = ast.Call(
            func=ast.Name(id='next', ctx=ast.Load()),
            args=[node.iter],
            keywords=[]
        )
        init_post = ast.copy_location(ast.Expr(value=call_next), node.iter)
        ast.fix_missing_locations(init_post)

        link = self._link_safe(node)
        next = link.next()
        prev = link.prev()
        link.unlink()

        self.parent_ = init_post
        self.visit(init_post)
        self.parent_ = None

        init_post_link = self._link_safe(init_post)
        pred = init_post_link.prev()
        if len(pred) == 0:
            pred = [init_post]

        self._connect(node.body, prev=node, next=pred)
        self._connect(node.orelse, prev=node, next=next)

        for p in prev:
            self._link_safe(p).replace(link, self._link_safe(pred[0]))

        init_post_link.add_next(link)

        defs = [get_value_access(node.target, self.symbols_)]
        uses = get_uses(init_post, self.symbols_)
        self._write_stmt(node, defs=defs, uses=uses, loc=node.target)

        for stmt in node.body:
            self.visit(stmt)

        for stmt in node.orelse:
            self.visit(stmt)

    def visit_While(self, node):
        self.parent_ = node.test
        self.visit(node.test)
        self.parent_ = None

        link = self._link_safe(node)
        next = link.next()
        link.unlink()
        self._connect(node.body, prev=node, next=node)
        self._connect(node.orelse, prev=node, next=next)

        uses = get_uses(node.test, self.symbols_)
        self._write_stmt(node, uses=uses, loc=node.test)

        for stmt in node.body:
            self.visit(stmt)

        for stmt in node.orelse:
            self.visit(stmt)

    def visit_Break(self, node):
        self.generic_visit(node)
        self._write_stmt(node)

    def visit_Continue(self, node):
        self.generic_visit(node)
        self._write_stmt(node)

    # TODO: Try, etc.

    def visit_With(self, node):
        link = self._link_safe(node)
        next = link.next()
        link.unlink()
        self._connect(node.body, prev=node, next=next)

        for item in node.items:
            self.parent_ = node.context_expr
            self.visit(node.context_expr)
            self.parent_ = None

            if item.optional_vars is not None:
                # TODO: Tuple/List destructuring.
                defs = get_value_access(item.optional_vars, self.symbols_)
            else:
                defs = []

            uses = get_uses(item.context_expr, self.symbols_)
            self._write_stmt(item.context_expr, defs=defs, uses=uses)

        for stmt in node.body:
            self.visit(stmt)

    def visit_Return(self, node):
        self.parent_ = node
        self.generic_visit(node)
        self.parent_ = None

        uses = get_uses(node.value, self.symbols_)
        self._write_stmt(node, uses=uses, meta=META_RET)

    def visit_Yield(self, node):
        self.parent_ = node
        self.generic_visit(node)
        self.parent_ = None

        uses = get_uses(node.value, self.symbols_)
        self._write_stmt(node, uses=uses, meta=META_RET)

    def visit_YieldFrom(self, node):
        self.parent_ = node
        self.generic_visit(node)
        self.parent_ = None

        uses = get_uses(node.value, self.symbols_)
        self._write_stmt(node, uses=uses, meta=META_RET)


def get_value_access(node, symbols):
    visitor = ValueAccessVisitor(symbols)
    visitor.visit(node)
    # print(f'get_value_access {node} {visitor.access_}')
    # import traceback
    # traceback.print_stack(limit=3)
    return visitor.access_


class ValueAccessVisitor(ast.NodeVisitor):
    def __init__(self, symbols):
        self.access_ = None
        self.symbols_ = symbols
        self.complex_ = False

    def visit_Name(self, node):
            try:
                self.access_ = Access.scalar(self.symbols_.lookup(node.id))
            except KeyError as e:
                self.access_ = Access.scalar(node.id)

    def visit_Call(self, node):
        self.visit(node.func)
        assert self.access_ is not None

        fullname = f'{self.access_}:{node.lineno}:{node.col_offset}'
        self.access_ = Access.scalar(fullname)

    def visit_Attribute(self, node):
        self.complex_ = True
        self.visit(node.value)
        field = Access.scalar(node.attr)
        self.access_ = Access.structural(self.access_, field)

    def visit_Subscript(self, node):
        self.complex_ = True
        self.visit(node.value)

        if isinstance(node.slice, ast.ExtSlice):
            index = [access for access in self._handle_slice(
                slice) for slice in node.slice.dims]
        else:
            index = self._handle_slice(node.slice)

        self.access_ = Access.array_like(self.access_, _filter_none(index))

    def _handle_slice(self, slice):
        if isinstance(slice, ast.Index):
            return [get_value_access(slice.value, self.symbols_)]
        elif isinstance(slice, ast.Slice):
            return [get_value_access(slice.lower, self.symbols_), get_value_access(slice.upper, self.symbols_)]


def get_uses(node, symbols):
    finder = UseFinder(symbols)
    finder.visit(node)
    return _filter_none(finder.uses_)


class UseFinder(ast.NodeVisitor):
    def __init__(self, symbols):
        self.uses_ = []
        self.symbols_ = symbols

    def visit_Name(self, node):
        self.uses_.append(get_value_access(node, self.symbols_))

    def visit_Call(self, node):
        self.uses_.append(get_value_access(node, self.symbols_))
        # Do not iterate over the arguments, the use of variables in arguments
        # are "captured" by this call.

    def visit_Attribute(self, node):
        self.uses_.append(get_value_access(node, self.symbols_))

    def visit_Subscript(self, node):
        self.uses_.append(get_value_access(node, self.symbols_))


def _filter_none(values):
    return list(filter(lambda value: value is not None, values))


class Link:
    def __init__(self, node):
        self.node_ = node
        self.prev_ = []
        self.next_ = []

    def unlink(self):
        self.prev_ = []
        self.next_ = []

    def add_next(self, links):
        if not isinstance(links, list):
            links = [links]

        for link in links:
            self.next_.append(link)
            link.prev_.append(self)

    def add_prev(self, links):
        if not isinstance(links, list):
            links = [links]

        for link in links:
            self.prev_.append(link)
            link.next_.append(self)

    def prepend(self, link):
        prev = self.prev_
        self.prev_ = []

        link.add_next(self)

        for p in prev:
            p.next_ = []
            link.add_prev(p)

    def replace(self, what, by):
        for i, link in enumerate(self.prev_):
            if link == what:
                self.prev_[i] = by

        for i, link in enumerate(self.next_):
            if link == what:
                self.next_[i] = by

    def node(self):
        return self.node_

    def next(self):
        return [link.node_ for link in self.next_]

    def prev(self):
        return [link.node_ for link in self.prev_]
