class Foo:
    # AARD: function: Foo::bar
    # AARD: #1:1 -> #1:2  ::  defs: %1 / uses:  [@1 4:13-4:17]  { arg }
    def bar(self):
        # AARD: #1:2 ->   ::  defs:  / uses: %1 [@1 6:9-6:20]  { ret }
        return self

# AARD: @1 = classes.py
