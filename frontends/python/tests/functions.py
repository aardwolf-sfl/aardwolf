# AARD: function: __main__
# AARD: #1:1 -> #1:2  ::  defs: %1 / uses:  [@1 3:1-3:8]
bar = 3
# AARD: #1:2 -> #1:3  ::  defs: %2 / uses: %1 [@1 6:7-6:15]  { call }
# AARD: #1:3 -> #1:4  ::  defs: %1 / uses: %2 [@1 6:1-6:15]
bar = foo(bar)
# AARD: #1:4 -> #1:5  ::  defs: %3 / uses: %1 [@1 8:1-8:9]  { call }
foo(bar)

# AARD: #1:5 -> #1:6  ::  defs: %4 / uses: %1 [@1 12:5-12:13]  { call }
# AARD: #1:6 ->   ::  defs: %5 / uses: %4 [@1 12:1-12:14]  { call }
foo(foo(bar))

# AARD: function: foo
# AARD: #1:7 -> #1:8  ::  defs: %6 / uses:  [@1 16:9-16:12]  { arg }
def foo(bar):
    # AARD: #1:8 ->   ::  defs:  / uses: %6 [@1 18:5-18:19]  { ret }
    return 2 * bar

# AARD: function: baz
def baz():
    def nested():
        return 0

    # AARD: #1:9 -> #1:10  ::  defs: %7 / uses:  [@1 27:12-27:20]  { call }
    # AARD: #1:10 ->   ::  defs:  / uses: %7 [@1 27:5-27:20]  { ret }
    return nested()

# AARD: function: baz::nested
# AARD: #1:11 ->   ::  defs:  / uses:  [@1 23:9-23:17]  { ret }

# AARD: @1 = functions.py
