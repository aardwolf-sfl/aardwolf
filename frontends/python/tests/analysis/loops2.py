# AARD: function: __main__

# AARD: #1:1 -> #1:2  ::  defs: %1 / uses:  [@1 4:1-4:6]
n = 3

# AARD: #1:2 -> #1:3  ::  defs: %2 / uses: %1 [@1 8:6-8:14]  { call }
# AARD: #1:3 -> #1:4  ::  defs: %3 / uses: %2 [@1 8:1-8:14]
it = range(n)

# AARD: #1:4 -> #1:5, #1:6  ::  defs: %4 / uses: %3 [@1 11:5-11:6]
for i in it:
    # AARD: #1:5 -> #1:4  ::  defs: %5 / uses: %4 [@1 13:5-13:13]  { call }
    print(i)

# AARD: #1:6 ->   ::  defs: %6 / uses: %1 [@1 16:1-16:9]  { call }
print(n)

# AARD: @1 = loops2.py
