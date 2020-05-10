# AARD: function: foo

def foo():
    # AARD: #1:1 -> #1:2  ::  defs: %1 / uses:  [@1 5:5-5:11]
    n = 42

    # AARD: #1:2 ->   ::  defs:  / uses:  [@1 6:5-6:5]  { ret }
    # return None

# AARD: @1 = normalization.py
