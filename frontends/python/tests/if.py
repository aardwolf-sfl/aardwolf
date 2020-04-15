# AARD: function: __main__

# AARD: #1:1 -> #1:2  ::  defs: %1 / uses:  [@1 4:1-4:17]
condition = True

# AARD: #1:2 -> #1:3, #1:4  ::  defs:  / uses: %1 [@1 7:4-7:13]
if condition:
    # AARD: #1:3 -> #1:4  ::  defs: %2 / uses:  [@1 9:5-9:12]
    foo = 3

# AARD: #1:4 -> #1:5, #1:6  ::  defs:  / uses: %1 [@1 12:4-12:13]
if condition:
    # AARD: #1:5 -> #1:7  ::  defs: %3 / uses:  [@1 14:5-14:12]
    bar = 1
else:
    # AARD: #1:6 -> #1:7  ::  defs: %2 / uses:  [@1 17:5-17:12]
    foo = 2

# AARD: #1:7 -> #1:8, #1:9  ::  defs:  / uses: %1 [@1 20:4-20:13]
if condition:
    # AARD: #1:8 ->   ::  defs: %3 / uses:  [@1 22:5-22:12]
    bar = 0
# AARD: #1:9 -> #1:10, #1:11  ::  defs:  / uses: %2 [@1 24:6-24:14]
elif foo == 2:
    # AARD: #1:10 ->   ::  defs: %3 / uses:  [@1 26:5-26:12]
    bar = 1
else:
    # AARD: #1:11 ->   ::  defs: %3 / uses:  [@1 29:5-29:12]
    bar = 2

# AARD: @1 = if.py
