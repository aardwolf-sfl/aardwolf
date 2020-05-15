# AARD: function: __main__

# AARD: #1:1 -> #1:2  ::  defs: %1 / uses: %2 [@1 4:5-4:8]
for foo in bar:
    # AARD: #1:2 -> #1:1  ::  defs: %1 / uses: %1 [@1 6:5-6:14]
    foo = foo

# AARD: @1 = loops4.py
