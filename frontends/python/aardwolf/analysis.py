import ast
import os

from .cfg import CFGBuilder
from .values import ValueAccessBuilder
from .utils import IdMap


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
        self.enter_scope(node.name)

        self._visit_body(node.decorator_list)
        if any([isinstance(decorator, ast.Call) for decorator in node.decorator_list]):
            # Break the decorators initialization from the body of the function.
            self.new_block()

        self._visit_body(node.body)

        self.pop_ctx()
        self.exit_scope()

    def visit_FunctionDef(self, node):
        name = f'{node.name}[{node.lineno}]'

        self.push_ctx(name)
        self.enter_scope(name)

        self._visit_body(node.decorator_list)
        if any([isinstance(decorator, ast.Call) for decorator in node.decorator_list]):
            # Break the decorators initialization from the body of the function.
            self.new_block()

        self.new_level()
        for arg in node.args.args:
            self.register_name(arg)
            self.add_def(arg, self.access())
            self.add_node(arg)

        self.collect_level()

        self._visit_body(node.args.defaults)

        self._visit_body(node.body)

        self.pop_ctx()
        self.exit_scope()

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
        self.generic_visit(node)
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

        levels = self.levels()
        self.visit(node.func)

        if not self.was_registered(levels):
            assert isinstance(node.func, ast.Lambda)
            name = f'lambda:{node.func.lineno}:{node.func.col_offset}'
            self.register_name(name)

        self.register_call(node)

        self.add_def(node, self.access())

        self.add_node(node)

    def visit_If(self, node):
        self.new_level()
        self.visit(node.test)
        self.add_uses(node, self.collect_level())

        if_block = self.block()
        self.add_node(node)

        then_block = self.new_block()
        if_block.add_succ(then_block)

        self._visit_body(node.body)
        then_block = self.block()

        if len(node.orelse) > 0:
            else_block = self.new_block()
            if_block.add_succ(else_block)

            self._visit_body(node.orelse)
            else_block = self.block()
        else:
            else_block = None

        new_block = self.new_block()

        then_block.add_succ(new_block)
        if else_block is None:
            if_block.add_succ(new_block)
        else:
            else_block.add_succ(new_block)

    # TODO: IfExp

    def visit_For(self, node):
        self.new_level()
        self.visit(node.iter)
        self.add_uses(node, self.collect_level())

        prev_block = self.block()

        loop_block = self.new_block()
        prev_block.add_succ(loop_block)

        self.push_loop()

        self.add_node(node)

        self.new_level()
        self.visit(node.target)
        self.add_defs(node, self.collect_level())

        body_block = self.new_block()
        loop_block.add_succ(body_block)

        self._visit_body(node.body)
        self.block().add_succ(loop_block)

        if len(node.orelse) > 0:
            else_block = self.new_block()
            loop_block.add_succ(else_block)

            self._visit_body(node.orelse)
            else_block = self.block()
        else:
            else_block = None

        new_block = self.new_block()

        loop_block.add_succ(new_block)
        if else_block is not None:
            else_block.add_succ(new_block)

        for block in loop_block.exits():
            block.add_succ(new_block, force=True)

        self.pop_loop()

    def visit_While(self, node):
        self.new_level()
        self.visit(node.test)
        self.add_uses(node, self.collect_level())

        prev_block = self.block()

        loop_block = self.new_block()
        prev_block.add_succ(loop_block)
        self.push_loop()

        self.add_node(node)

        body_block = self.new_block()
        loop_block.add_succ(body_block)

        self._visit_body(node.body)
        self.block().add_succ(loop_block)

        if len(node.orelse) > 0:
            else_block = self.new_block()
            loop_block.add_succ(else_block)

            self._visit_body(node.orelse)
            else_block = self.block()
        else:
            else_block = None

        new_block = self.new_block()

        loop_block.add_succ(new_block)
        if else_block is not None:
            else_block.add_succ(new_block)

        for block in loop_block.exits():
            block.add_succ(new_block, force=True)

        self.pop_loop()

    def visit_Break(self, node):
        block = self.block()
        self.add_node(node)

        loop_block = self.peek_loop()
        loop_block.add_exit(block)
        block.freeze()

    def visit_Continue(self, node):
        block = self.block()
        self.add_node(node)

        loop_block = self.peek_loop()
        block.add_succ(loop_block)
        block.freeze()

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

    def visit_Lambda(self, node):
        name = f'lambda:{node.lineno}:{node.col_offset}'
        self.push_ctx(name)
        self.enter_scope(name)

        self.new_level()
        for arg in node.args.args:
            self.register_name(arg)
            self.add_def(arg, self.access())
            self.add_node(arg)

        self.collect_level()

        self._visit_body(node.args.defaults)

        body = ast.Return(value=node.body)
        ast.copy_location(body, node.body)
        ast.fix_missing_locations(body)
        self.visit(body)

        self.pop_ctx()
        self.exit_scope()

    def visit_Return(self, node):
        if node.value is not None:
            self.new_level()
            self.visit(node.value)
            self.add_uses(node, self.collect_level())

        self.add_node(node)
        self.block().freeze()

    def visit_Yield(self, node):
        if node.value is not None:
            self.new_level()
            self.visit(node.value)
            self.add_uses(node, self.collect_level())

        self.add_node(node)

    def visit_YieldFrom(self, node):
        if node.value is not None:
            self.new_level()
            self.visit(node.value)
            self.add_uses(node, self.collect_level())

        self.add_node(node)

    def visit_Name(self, node):
        self.register_name(node)

    def visit_Attribute(self, node):
        levels = self.levels()
        self.visit(node.value)

        if self.was_registered(levels):
            self.register_attribute(node)
        else:
            # Probably constant node.value.
            self.register_name(node.attr)

    def visit_Subscript(self, node):
        levels = self.levels()
        self.visit(node.value)

        # if isinstance(node.slice, ast.Index):
        self.new_level()
        self.visit(node.slice)
        index = self.collect_level()

        if not self.was_registered(levels):
            # Probably constant node.value. Use a dummy base.
            self.register_name('$constant')

        self.register_subscript(index)

    def _visit_body(self, body):
        for node in body:
            self.visit(node)
