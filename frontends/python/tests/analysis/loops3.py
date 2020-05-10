# AARD: function: __main__

# AARD: #1:1 -> #1:2  ::  defs: %1 / uses:  [@1 4:1-4:6]
n = 3

# AARD: #1:2 -> #1:3  ::  defs: %2 / uses: %1 [@1 8:10-8:18]  { call }
# AARD: #1:3 -> #1:4, #1:5  ::  defs: %3 / uses: %2 [@1 8:5-8:6]
for i in range(n):
    # AARD: #1:4 -> #1:3  ::  defs: %4 / uses: %3 [@1 10:5-10:13]  { call }
    print(i)

# AARD: #1:5 -> #1:6  ::  defs: %5 / uses: %1 [@1 14:6-14:14]  { call }
# AARD: #1:6 -> #1:7  ::  defs: %6 / uses: %5 [@1 14:1-14:14]
it = range(n)

# AARD: #1:7 -> #1:8, #1:9  ::  defs: %3 / uses: %6 [@1 17:5-17:6]
for i in it:
    # AARD: #1:8 -> #1:7  ::  defs: %7 / uses: %3 [@1 19:5-19:13]  { call }
    print(i)

# AARD: #1:9 ->   ::  defs: %8 / uses: %1 [@1 22:1-22:9]  { call }
print(n)

# AARD: @1 = loops3.py
