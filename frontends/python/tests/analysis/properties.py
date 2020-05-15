class Foo:
    # AARD: function: Foo::__init__
    # AARD: #1:1 -> #1:2  ::  defs: %1 / uses:  [@1 4:18-4:22]  { arg }
    def __init__(self):
        # AARD: #1:2 -> #1:3  ::  defs: %1.%2 / uses:  [@1 6:9-6:22]
        self.bar_ = 0
        # AARD: #1:3 ->   ::  defs:  / uses:  [@1 7:9-7:9]  { ret }

    # AARD: function: Foo::bar
    # AARD: #1:4 -> #1:5  ::  defs: %3 / uses:  [@1 12:13-12:17]  { arg }
    @property
    def bar(self):
        # AARD: #1:5 ->   ::  defs:  / uses: %3.%2 [@1 14:9-14:25]  { ret }
        return self.bar_

    # AARD: function: Foo::bar@2
    # AARD: #1:6 -> #1:7  ::  defs: %4 / uses:  [@1 20:13-20:17]  { arg }
    # AARD: #1:7 -> #1:8  ::  defs: %5 / uses:  [@1 20:19-20:24]  { arg }
    @bar.setter
    def bar(self, value):
        # AARD: #1:8 -> #1:9  ::  defs: %4.%2 / uses: %5 [@1 22:9-22:26]
        self.bar_ = value
        # AARD: #1:9 ->   ::  defs:  / uses:  [@1 23:9-23:9]  { ret }

# AARD: @1 = properties.py
