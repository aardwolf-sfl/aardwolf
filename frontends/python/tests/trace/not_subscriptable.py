items = [['foo', 'bar'], [3, 42]]
a, b = zip(*items)

# items = [['foo', 'bar'], [3, 42]]
# AARD: statement: #1:1
# AARD: unsupported data type

# zip(*items)
# AARD: statement: #1:2
# AARD: unsupported data type

# a, b = zip(*items)
# AARD: statement: #1:3
# AARD: unsupported data type
# AARD: unsupported data type
