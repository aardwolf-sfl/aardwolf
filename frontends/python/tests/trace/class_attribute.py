def init():
    return 3


class Foo:
    # AARD: statement: #1:2
    # AARD: i64: 42
    BAR = 42

    # AARD: statement: #1:3
    # AARD: unsupported data type
    BAZ = init

    # AARD: statement: #1:4
    # AARD: statement: #1:1
    # AARD: i64: 3
    # AARD: statement: #1:5
    # AARD: i64: 3
    QUO = BAZ()

    def bar(self):
        return self.BAR
