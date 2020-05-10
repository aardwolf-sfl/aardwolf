# AARD: function: __main__

# AARD: #1:1 -> #1:2  ::  defs: %1 / uses:  [@1 5:4-5:10]  { call }
# AARD: #1:2 -> #1:3, #1:4  ::  defs:  / uses: %1 [@1 5:4-5:10]
if test():
    # AARD: #1:3 -> #1:4  ::  defs: %2 / uses:  [@1 7:5-7:12]
    foo = 3

# AARD: #1:4 ->   ::  defs: %3 / uses:  [@1 10:1-10:8]  { call }
print()

# AARD: @1 = if2.py
