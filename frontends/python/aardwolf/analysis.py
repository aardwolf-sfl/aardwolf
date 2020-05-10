import ast
import os

from .cfg import CFGBuilder
from .values import ValueAccessBuilder


class Analysis(ast.NodeVisitor, CFGBuilder, ValueAccessBuilder):
    def __init__(self, symbols, filename):
        CFGBuilder.__init__(self)
        ValueAccessBuilder.__init__(self, symbols)

        self.nodes_ = IdMap()
        self.values_ = IdMap()

        self.filename_ = filename
        try:
            self.file_id_ = os.stat(filename).st_ino
        except:
            self.file_id_ = 0

    def visit_ClassDef(self, node):
        self.push_ctx(node.name)
        symbols = self.use_symbols_of(node.name)

        self._visit_body(node.body)

        self.pop_ctx()
        self.use_symbols(symbols)

    def visit_FunctionDef(self, node):
        self.push_ctx(node.name)
        symbols = self.use_symbols_of(node.name)

        self.new_level()
        for arg in node.args.args:
            self.register_name(arg)
            self.add_def(arg, self.access())
            self.add_node(arg)

        self.collect_level()

        self._visit_body(node.body)

        self.pop_ctx()
        self.use_symbols(symbols)

    def visit_Assign(self, node):
        self.new_level()
        self.visit(node.value)
        self.add_uses(node, self.collect_level())

        for target in node.targets:
            self.new_level()
            self.visit(target)
            self.add_defs(node, self.collect_level())

        self.add_node(node)

    def visit_AugAssign(self, node):
        self.new_level()
        self.visit(node.value)
        self.add_uses(node, self.collect_level())

        self.new_level()
        self.visit(node.target)
        defs = self.collect_level()
        self.add_defs(node, defs)
        self.add_uses(node, defs)

        self.add_node(node)

    def visit_Assert(self, node):
        self.new_level()
        self.visit(node.test)
        self.add_uses(node, self.collect_level())

        self.add_node(node)

    def visit_Delete(self, node):
        for target in node.targets:
            self.new_level()
            self.visit(target)
            self.add_uses(node, self.collect_level())

        self.add_node(node)

    def visit_Call(self, node):
        self.new_level()

        for arg in node.args:
            self.visit(arg)

        for keyword in node.keywords:
            self.visit(keyword.value)

        self.add_uses(node, self.collect_level())

        self.visit(node.func)
        self.register_call(node)

        self.add_def(node, self.access())

        self.add_node(node)

    def visit_If(self, node):
        self.new_level()
        self.visit(node.test)
        self.add_uses(node, self.collect_level())

        self.add_node(node)

        prev_block = self.block_

        then_block = self.new_block()
        prev_block.add_succ(then_block)

        self._visit_body(node.body)

        if len(node.orelse) > 0:
            else_block = self.new_block()
            prev_block.add_succ(else_block)

            self._visit_body(node.orelse)
        else:
            else_block = None

        new_block = self.new_block()

        then_block.add_succ(new_block)
        if else_block is not None:
            else_block.add_succ(new_block)
        else:
            prev_block.add_succ(new_block)

    def visit_For(self, node):
        self.new_level()
        self.visit(node.iter)
        self.add_uses(node, self.collect_level())

        prev_block = self.block_

        target_block = self.new_block()
        self.push_loop()
        self.add_node(node)

        prev_block.add_succ(target_block)

        self.new_level()
        self.visit(node.target)
        self.add_defs(node, self.collect_level())

        body_block = self.new_block()
        target_block.add_succ(body_block)

        self._visit_body(node.body)
        self.block_.add_succ(target_block)

        if len(node.orelse) > 0:
            else_block = self.new_block()
            target_block.add_succ(else_block)

            self._visit_body(node.orelse)
        else:
            else_block = None

        new_block = self.new_block()

        target_block.add_succ(new_block)
        if else_block is not None:
            else_block.add_succ(new_block)

        self.pop_loop()

    def visit_While(self, node):
        self.new_level()
        self.visit(node.test)
        self.add_uses(node, self.collect_level())

        prev_block = self.block_

        while_block = self.new_block()
        prev_block.add_succ(while_block)

        self.push_loop()
        self.add_node(node)

        body_block = self.new_block()
        while_block.add_succ(body_block)

        self._visit_body(node.body)
        self.block_.add_succ(while_block)

        if len(node.orelse) > 0:
            else_block = self.new_block()
            while_block.add_succ(else_block)

            self._visit_body(node.orelse)
        else:
            else_block = None

        new_block = self.new_block()

        while_block.add_succ(new_block)
        if else_block is not None:
            else_block.add_succ(new_block)

        self.pop_loop()

    def visit_Break(self, node):
        self.add_node(node)
        self.break_loop()

    def visit_Continue(self, node):
        self.add_node(node)
        self.block_.add_succ(self.peek_loop()[0])
        self.block_.freeze_succ()

    # TODO: Try, Raise, etc.

    def visit_With(self, node):
        for item in node.items:
            self.new_level()
            self.visit(item.context_expr)
            self.add_uses(item, self.collect_level())

            if item.optional_vars is not None:
                self.new_level()
                self.visit(item.optional_vars)
                self.add_defs(item, self.collect_level())

            self.add_node(item)

        self._visit_body(node.body)

    # TODO: Lambda

    def visit_Return(self, node):
        self.new_level()
        self.visit(node.value)
        self.add_uses(node, self.collect_level())
        self.add_node(node)
        self.block_.freeze_succ()

    def visit_Yield(self, node):
        self.new_level()
        self.visit(node.value)
        self.add_uses(node, self.collect_level())
        self.add_node(node)

    def visit_YieldFrom(self, node):
        self.new_level()
        self.visit(node.value)
        self.add_uses(node, self.collect_level())
        self.add_node(node)

    def visit_Name(self, node):
        self.register_name(node)

    def visit_Attribute(self, node):
        self.visit(node.value)
        self.register_attribute(node)

    def visit_Subscript(self, node):
        self.visit(node.value)

        if isinstance(node.slice, ast.Index):
            self.new_level()
            self.visit(node.slice)
            index = self.collect_level()
            self.register_subscript(index)

    def _visit_body(self, body):
        for node in body:
            self.visit(node)


class IdMap:
    def __init__(self):
        self.data_ = dict()

    def get(self, value):
        if value not in self.data_:
            self.data_[value] = len(self.data_) + 1

        return self.data_[value]

    def get_checked(self, value):
        orig_len = len(self.data_)
        index = self.get(value)
        return index, len(self.data_) != orig_len
