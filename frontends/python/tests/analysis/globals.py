# AARD: function: __main__

# AARD: #1:1 ->   ::  defs: %1 / uses:  [@1 4:1-4:8]
foo = 3

# AARD: function: get_foo
def get_foo():
    # AARD: #1:2 ->   ::  defs:  / uses: %1 [@1 9:5-9:15]  { ret }
    return foo

# AARD: @1 = globals.py
