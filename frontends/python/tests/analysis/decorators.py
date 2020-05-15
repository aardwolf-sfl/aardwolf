# AARD: function: foo
# AARD: #1:1 -> #1:2  ::  defs: %1 / uses:  [@1 3:9-3:10]  { arg }
def foo(f):
    # AARD: #1:2 ->   ::  defs:  / uses: %1 [@1 5:5-5:13]  { ret }
    return f

# AARD: function: bar
# AARD: #1:3 -> #1:4  ::  defs: %2 / uses:  [@1 9:9-9:10]  { arg }
def bar(x):
    def _wrap(f):
        return f

    # AARD: #1:4 ->   ::  defs:  / uses: %3 [@1 14:5-14:17]  { ret }
    return _wrap

# AARD: function: bar::_wrap
# AARD: #1:5 -> #1:6  ::  defs: %4 / uses:  [@1 10:15-10:16]  { arg }
# AARD: #1:6 ->   ::  defs:  / uses: %4 [@1 11:9-11:17]  { ret }

# AARD: function: baz
# AARD: #1:7 ->   ::  defs: %5 / uses:  [@1 23:2-23:9]  { call }
@foo
@bar(42)
def baz():
    # AARD: #1:8 ->   ::  defs:  / uses:  [@1 26:5-26:13]  { ret }
    return 3

# AARD: @1 = decorators.py
