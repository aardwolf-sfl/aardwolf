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
            write_id = self._make_write_stmt(arg)
            node.body.insert(2 * index, write_id)

            trace_value = self._make_trace_arg(arg)
            node.body.insert(2 * index + 1, trace_value)

        return node

    def visit_Assign(self, node):
        self.generic_visit(node)

        node.value = self._instrument_expr(node.value, node)

        for target in node.targets:
            builder = TargetAccessorBuilder()
            builder.visit(target)
            accessors = builder.build()

            # Wrap the value with the tracing call as many times as the number
            # of targets to match the number of definitions.
            node.value = self._instrument_trace_value(node.value, accessors)

        return node

    def visit_AugAssign(self, node):
        self.generic_visit(node)

        builder = TargetAccessorBuilder()
        builder.visit(node.target)
        accessors = builder.build()

        node.value = self._instrument_expr(node.value, node)
        node.value = self._instrument_trace_value(node.value, accessors)

        return node

    def visit_Assert(self, node):
        self.generic_visit(node)
        return self._instrument_term_expr(node, 'test')

    def visit_Delete(self, node):
        self.generic_visit(node)
        return self._instrument_stmt(node)

    def visit_Call(self, node):
        self.generic_visit(node)

        # foo(bar) -> write_expr(foo)(bar)
        node_id = self._make_node_id(node)
        node.func = self._make_runtime_call('write_expr', [node.func, node_id])
        ast.fix_missing_locations(node)

        return self._instrument_trace_value(node)

    def visit_If(self, node):
        self.generic_visit(node)
        node.test = self._instrument_expr(node.test, node)
        return node

    def visit_For(self, node):
        self.generic_visit(node)

        stmt_id = self._make_node_id(node)

        builder = TargetAccessorBuilder()
        builder.visit(node.target)
        accessors = builder.build()

        call = self._make_runtime_call(
            'aardwolf_iter', [node.iter, stmt_id, accessors])
        ast.copy_location(call, node.iter)
        ast.fix_missing_locations(call)

        node.iter = call

        return node

    def visit_While(self, node):
        self.generic_visit(node)
        node.test = self._instrument_expr(node.test, node)
        return node

    def visit_Break(self, node):
        self.generic_visit(node)
        return self._instrument_term(node)

    def visit_Continue(self, node):
        self.generic_visit(node)
        return self._instrument_term(node)

    def visit_With(self, node):
        self.generic_visit(node)

        for item in node.items:
            item.context_expr = self._instrument_trace_value(
                self._instrument_expr(item.context_expr, item))

        return node

    # TODO: visit_Lambda (do not forget to trace values of arguments)

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

    def _make_write_stmt(self, node, id_node=None):
        if id_node is None:
            id_node = node

        node_id = self._make_node_id(id_node)
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

    def _instrument_expr(self, node, id_node=None):
        if id_node is None:
            id_node = node

        node_id = self._make_node_id(id_node)
        call = self._make_runtime_call('write_expr', [node, node_id])

        ast.copy_location(call, node)
        ast.fix_missing_locations(call)

        return call

    def _instrument_term_expr(self, node, expr_field):
        if getattr(node, expr_field) is None:
            return node

        node_id = self._make_node_id(node)
        call = self._make_runtime_call(
            'write_expr', [getattr(node, expr_field), node_id])

        ast.copy_location(call, node)
        ast.fix_missing_locations(call)

        setattr(node, expr_field, call)

        return node

    def _instrument_trace_value(self, node, accessors=None):
        if accessors is None:
            accessors = ast.Constant(value=None, kind=None)

        call = self._make_runtime_call('write_value', [node, accessors])

        ast.copy_location(call, node)
        ast.fix_missing_locations(call)

        return call

    def _make_trace_arg(self, arg):
        node = ast.Name(id=arg.arg, ctx=ast.Load())
        call = self._make_runtime_call('write_value', [node])
        stmt = ast.Expr(value=call)

        ast.copy_location(stmt, arg)
        ast.fix_missing_locations(stmt)

        return stmt


# For unpacking assignment.
class TargetAccessorBuilder(ast.NodeVisitor):
    def __init__(self):
        self.tree_ = []

    def build(self):
        tree = self._build_node(self.tree_)
        ast.fix_missing_locations(tree)
        return tree

    def visit_Tuple(self, node):
        assert isinstance(node.ctx, ast.Store)

        tree = self.tree_
        for elt in node.elts:
            self.tree_ = []
            self.visit(elt)
            tree.append(self.tree_)
        self.tree_ = tree

    def visit_List(self, node):
        assert isinstance(node.ctx, ast.Store)

        tree = self.tree_
        for elt in node.elts:
            self.tree_ = []
            self.visit(elt)
            tree.append(self.tree_)
        self.tree_ = tree

    def visit_Starred(self, node):
        assert isinstance(node.ctx, ast.Store)
        self.visit(node.value)
        # Our indication that it is starred, quite hacky tbh.
        self.tree_ = tuple(self.tree_)

    def visit_Name(self, node):
        assert isinstance(node.ctx, ast.Store)
        # Stop the visitor here

    def visit_Attribute(self, node):
        assert isinstance(node.ctx, ast.Store)
        # Stop the visitor here

    def visit_Subscript(self, node):
        assert isinstance(node.ctx, ast.Store)
        # Stop the visitor here

    def _build_node(self, value):
        assert isinstance(value, (list, tuple))
        elts = [self._build_node(elem) for elem in value]

        if isinstance(value, list):
            return ast.List(elts=elts, ctx=ast.Load())
        else:
            return ast.Tuple(elts=elts, ctx=ast.Load())
