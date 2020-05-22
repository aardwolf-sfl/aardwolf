# AARD: function: __main__

# AARD: #1:1 -> #1:2  ::  defs: %1 / uses:  [@1 4:1-4:17]
condition = True

# AARD: #1:2 -> #1:3  ::  defs: %2 / uses:  [@1 7:1-7:10]
true = 42

# AARD: #1:3 -> #1:4  ::  defs: %3 / uses:  [@1 10:1-10:10]
false = 0

# AARD: #1:4 ->   ::  defs: %4 / uses: %1, %2, %3 [@1 13:1-13:38]
result = true if condition else false

# AARD: @1 = ifexp.py
