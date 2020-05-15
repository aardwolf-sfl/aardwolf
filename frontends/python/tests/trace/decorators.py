def foo(f):
    return f

def bar(x):
    def _wrap(f):
        return f

    return _wrap

@foo
@bar(42)
def baz():
    return 3

# @bar(42)
# AARD: statement: #1:7

# def bar(x)
# AARD: statement: #1:3
# AARD: i64: 42

# return _wrap
# AARD: statement: #1:4

# result of @bar(42)
# AARD: unsupported data type

# def _wrap(f)
# AARD: statement: #1:5
# AARD: unsupported data type

# return f
# AARD: statement: #1:6

# def foo(f)
# AARD: statement: #1:1
# AARD: unsupported data type

# return f
# AARD: statement: #1:2
