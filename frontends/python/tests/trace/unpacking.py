foo, (bar, quo) = 1, (2, 3)

print(foo, bar, quo)

for idx, value in enumerate(['foo', 'bar', 'quo']):
    print(idx, value)

# AARD: statement: #1:1
# AARD: i64: 1
# AARD: i64: 2
# AARD: i64: 3

# AARD: statement: #1:2
# AARD: unsupported data type

# AARD: statement: #1:3
# AARD: unsupported data type

# AARD: statement: #1:4
# AARD: i64: 0
# AARD: unsupported data type

# AARD: statement: #1:5
# AARD: unsupported data type

# AARD: statement: #1:4
# AARD: i64: 1
# AARD: unsupported data type

# AARD: statement: #1:5
# AARD: unsupported data type

# AARD: statement: #1:4
# AARD: i64: 2
# AARD: unsupported data type

# AARD: statement: #1:5
# AARD: unsupported data type
