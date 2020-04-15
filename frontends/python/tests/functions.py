# AARD: function: foo
# AARD: #1:1 ->   ::  defs: %1 / uses:  [@1 3:9-3:12]  { arg }
def foo(bar):
    # AARD: #1:2 -> #1:3  ::  defs:  / uses: %1 [@1 5:5-5:19]  { ret }
    return 2 * bar

# AARD: function: __main__
# AARD: #1:4 -> #1:5  ::  defs: %2 / uses:  [@1 9:1-9:8]
bar = 3
# AARD: #1:5 -> #1:6  ::  defs: %3 / uses: %2 [@1 12:7-12:15]  { call }
# AARD: #1:6 -> #1:7  ::  defs: %2 / uses: %3 [@1 12:1-12:15]
bar = foo(bar)
# AARD: #1:7 -> #1:8  ::  defs: %4 / uses: %2 [@1 14:1-14:9]  { call }
foo(bar)

# AARD: #1:8 -> #1:9  ::  defs: %5 / uses: %2 [@1 18:5-18:13]  { call }
# AARD: #1:9 ->   ::  defs: %6 / uses: %5 [@1 18:1-18:14]  { call }
foo(foo(bar))

# TODO: Nested functions
# # AARD: function: baz::nested
# # AARD: #1:4 -> #1:5  ::  defs:  / uses:  [@1 13:9-13:17]  { ret }

# # AARD: function: baz
# def baz():
#     def nested():
#         return 0

#     # AARD: #1:6 ->   ::  defs: %5 / uses:  [@1 16:12-16:20]  { call }
#     # AARD: #1:7 -> #1:8  ::  defs:  / uses: %5 [@1 16:5-16:20]  { ret }
#     return nested()

# AARD: @1 = functions.py
