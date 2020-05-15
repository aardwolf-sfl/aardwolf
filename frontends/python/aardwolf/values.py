import ast

from .symbols import Symbol

SCALAR = 'scalar'
STRUCTURAL = 'structural'
ARRAY_LIKE = 'array_like'


class Access:
    def __init__(self, type, value=None, base=None, accessors=None, meta=None):
        self.type_ = type
        self.value_ = value
        self.base_ = base
        self.accessors_ = accessors
        self.meta_ = meta

    @staticmethod
    def scalar(value):
        assert isinstance(value, (Symbol, str))
        return Access(SCALAR, value=value)

    @staticmethod
    def call(value, line, col):
        assert isinstance(value, Access)
        return Access(SCALAR, value=value, meta=(line, col))

    @staticmethod
    def structural(base, field):
        assert isinstance(base, Access)
        assert isinstance(field, Access) and field.is_scalar()
        return Access(STRUCTURAL, base=base, accessors=field)

    @staticmethod
    def array_like(base, index):
        if not isinstance(index, list):
            index = [index]

        assert isinstance(base, Access)
        for i in index:
            assert isinstance(i, Access)

        return Access(ARRAY_LIKE, base=base, accessors=index)

    def is_scalar(self):
        return self.type_ == SCALAR

    def is_structural(self):
        return self.type_ == STRUCTURAL

    def is_array_like(self):
        return self.type_ == ARRAY_LIKE

    def __str__(self):
        if self.type_ == SCALAR:
            if isinstance(self.value_, Symbol):
                name = self.value_.get_name()
            else:
                name = str(self.value_)

            if self.meta_ is not None:
                line, col = self.meta_
                name += f':{line}:{col}'

            return name
        elif self.type_ == STRUCTURAL:
            return f'{self.base_}.{self.accessors_}'
        elif self.type_ == ARRAY_LIKE:
            index = ', '.join([str(access) for access in self.accessors_])
            return f'{self.base_}[{index}]'

    def __eq__(self, other):
        return self.type_ == other.type_ and self.value_ == other.value_ and self.base_ == other.base_ and self.accessors_ == other.accessors_ and self.meta_ == other.meta_

    def __hash__(self):
        if self.is_scalar():
            return hash(self.value_) ^ (hash(self.meta_) << 1)
        elif self.is_structural():
            return hash((self.base_, self.accessors_))
        elif self.is_array_like():
            h = hash(self.base_)
            for i, access in enumerate(self.accessors_):
                h = h ^ (hash(access) << (i + 1))
            return h


class ValueAccessBuilder:
    def __init__(self, symbols):
        self.level_ = []
        self.levels_ = [self.level_]
        self.symbols_ = symbols

        self.defs_ = dict()
        self.uses_ = dict()

    def new_level(self):
        self.level_ = []
        self.levels_.append(self.level_)
        return self.level_

    def collect_level(self):
        level = self.levels_.pop()
        self.level_ = self.levels_[-1]
        return level

    def access(self):
        return self.level_[-1]

    def enter_scope(self, name):
        self.symbols_ = self.symbols_.lookup(name)

    def exit_scope(self):
        self.symbols_ = self.symbols_.get_parent()

    def add_defs(self, node, accesses):
        assert all([isinstance(access, Access) for access in accesses])
        if not node in self.defs_:
            self.defs_[node] = []

        self.defs_[node].extend(accesses)

    def add_def(self, node, access):
        self.add_defs(node, [access])

    def add_uses(self, node, accesses):
        assert all([isinstance(access, Access) for access in accesses])
        if not node in self.uses_:
            self.uses_[node] = []

        self.uses_[node].extend(accesses)

    def add_use(self, node, access):
        self.add_uses(node, [access])

    def register_name(self, node):
        if isinstance(node, ast.Name):
            name = node.id
        elif isinstance(node, ast.arg):
            name = node.arg
        elif isinstance(node, str):
            name = node
        else:
            name = None

        try:
            access = Access.scalar(self.symbols_.lookup(name))
        except KeyError:
            access = Access.scalar(name)

        self.level_.append(access)

    def register_call(self, node):
        line, col = node.lineno, node.col_offset + 1
        call = self.level_.pop()
        self.level_.append(Access.call(call, line, col))

    def register_attribute(self, node):
        base = self.level_.pop()
        field = Access.scalar(node.attr)
        self.level_.append(Access.structural(base, field))

    def register_subscript(self, index):
        base = self.level_.pop()
        self.level_.append(Access.array_like(base, index))

    def levels(self):
        return len(self.level_)

    def was_registered(self, orig_levels):
        return self.levels() > orig_levels
