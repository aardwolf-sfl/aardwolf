# AARD: function: __main__
import sys

# AARD: #1:1 -> #1:2  ::  defs: %1 / uses: %2.%3[] [@1 5:1-5:18]
foo = sys.argv[0]

# AARD: #1:2 -> #1:3  ::  defs: %4 / uses: %1 [@1 9:16-9:24]  { call }
# AARD: #1:3 -> #1:4  ::  defs: %5 / uses: %2.%3[%4] [@1 9:1-9:25]
bar = sys.argv[int(foo)]

# AARD: #1:4 -> #1:5  ::  defs: %6 / uses:  [@1 13:7-13:13]  { call }
# AARD: #1:5 -> #1:6  ::  defs: %7 / uses: %6 [@1 13:1-13:13]
baz = dict()

# AARD: #1:6 ->   ::  defs: %7[] / uses: %5 [@1 16:1-16:17]
baz['quo'] = bar

# AARD: @1 = access.py
