# AARD: function: init
def init():
    # AARD: #1:1 ->   ::  defs:  / uses:  [@1 4:5-4:13]  { ret }
    return 3

# AARD: function: foo
# AARD: #1:2 -> #1:3  ::  defs: %1 / uses:  [@1 11:9-11:10]  { arg }
# AARD: #1:3 -> #1:4  ::  defs: %2 / uses:  [@1 11:12-11:13]  { arg }
# AARD: #1:4 -> #1:5  ::  defs: %3 / uses:  [@1 11:18-11:19]  { arg }
# AARD: #1:5 -> #1:6  ::  defs: %4 / uses:  [@1 11:20-11:26]  { call }
def foo(x, y=42, z=init()):
    # AARD: #1:6 ->   ::  defs:  / uses: %1, %2, %3 [@1 13:5-13:21]  { ret }
    return x + y + z

# AARD: @1 = default_args.py
