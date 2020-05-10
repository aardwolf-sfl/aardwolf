import ast

CTX_ASSIGN = 'assign'
CTX_DEFINE = 'define'


class Symbol:
    def __init__(self, name):
        self.name_ = name

    def get_name(self):
        return self.name_

    def __str__(self):
        return f'<Symbol "{self.name_}">'


class Scope(Symbol):
    def __init__(self, name, parent):
        super().__init__(name)
        self.parent_ = parent
        self.children_ = []

    def define(self, symbol, ctx):
        assert ctx in [CTX_ASSIGN, CTX_DEFINE]

        if isinstance(symbol, str):
            symbol = Symbol(symbol)

        assert isinstance(symbol, Symbol)
        name = symbol.get_name()

        if ctx == CTX_ASSIGN:
            try:
                self.lookup(name)
            except KeyError:
                self.children_.append(symbol)
        elif ctx == CTX_DEFINE:
            self.children_.append(symbol)

    def lookup(self, name):
        for symbol in self.get_namespace():
            if symbol.get_name() == name:
                return symbol

        parent = self.get_parent()
        if parent is None:
            raise KeyError(name)
        else:
            return parent.lookup(name)

    def get_parent(self):
        return self.parent_

    def get_namespace(self):
        return self.children_

    def __str__(self):
        namespace = ', '.join([str(child) for child in self.children_])
        return f'<Scope "{self.name_}": [{namespace}]>'


class SymbolTable:
    def __init__(self):
        self.scope_ = Scope('top', None)

    def define(self, name, ctx):
        self.scope_.define(name, ctx)

    def push_scope(self, name):
        self.scope_ = Scope(name, self.scope_)
        self.scope_.get_parent().define(self.scope_, CTX_DEFINE)

    def pop_scope(self):
        self.scope_ = self.scope_.get_parent()

    def __str__(self):
        return str(self.scope_)


class SymbolDefiner(ast.NodeVisitor):
    def __init__(self, table, ctx):
        self.table_ = table
        self.ctx_ = ctx

    def visit_Name(self, node):
        if isinstance(node.ctx, ast.Store):
            self.table_.define(node.id, self.ctx_)

    def visit_arg(self, node):
        self.table_.define(node.arg, self.ctx_)

    def visit_alias(self, node):
        if node.asname is not None:
            self.table_.define(node.asname, self.ctx_)
        else:
            self.table_.define(node.name, self.ctx_)


class SymbolTableBuilder(ast.NodeVisitor):
    def __init__(self):
        self.table_ = SymbolTable()
        self.assigner_ = SymbolDefiner(self.table_, CTX_ASSIGN)
        self.definer_ = SymbolDefiner(self.table_, CTX_DEFINE)

    def visit_Assign(self, node):
        for target in node.targets:
            self.assigner_.visit(target)

        self.generic_visit(node)

    def visit_AnnAssign(self, node):
        self.assigner_.visit(node.target)
        self.generic_visit(node)

    def visit_AugAssign(self, node):
        self.assigner_.visit(node.target)
        self.generic_visit(node)

    def visit_For(self, node):
        self.definer_.visit(node.target)
        self.generic_visit(node)

    def visit_With(self, node):
        for item in node.items:
            self.definer_.visit(item)

        self.generic_visit(node)

    def visit_Import(self, node):
        for name in node.names:
            self.definer_.visit(name)

        self.generic_visit(node)

    def visit_ImportFrom(self, node):
        for name in node.names:
            self.definer_.visit(name)

        self.generic_visit(node)

    def visit_FunctionDef(self, node):
        self.table_.push_scope(node.name)

        self.definer_.visit(node.args)

        self.generic_visit(node)
        self.table_.pop_scope()

    def visit_Lambda(self, node):
        self.table_.push_scope(f'lambda:{node.lineno}:{node.col_offset}')

        self.definer_.visit(node.args)

        self.generic_visit(node)
        self.table_.pop_scope()

    def visit_ClassDef(self, node):
        self.table_.push_scope(node.name)
        self.generic_visit(node)
        self.table_.pop_scope()


def symbol_table(tree):
    builder = SymbolTableBuilder()
    builder.visit(tree)
    return builder.table_.scope_
