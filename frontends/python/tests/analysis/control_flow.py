# AARD: function: foo

def foo():
    # AARD: #1:1 -> #1:2  ::  defs: %1 / uses:  [@1 5:5-5:10]
    n = 0

    # AARD: #1:2 -> #1:3  ::  defs: %2 / uses:  [@1 9:14-9:23]  { call }
    # AARD: #1:3 -> #1:4, #1:5  ::  defs: %3 / uses: %2 [@1 9:9-9:10]
    for i in range(10):
        # AARD: #1:4 -> #1:6, #1:7  ::  defs:  / uses: %3 [@1 11:12-11:17]
        if i < 5:
            # AARD: #1:6 -> #1:3  ::  defs:  / uses:  [@1 13:13-13:21]
            continue

        # AARD: #1:7 -> #1:8, #1:9  ::  defs:  / uses: %3 [@1 16:12-16:17]
        if i > 7:
            # AARD: #1:8 -> #1:5  ::  defs:  / uses:  [@1 18:13-18:18]
            break

        # AARD: #1:9 -> #1:3  ::  defs: %1 / uses: %3 [@1 21:9-21:14]
        n = i

    # AARD: #1:5 -> #1:10, #1:11  ::  defs:  / uses: %1 [@1 24:8-24:13]
    if n > 0:
        # AARD: #1:10 ->   ::  defs:  / uses: %1 [@1 26:9-26:17]  { ret }
        return n

    # AARD: #1:11 -> #1:12  ::  defs: %1 / uses:  [@1 29:5-29:11]
    n = 42

    # AARD: #1:12 ->   ::  defs:  / uses: %1 [@1 32:5-32:13]  { ret }
    return n

# AARD: @1 = control_flow.py
