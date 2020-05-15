class Foo:
    def __init__(self):
        self.bar_ = 0

    @property
    def bar(self):
        return self.bar_

    @bar.setter
    def bar(self, value):
        self.bar_ = value

foo = Foo()
foo.bar = foo.bar

# Foo()
# AARD: statement: #1:1

# Foo::__init__(self)
# AARD: statement: #1:4
# AARD: unsupported data type

# self.bar_ = 0
# AARD: statement: #1:5
# AARD: i64: 0

# (implicit) return
# AARD: statement: #1:6

# result of Foo()
# AARD: unsupported data type

# foo = Foo()
# AARD: statement: #1:2
# AARD: unsupported data type

# @property
# def bar(self):
# AARD: statement: #1:7
# AARD: unsupported data type

# return self.bar_
# AARD: statement: #1:8

# foo.bar = foo.bar
# AARD: statement: #1:3
# AARD: i64: 0

# @bar.setter
# def bar(self, value):
# AARD: statement: #1:9
# AARD: unsupported data type
# AARD: statement: #1:10
# AARD: i64: 0

# self.bar_ = value
# AARD: statement: #1:11
# AARD: i64: 0

# (implicit) return
# AARD: statement: #1:12
