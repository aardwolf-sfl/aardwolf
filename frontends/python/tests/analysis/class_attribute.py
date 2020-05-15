# AARD: function: init
def init():
    # AARD: #1:1 ->   ::  defs:  / uses:  [@1 4:5-4:13]  { ret }
    return 3

# AARD: function: Foo
class Foo:
    # AARD: #1:2 -> #1:3  ::  defs: %1 / uses:  [@1 9:5-9:13]
    BAR = 42
    # AARD: #1:3 -> #1:4  ::  defs: %2 / uses: %3 [@1 11:5-11:15]
    BAZ = init
    # AARD: #1:4 -> #1:5  ::  defs: %4 / uses:  [@1 14:11-14:16]  { call }
    # AARD: #1:5 ->   ::  defs: %5 / uses: %4 [@1 14:5-14:16]
    QUO = BAZ()

    # AARD: function: Foo::bar
    # AARD: #1:6 -> #1:7  ::  defs: %6 / uses:  [@1 18:13-18:17]  { arg }
    def bar(self):
        # AARD: #1:7 ->   ::  defs:  / uses: %6.%7 [@1 20:9-20:24]  { ret }
        return self.BAR

# AARD: @1 = class_attribute.py
