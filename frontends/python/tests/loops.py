# AARD: function: __main__

# AARD: #1:1 -> #1:2  ::  defs: %1 / uses:  [@1 4:1-4:18]
condition = False

# AARD: #1:2 -> #1:3  ::  defs: %2 / uses:  [@1 7:1-7:6]
n = 3

# AARD: #1:3 -> #1:4, #1:5  ::  defs:  / uses: %1 [@1 10:7-10:16]
while condition:
    # AARD: #1:4 -> #1:3  ::  defs: %2 / uses: %2 [@1 12:5-12:11]
    n += 1

# AARD: #1:5 -> #1:6  ::  defs: %3 / uses: %2 [@1 16:10-16:18]  { call }
# AARD: #1:6 -> #1:7, #1:8  ::  defs: %4 / uses: %3 [@1 16:5-16:6]
for i in range(n):
    # AARD: #1:7 -> #1:6  ::  defs: %5 / uses: %4 [@1 18:5-18:13]  { call }
    print(i)

# AARD: #1:8 ->   ::  defs: %6 / uses: %2 [@1 21:1-21:9]  { call }
print(n)

# AARD: @1 = loops.py
