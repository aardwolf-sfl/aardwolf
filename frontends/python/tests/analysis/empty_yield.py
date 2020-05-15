# AARD: function: foo
def foo():
    # AARD: #1:1 -> #1:2  ::  defs: %1 / uses:  [@1 5:15-5:20]
    # AARD: #1:2 -> #1:3  ::  defs: %2 / uses: %1 [@1 5:5-5:20]
    outcome = yield
    # AARD: #1:3 ->   ::  defs:  / uses:  [@1 6:5-6:5]  { ret }

# AARD: @1 = empty_yield.py
# AARD: SKIP
