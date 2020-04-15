# AARD: function: __main__

# AARD: #1:1 -> #1:2  ::  defs: %1 / uses:  [@1 4:1-4:6]
n = 3

# AARD: #1:2 -> #1:3  ::  defs: %2 / uses: %1 [@1 9:10-9:18]  { call }
# AARD: #1:3 -> #1:4  ::  defs: %3 / uses: %2 [@1 9:10-9:18]  { call }
# AARD: #1:4 -> #1:5, #1:6  ::  defs: %4 / uses: %3 [@1 9:5-9:6]
for i in range(n):
    # AARD: #1:5 -> #1:2  ::  defs: %5 / uses: %4 [@1 11:5-11:13]  { call }
    print(i)

# AARD: #1:6 -> #1:7  ::  defs: %6 / uses: %1 [@1 15:6-15:14]  { call }
# AARD: #1:7 -> #1:8  ::  defs: %7 / uses: %6 [@1 15:1-15:14]
it = range(n)

# AARD: #1:8 -> #1:9  ::  defs: %8 / uses: %7 [@1 19:10-19:18]  { call }
# AARD: #1:9 -> #1:10, #1:11  ::  defs: %9 / uses: %8 [@1 19:5-19:6]
for i in it:
    # AARD: #1:10 -> #1:8  ::  defs: %10 / uses: %9 [@1 21:5-21:13]  { call }
    print(i)

# AARD: #1:11 ->   ::  defs: %10 / uses: %1 [@1 24:1-24:9]  { call }
print(n)

# AARD: @1 = loops3.py

# AARD: SKIP
