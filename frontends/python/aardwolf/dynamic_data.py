import ast


class Instrumenter(ast.NodeTransformer):
    def __init__(self, analysis):
        self.analysis_ = analysis

    def visit_Module(self, node):
        self.generic_visit(node)

        import_aardwolf = ast.Import(
            names=[ast.alias(name='aardwolf', asname=None)])
        ast.fix_missing_locations(import_aardwolf)

        node.body.insert(0, import_aardwolf)

        return node

    def visit_FunctionDef(self, node):
        self.generic_visit(node)

        for index, arg in enumerate(node.args.args):
            write = self._make_write_stmt(arg)
            node.body.insert(index, write)

        return node

    def visit_Assign(self, node):
        self.generic_visit(node)
        return self._instrument_stmt(node)

    def visit_AugAssign(self, node):
        self.generic_visit(node)
        return self._instrument_stmt(node)

    def visit_Assert(self, node):
        self.generic_visit(node)
        return self._instrument_term_expr(node, 'test')

    def visit_Delete(self, node):
        self.generic_visit(node)
        return self._instrument_stmt(node)

    def visit_Call(self, node):
        self.generic_visit(node)
        return self._instrument_expr_lazy(node)

    def visit_If(self, node):
        self.generic_visit(node)
        node.test = self._instrument_expr(node.test)
        return node

    def visit_For(self, node):
        self.generic_visit(node)

        write = self._make_write_stmt(node.target)
        node.body.insert(0, write)

        return node

    def visit_While(self, node):
        self.generic_visit(node)
        node.test = self._instrument_expr(node.test)
        return node

    def visit_Break(self, node):
        self.generic_visit(node)
        return self._instrument_term(node)

    def visit_Continue(self, node):
        self.generic_visit(node)
        return self._instrument_term(node)

    def visit_With(self, node):
        self.generic_visit(node)
        node.items = [self._instrument_expr(
            item.context_expr) for item in node.items]
        return node

    def visit_Return(self, node):
        self.generic_visit(node)
        return self._instrument_term_expr(node, 'value')

    def visit_Yield(self, node):
        self.generic_visit(node)
        return self._instrument_term_expr(node, 'value')

    def visit_YieldFrom(self, node):
        self.generic_visit(node)
        return self._instrument_term_expr(node, 'value')

    def _make_node_id(self, node):
        file_id = ast.Constant(value=self.analysis_.file_id_, kind=None)
        stmt_id, changed = self.analysis_.nodes_.get_checked(node)
        assert not changed, 'instrumentation must not create new statement indexes'
        stmt_id = ast.Constant(value=stmt_id, kind=None)

        return ast.Tuple(elts=[file_id, stmt_id], ctx=ast.Load())

    def _make_runtime_call(self, name, args):
        func = ast.Attribute(value=ast.Name(
            id='aardwolf', ctx=ast.Load()), attr=name, ctx=ast.Load())
        call = ast.Call(func=func, args=args, keywords=[])
        return call

    def _make_write_stmt(self, node):
        node_id = self._make_node_id(node)
        call = self._make_runtime_call('write_stmt', [node_id])
        stmt = ast.Expr(value=call)

        ast.copy_location(stmt, node)
        ast.fix_missing_locations(stmt)

        return stmt

    def _instrument_stmt(self, node):
        stmt = self._make_write_stmt(node)
        return [node, stmt]

    def _instrument_term(self, node):
        [node, stmt] = self._instrument_stmt(node)
        return [stmt, node]

    def _instrument_expr(self, node):
        node_id = self._make_node_id(node)
        call = self._make_runtime_call('write_expr', [node, node_id])

        ast.copy_location(call, node)
        ast.fix_missing_locations(call)

        return call

    def _instrument_term_expr(self, node, expr_field):
        node_id = self._make_node_id(node)
        call = self._make_runtime_call(
            'write_expr', [getattr(node, expr_field), node_id])

        ast.copy_location(call, node)
        ast.fix_missing_locations(call)

        setattr(node, expr_field, call)

        return node

    def _instrument_expr_lazy(self, node):
        node_id = self._make_node_id(node)

        lazy = ast.Lambda(args=ast.arguments(
            posonlyargs=[],
            args=[],
            vararg=None,
            kwonlyargs=[],
            kw_defaults=[],
            kwarg=None,
            defaults=[]), body=node)
        call = self._make_runtime_call('write_lazy', [lazy, node_id])

        ast.copy_location(call, node)
        ast.fix_missing_locations(call)

        return call
