import symtable

SCALAR = 'scalar'
STRUCTURAL = 'structural'
ARRAY_LIKE = 'array_like'


class Access:
    def __init__(self, type, value=None, base=None, accessors=None):
        self.type_ = type
        self.value_ = value
        self.base_ = base
        self.accessors_ = accessors

    @staticmethod
    def scalar(value):
        assert isinstance(value, (str, symtable.Symbol))
        return Access(SCALAR, value=value)

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
        # For structural and array-like accesses, it generally calls `__str__`
        # recursively.
        if self.type_ == SCALAR:
            if isinstance(self.value_, symtable.Symbol):
                return self.value_.get_name()
            else:
                return self.value_
        elif self.type_ == STRUCTURAL:
            return f'{self.base_}.{self.accessors_}'
        elif self.type_ == ARRAY_LIKE:
            index = ', '.join([str(access) for access in self.accessors_])
            return f'{self.base_}[{index}]'

    def __eq__(self, other):
        return self.type_ == other.type_ and self.value_ == other.value_ and self.base_ == other.base_ and self.accessors_ == other.accessors_

    def __hash__(self):
        if self.is_scalar():
            return hash(self.value_)
        elif self.is_structural():
            return hash((self.base_, self.accessors_))
        elif self.is_array_like():
            h = hash(self.base_)
            for i, access in enumerate(self.accessors_):
                h = h ^ (hash(access) << (i + 1))
            return h
