# AARD: function: foo

# AARD: #1:1 -> #1:2  ::  defs: %1 / uses:  [@1 4:9-4:10]  { arg }
def foo(a):
    # AARD: #1:2 -> #1:3  ::  defs: %2 / uses:  [@1 6:5-6:27]
    test = lambda x: x > a
    # AARD: #1:3 -> #1:4  ::  defs: %3 / uses:  [@1 8:5-8:51]
    value = lambda n, m: (lambda x: n * x + m * x)

    # AARD: #1:4 ->   ::  defs:  / uses: %2, %3 [@1 11:5-11:23]  { ret }
    return test, value

# AARD: function: foo::lambda:6:11
# AARD: #1:5 -> #1:6  ::  defs: %4 / uses:  [@1 6:19-6:20]  { arg }
# AARD: #1:6 ->   ::  defs:  / uses: %1, %4 [@1 6:22-6:27]  { ret }

# AARD: function: foo::lambda:8:12
# AARD: #1:7 -> #1:8  ::  defs: %5 / uses:  [@1 8:20-8:21]  { arg }
# AARD: #1:8 -> #1:9  ::  defs: %6 / uses:  [@1 8:23-8:24]  { arg }
# AARD: #1:9 ->   ::  defs:  / uses:  [@1 8:27-8:50]  { ret }

# AARD: function: foo::lambda:8:12::lambda:8:26
# AARD: #1:10 -> #1:11  ::  defs: %7 / uses:  [@1 8:34-8:35]  { arg }
# AARD: #1:11 ->   ::  defs:  / uses: %5, %6, %7 [@1 8:37-8:50]  { ret }

# AARD: @1 = lambda.py
