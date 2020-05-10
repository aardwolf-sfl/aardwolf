# AARD: function: __main__

# AARD: #1:1 -> #1:2  ::  defs: %1 / uses:  [@1 4:1-4:8]
foo = 3

# AARD: #1:2 ->   ::  defs: %2 / uses: %1 [@1 7:1-7:16]
bar = foo * foo

# AARD: @1 = def_use.py
