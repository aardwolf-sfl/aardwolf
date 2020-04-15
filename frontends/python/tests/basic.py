# AARD: function: __main__

# AARD: #1:1 -> #1:2  ::  defs: %1 / uses:  [@1 4:1-4:8]
foo = 3

# AARD: #1:2 -> #1:3  ::  defs: %2 / uses: %1 [@1 7:1-7:14]
bar = foo * 2

# AARD: #1:3 ->   ::  defs: %1 / uses: %1, %2 [@1 10:1-10:11]
foo += bar

# AARD: @1 = basic.py
