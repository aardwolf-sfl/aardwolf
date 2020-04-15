# AARD: function: __main__

# AARD: #1:1 -> #1:2  ::  defs: %1 / uses:  [@1 4:1-4:6]
n = 3

# AARD: #1:2 -> #1:3  ::  defs: %2 / uses: %1 [@1 8:6-8:14]  { call }
# AARD: #1:3 -> #1:4  ::  defs: %3 / uses: %2 [@1 8:1-8:14]
it = range(n)

# AARD: #1:4 -> #1:5  ::  defs: %4 / uses: %3 [@1 12:10-12:12]  { call }
# AARD: #1:5 -> #1:6, #1:7  ::  defs: %5 / uses: %4 [@1 12:5-12:6]
for i in it:
    # AARD: #1:6 -> #1:4  ::  defs: %6 / uses: %5 [@1 14:5-14:13]  { call }
    print(i)

# AARD: #1:7 ->   ::  defs: %7 / uses: %1 [@1 17:1-17:9]  { call }
print(n)

# AARD: @1 = loops2.py
